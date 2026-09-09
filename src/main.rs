mod bridge;
mod catalog;
mod indicator;
mod platform;
mod resolver;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, Semaphore},
};
pub fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
static TEMP: AtomicU64 = AtomicU64::new(0);
pub fn atomic_json(p: &Path, v: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(v)?;
    atomic_write(p, &bytes)
}
pub fn atomic_write(p: &Path, b: &[u8]) -> Result<()> {
    let temp = p.with_extension(format!(
        "{}.{}.tmp",
        std::process::id(),
        TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)?;
    use std::os::unix::fs::PermissionsExt;
    f.set_permissions(fs::Permissions::from_mode(0o600))?;
    f.write_all(b)?;
    drop(f);
    if let Err(e) = fs::rename(&temp, p) {
        let _ = fs::remove_file(&temp);
        return Err(e.into());
    }
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Prefs {
    pub enabled: bool,
    pub retention_days: u32,
}
impl Default for Prefs {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: 7,
        }
    }
}
#[derive(Clone, Serialize, Default)]
pub struct Status {
    pub phase: String,
    pub music_pid: i32,
    pub installed: bool,
    pub requests: u64,
    pub edits: u64,
    pub last_ms: u64,
    pub cached: usize,
    pub menu_ready: bool,
    pub last_error: Option<String>,
}
pub enum Action {
    Toggle,
    Retention(u32),
    Login,
    Clear,
    Reconnect,
    Quit,
}
pub struct App {
    pub state: PathBuf,
    pub logs: PathBuf,
    pub prefs: Mutex<Prefs>,
    pub status: Mutex<Status>,
    pub stop: AtomicBool,
    pub tx: mpsc::UnboundedSender<Action>,
    log_lock: Mutex<()>,
}
impl App {
    pub fn enabled(&self) -> bool {
        self.prefs.lock().unwrap().enabled
    }
    pub fn save_prefs(&self) {
        let _ = atomic_json(
            &self.state.join("preferences.json"),
            &serde_json::to_value(&*self.prefs.lock().unwrap()).unwrap(),
        );
    }
    pub fn log(&self, event: &str, data: Value) {
        let _guard = self.log_lock.lock().unwrap();
        let path = self
            .logs
            .join(format!("{}.jsonl", chrono::Local::now().format("%Y-%m-%d")));
        if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) > 4 * 1024 * 1024 {
            return;
        }
        let v = json!({"time":chrono::Local::now().to_rfc3339(),"event":event,"data":data});
        if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(path) {
            use std::os::unix::fs::PermissionsExt;
            let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
            let _ = writeln!(f, "{v}");
        }
    }
    pub fn purge_logs(&self) {
        let days = self.prefs.lock().unwrap().retention_days;
        purge(&self.logs, chrono::Local::now().date_naive(), days);
    }
    pub fn publish_status(&self) {
        let _ = atomic_json(
            &self.state.join("status.json"),
            &serde_json::to_value(&*self.status.lock().unwrap()).unwrap(),
        );
    }
    pub fn phase(&self, phase: &str) {
        self.status.lock().unwrap().phase = phase.into();
        self.publish_status();
    }
}
pub fn purge(dir: &Path, today: chrono::NaiveDate, days: u32) {
    let first = today - chrono::Duration::days(days.max(1) as i64 - 1);
    if let Ok(files) = fs::read_dir(dir) {
        for f in files.flatten() {
            let p = f.path();
            if p.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }
            if p.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .is_some_and(|d| d < first)
            {
                let _ = fs::remove_file(p);
            }
        }
    }
}
fn dirs() -> Result<(PathBuf, PathBuf)> {
    let home = std::env::var("HOME")?;
    let state = std::env::var_os("MNN_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(&home).join("Library/Application Support/MusicNativeNames")
        });
    let logs = std::env::var_os("MNN_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(home).join("Library/Logs/MusicNativeNames"));
    fs::create_dir_all(&state)?;
    fs::create_dir_all(&logs)?;
    Ok((state, logs))
}
fn lock_instance(state: &Path) -> Result<File> {
    use std::os::fd::AsRawFd;
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(state.join("instance.lock"))?;
    if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        anyhow::bail!("Music Native Names is already running")
    }
    Ok(f)
}
async fn serve(app: Arc<App>, resolver: Arc<resolver::Resolver>, listener: TcpListener) {
    let slots = Arc::new(Semaphore::new(16));
    while !app.stop.load(Ordering::Relaxed) {
        let Ok((mut socket, _)) = listener.accept().await else {
            break;
        };
        let Ok(permit) = slots.clone().try_acquire_owned() else {
            drop(socket);
            continue;
        };
        let a = app.clone();
        let r = resolver.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let _ = tokio::time::timeout(Duration::from_secs(7), async {
                let n = socket.read_u64_le().await?;
                if n == 0 || n > 8 * 1024 * 1024 || !a.enabled() {
                    socket.write_u64_le(0).await?;
                    return Ok::<(), anyhow::Error>(());
                }
                let mut data = vec![0; n as usize];
                socket.read_exact(&mut data).await?;
                let result = r.transform(&data).await;
                match result {
                    Ok((bytes, metrics)) => {
                        if a.enabled() && !a.stop.load(Ordering::Relaxed) {
                            socket.write_u64_le(bytes.len() as u64).await?;
                            socket.write_all(&bytes).await?;
                        } else {
                            r.publish(false)?;
                            socket.write_u64_le(0).await?;
                        }
                        let mut s = a.status.lock().unwrap();
                        s.requests += 1;
                        s.edits += metrics["edits"].as_u64().unwrap_or(0);
                        s.last_ms = metrics["ms"].as_u64().unwrap_or(0);
                        if s.installed {
                            let unavailable = metrics["catalogErrors"].as_u64().unwrap_or(0) > 0
                                || metrics["timedOut"] == true;
                            s.last_error = if unavailable {
                                Some("目录查询暂不可用，保留已有显示".into())
                            } else {
                                None
                            };
                            s.phase = if unavailable {
                                "目录暂不可用 · 使用已有缓存"
                            } else {
                                "原文显示已启用"
                            }
                            .into();
                        }
                        drop(s);
                        a.log("response", metrics);
                    }
                    Err(_) => {
                        socket.write_u64_le(0).await?;
                        a.log(
                            "response_error",
                            json!({"reason":"invalid response or cache write failed"}),
                        );
                    }
                }
                Ok(())
            })
            .await;
        });
    }
}
async fn hook(app: Arc<App>, action: &str, pid: i32) -> bool {
    let a = app.clone();
    let action = action.to_string();
    tokio::task::spawn_blocking(move || bridge::control(&a, &action, pid))
        .await
        .unwrap_or(false)
}
async fn worker(
    app: Arc<App>,
    mut rx: mpsc::UnboundedReceiver<Action>,
    port: u16,
    no_hooks: bool,
) -> Result<()> {
    let resolver = resolver::Resolver::new(app.state.clone())?;
    resolver.set_enabled(app.enabled())?;
    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .context("local resolver port is already in use")?;
    app.log("started",json!({"version":env!("CARGO_PKG_VERSION"),"port":port,"runtime":"Rust","retentionDays":app.prefs.lock().unwrap().retention_days}));
    let server = tokio::spawn(serve(app.clone(), resolver.clone(), listener));
    let c = resolver.catalog.clone();
    let a = app.clone();
    tokio::spawn(async move {
        match c.warm().await {
            Ok(_) => a.log("catalog_ready", json!({})),
            Err(_) => a.log("catalog_unavailable", json!({"retry":"next request"})),
        }
    });
    let mut handled = 0;
    let mut installed = 0;
    let mut ticks = 0u64;
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    loop {
        tokio::select! {
         _=term.recv()=>{break},_ = tokio::signal::ctrl_c()=>{break},
         action=rx.recv()=>{match action {
          Some(Action::Quit)|None=>break,
          Some(Action::Toggle)=>{
            let enabled={let mut p=app.prefs.lock().unwrap();p.enabled = !p.enabled;p.enabled};
            app.save_prefs();resolver.set_enabled(enabled)?;
            if installed>0&&!no_hooks {
                let okay=hook(app.clone(),if enabled{"enable"}else{"remove"},installed).await;
                let mut s=app.status.lock().unwrap();s.installed=enabled&&okay;
                s.phase=if !enabled{"已暂停"}else if okay{"原文显示已启用"}else{"连接失败，查看 Log"}.into();
                handled=if okay{installed}else{0};
            } else {handled=0;app.phase(if enabled{"等待 Music"}else{"已暂停"});}
            app.publish_status();app.log("enabled_changed",json!({"enabled":enabled}));
          },
          Some(Action::Retention(days))=>{app.prefs.lock().unwrap().retention_days=days;app.save_prefs();app.purge_logs();app.log("retention_changed",json!({"days":days}));},
          Some(Action::Login)=>{match platform::set_login(!platform::login_enabled()){Ok(())=>app.log("login_changed",json!({"enabled":platform::login_enabled()})),Err(_)=>{app.status.lock().unwrap().last_error=Some("登录自启设置失败，请查看 Log".into());app.log("login_error",json!({}));}}},
          Some(Action::Clear)=>{resolver.clear()?;app.log("cache_cleared",json!({}));},
          Some(Action::Reconnect)=>{handled=0;app.log("reconnect",json!({}));}
         }},
         _=tick.tick()=>{ticks+=1;let enabled=app.enabled();let n=resolver.publish(enabled)?;app.status.lock().unwrap().cached=n;if ticks%720==1{app.purge_logs();}let pid=if no_hooks{0}else{platform::music_pid()};
          if pid==0{installed=0;handled=0;let mut s=app.status.lock().unwrap();s.music_pid=0;s.installed=false;s.phase=if enabled{"等待 Music"}else{"已暂停"}.into();}
          else if enabled&&pid!=handled{handled=pid;app.phase("正在连接 Music…");let action=if pid==installed{"enable"}else{"install"};let okay=hook(app.clone(),action,pid).await;if okay{installed=pid;}let mut s=app.status.lock().unwrap();s.music_pid=pid;s.installed=okay;s.phase=if okay{"原文显示已启用"}else{"连接失败，查看 Log"}.into();s.last_error=if okay{None}else{Some("无法安装 Music 原生桥接".into())};}
          else if !enabled{app.status.lock().unwrap().phase="已暂停".into();}
          app.publish_status();
         }
        }
    }
    app.stop.store(true, Ordering::Relaxed);
    app.phase("正在退出…");
    resolver.set_enabled(false)?;
    let restored = if installed > 0 && platform::music_pid() == installed && !no_hooks {
        hook(app.clone(), "remove", installed).await
    } else {
        true
    };
    server.abort();
    app.log("stopped", json!({"hooksRestored":restored}));
    app.phase("已退出");
    Ok(())
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (state, logs) = dirs()?;
    if args.iter().any(|a| a == "--status") {
        println!(
            "{}",
            fs::read_to_string(state.join("status.json")).unwrap_or_else(|_| "{}".into())
        );
        return Ok(());
    }
    if args.iter().any(|a| a == "--install-login") {
        return platform::set_login(true);
    }
    if args.iter().any(|a| a == "--remove-login") {
        return platform::set_login(false);
    }
    if let Some(pos) = args.iter().position(|a| a == "--bridge-status") {
        let pid = args.get(pos + 1).context("PID required")?.parse()?;
        let (tx, _) = mpsc::unbounded_channel();
        let app = Arc::new(App {
            state,
            logs,
            prefs: Mutex::new(Prefs::default()),
            status: Mutex::new(Status::default()),
            stop: AtomicBool::new(false),
            tx,
            log_lock: Mutex::new(()),
        });
        let okay = bridge::control(&app, "status", pid);
        if !okay {
            anyhow::bail!("bridge status failed")
        }
        return Ok(());
    }
    if let Some(pos) = args.iter().position(|a| a == "--resolve-file") {
        let input = PathBuf::from(args.get(pos + 1).context("input required")?);
        let output = PathBuf::from(args.get(pos + 2).context("output required")?);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        return rt.block_on(async {
            let r = resolver::Resolver::new(state)?;
            r.catalog.warm().await?;
            let raw = fs::read(input)?;
            let (bytes, metrics) = r.transform(&raw).await?;
            fs::write(output, if bytes.is_empty() { raw } else { bytes })?;
            println!("{metrics}");
            Ok(())
        });
    }
    let _lock = match lock_instance(&state) {
        Ok(f) => f,
        Err(_) => return Ok(()),
    };
    let prefs = fs::read(state.join("preferences.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Prefs>(&b).ok())
        .filter(|p| [1, 7, 14, 30].contains(&p.retention_days))
        .unwrap_or_default();
    let (tx, rx) = mpsc::unbounded_channel();
    let app = Arc::new(App {
        state,
        logs,
        prefs: Mutex::new(prefs),
        status: Mutex::new(Status {
            phase: "启动中…".into(),
            ..Default::default()
        }),
        stop: AtomicBool::new(false),
        tx,
        log_lock: Mutex::new(()),
    });
    app.save_prefs();
    app.purge_logs();
    let a = app.clone();
    std::panic::set_hook(Box::new(move |_| {
        a.log("panic", json!({"reason":"unexpected internal error"}));
    }));
    let port = std::env::var("MNN_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(18090);
    let no_hooks = args.iter().any(|a| a == "--no-hooks");
    let headless = args.iter().any(|a| a == "--headless");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(2)
        .enable_all()
        .build()?;
    if headless {
        return rt.block_on(worker(app, rx, port, no_hooks));
    }
    let a = app.clone();
    rt.spawn(async move {
        if worker(a.clone(), rx, port, no_hooks).await.is_err() {
            a.log(
                "fatal",
                json!({"reason":"service initialization or persistence failed"}),
            );
            a.stop.store(true, Ordering::Relaxed);
            std::process::exit(1);
        }
        a.stop.store(true, Ordering::Relaxed);
    });
    platform::run_menu(app);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retention_keeps_exactly_seven_dates() {
        let d = std::env::temp_dir().join(format!("mnn-log-test-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        for n in 0..10 {
            fs::write(
                d.join(format!("{}.jsonl", today - chrono::Duration::days(n))),
                "{}",
            )
            .unwrap();
        }
        fs::write(d.join("keep.txt"), "x").unwrap();
        purge(&d, today, 7);
        assert_eq!(fs::read_dir(&d).unwrap().count(), 8);
        assert!(!d.join("2026-09-01.jsonl").exists());
        assert!(d.join("2026-09-02.jsonl").exists());
        fs::remove_dir_all(d).unwrap();
    }
}
