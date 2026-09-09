use crate::indicator::Indicator;
use crate::{Action, App};
use anyhow::{bail, Result};
use std::{
    ffi::{c_void, CStr, CString},
    path::PathBuf,
    process::Command,
    ptr,
    sync::{atomic::Ordering, Arc, OnceLock},
};
type Id = *mut c_void;
type Sel = *mut c_void;
#[link(name = "AppKit", kind = "framework")]
extern "C" {}
#[link(name = "Foundation", kind = "framework")]
extern "C" {
    static NSRunLoopCommonModes: Id;
}
#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const i8) -> Id;
    fn sel_registerName(name: *const i8) -> Sel;
    fn objc_msgSend();
    fn objc_allocateClassPair(superclass: Id, name: *const i8, extra: usize) -> Id;
    fn objc_registerClassPair(cls: Id);
    fn class_addMethod(cls: Id, sel: Sel, imp: *const c_void, types: *const i8) -> bool;
}
#[link(name = "proc")]
extern "C" {
    fn proc_listpids(kind: u32, typeinfo: u32, buffer: *mut c_void, buffersize: i32) -> i32;
    fn proc_name(pid: i32, buffer: *mut c_void, buffersize: u32) -> i32;
    fn proc_pidpath(pid: i32, buffer: *mut c_void, buffersize: u32) -> i32;
}
unsafe fn cls(s: &str) -> Id {
    objc_getClass(CString::new(s).unwrap().as_ptr())
}
unsafe fn sel(s: &str) -> Sel {
    sel_registerName(CString::new(s).unwrap().as_ptr())
}
unsafe fn msg0<R>(o: Id, s: &str) -> R {
    let f: unsafe extern "C" fn(Id, Sel) -> R = std::mem::transmute(objc_msgSend as *const ());
    f(o, sel(s))
}
unsafe fn msg1<A, R>(o: Id, s: &str, a: A) -> R {
    let f: unsafe extern "C" fn(Id, Sel, A) -> R = std::mem::transmute(objc_msgSend as *const ());
    f(o, sel(s), a)
}
unsafe fn string(s: &str) -> Id {
    msg1(
        cls("NSString"),
        "stringWithUTF8String:",
        CString::new(s.replace('\0', "")).unwrap().as_ptr(),
    )
}
unsafe fn text(s: Id) -> String {
    if s.is_null() {
        return String::new();
    }
    let p: *const i8 = msg0(s, "UTF8String");
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}
pub fn music_pid() -> i32 {
    unsafe {
        let mut pids = [0i32; 4096];
        let n = proc_listpids(
            1,
            0,
            pids.as_mut_ptr().cast(),
            std::mem::size_of_val(&pids) as i32,
        );
        for pid in pids.iter().take((n.max(0) as usize / 4).min(pids.len())) {
            let mut name = [0i8; 128];
            if proc_name(*pid, name.as_mut_ptr().cast(), 128) > 0
                && CStr::from_ptr(name.as_ptr()).to_bytes() == b"Music"
            {
                let mut path = [0i8; 4096];
                if proc_pidpath(*pid, path.as_mut_ptr().cast(), 4096) > 0
                    && CStr::from_ptr(path.as_ptr()).to_bytes()
                        == b"/System/Applications/Music.app/Contents/MacOS/Music"
                {
                    return *pid;
                }
            }
        }
        0
    }
}
static LOGIN_CACHE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const LABEL: &str = "local.music-native-names.rust";
fn plist() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join(format!("Library/LaunchAgents/{LABEL}.plist"))
}
fn target() -> String {
    format!("gui/{}/{LABEL}", unsafe { libc::getuid() })
}
pub fn login_enabled() -> bool {
    if !plist().exists() {
        return false;
    }
    Command::new("/bin/launchctl")
        .args([
            "print-disabled",
            &format!("gui/{}", unsafe { libc::getuid() }),
        ])
        .output()
        .ok()
        .is_some_and(|o| {
            let text = String::from_utf8_lossy(&o.stdout);
            !text.contains(&format!("\"{LABEL}\" => true"))
                && !text.contains(&format!("\"{LABEL}\" => disabled"))
        })
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn set_login(enabled: bool) -> Result<()> {
    let p = plist();
    if enabled {
        let exe = std::env::current_exe()?;
        std::fs::create_dir_all(p.parent().unwrap())?;
        let doc = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>Label</key><string>{LABEL}</string><key>ProgramArguments</key><array><string>{}</string></array><key>RunAtLoad</key><true/><key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict><key>LimitLoadToSessionType</key><string>Aqua</string><key>ThrottleInterval</key><integer>30</integer><key>ExitTimeOut</key><integer>90</integer><key>StandardOutPath</key><string>/dev/null</string><key>StandardErrorPath</key><string>/dev/null</string></dict></plist>"#,
            xml(&exe.to_string_lossy())
        );
        crate::atomic_write(&p, doc.as_bytes())?;
        if !Command::new("/bin/launchctl")
            .args(["enable", &target()])
            .status()?
            .success()
        {
            bail!("launchctl enable failed")
        }
    } else {
        if !Command::new("/bin/launchctl")
            .args(["disable", &target()])
            .status()?
            .success()
        {
            bail!("launchctl disable failed")
        }
        if p.exists() {
            std::fs::remove_file(p)?;
        }
    }
    LOGIN_CACHE.store(enabled, Ordering::Relaxed);
    Ok(())
}
static APP: OnceLock<Arc<App>> = OnceLock::new();
// AppKit objects are created and used exclusively on the main thread.
#[cfg(feature = "ui-test")]
mod menu_test;
static mut STATUS: Id = ptr::null_mut();
static mut INFO: Id = ptr::null_mut();
static mut METRICS: Id = ptr::null_mut();
static mut ENABLE: Id = ptr::null_mut();
static mut LOGIN: Id = ptr::null_mut();
static mut RETENTION: [Id; 4] = [ptr::null_mut(); 4];
unsafe fn title(item: Id, s: &str) {
    let current: Id = msg0(item, "title");
    if text(current) != s {
        msg1::<_, ()>(item, "setTitle:", string(s));
    }
}
unsafe extern "C" fn tick(_: Id, _: Sel, _: Id) {
    let Some(app) = APP.get() else { return };
    let pool: Id = msg0(cls("NSAutoreleasePool"), "new");
    if app.stop.load(Ordering::Relaxed) && app.status.lock().unwrap().phase == "已退出" {
        std::process::exit(0)
    }
    let s = app.status.lock().unwrap().clone();
    let p = app.prefs.lock().unwrap().clone();
    title(INFO, &s.phase);
    title(
        METRICS,
        &format!("已缓存 {} 项 · 已替换 {} 处", s.cached, s.edits),
    );
    msg1::<_, ()>(ENABLE, "setState:", if p.enabled { 1isize } else { 0 });
    msg1::<_, ()>(
        LOGIN,
        "setState:",
        if LOGIN_CACHE.load(Ordering::Relaxed) {
            1isize
        } else {
            0
        },
    );
    for (i, days) in [1, 7, 14, 30].iter().enumerate() {
        msg1::<_, ()>(
            RETENTION[i],
            "setState:",
            if p.retention_days == *days { 1isize } else { 0 },
        );
    }
    let button: Id = msg0(STATUS, "button");
    let indicator = Indicator::from_status(p.enabled, &s);
    set_indicator(button, indicator);
    msg1::<_, ()>(
        button,
        "setToolTip:",
        string(&format!("Music Native Names · {}", s.phase)),
    );
    msg1::<_, ()>(
        button,
        "setAccessibilityLabel:",
        string(&format!("Music Native Names — {}", indicator.label())),
    );
    #[cfg(feature = "ui-test")]
    if std::env::args().any(|a| a == "--self-test-menu") {
        menu_test::menu_self_test(app);
    }
    msg0::<()>(pool, "drain");
}
unsafe fn set_indicator(button: Id, indicator: Indicator) {
    // Template symbols follow the menu bar appearance; semantic colors carry state.
    let image_fn: unsafe extern "C" fn(Id, Sel, Id, Id) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    let image = image_fn(
        cls("NSImage"),
        sel("imageWithSystemSymbolName:accessibilityDescription:"),
        string(indicator.symbol()),
        string(indicator.label()),
    );
    if image.is_null() {
        title(button, "♫");
    } else {
        title(button, "");
        msg1::<_, ()>(image, "setTemplate:", true);
        msg1::<_, ()>(button, "setImage:", image);
    }
    let color: Id = msg0(cls("NSColor"), indicator.color_selector());
    msg1::<_, ()>(button, "setContentTintColor:", color);
}

unsafe fn show_about() {
    let app: Id = msg0(cls("NSApplication"), "sharedApplication");
    msg1::<_, ()>(app, "activateIgnoringOtherApps:", true);
    let alert: Id = msg0(cls("NSAlert"), "new");
    msg1::<_, ()>(alert, "setMessageText:", string("Music Native Names"));
    msg1::<_, ()>(
        alert,
        "setInformativeText:",
        string(&format!(
            "Version {}\nMIT License\n\nOriginal-language names for Apple Music.\n\n{}",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_REPOSITORY"),
        )),
    );
    msg1::<_, Id>(alert, "addButtonWithTitle:", string("GitHub"));
    msg1::<_, Id>(alert, "addButtonWithTitle:", string("关闭"));
    let response: isize = msg0(alert, "runModal");
    msg0::<()>(alert, "release");
    if response == 1000 {
        let _ = Command::new("/usr/bin/open")
            .arg(env!("CARGO_PKG_REPOSITORY"))
            .spawn();
    }
}

unsafe extern "C" fn clicked(_: Id, _: Sel, sender: Id) {
    let Some(app) = APP.get() else { return };
    let tag: isize = msg0(sender, "tag");
    let action = match tag {
        1 => Some(Action::Toggle),
        2 => {
            let _ = Command::new("/usr/bin/open")
                .args(["-b", "com.apple.Music"])
                .spawn();
            None
        }
        3 => Some(Action::Login),
        4 => {
            let _ = Command::new("/usr/bin/open").arg(&app.logs).spawn();
            None
        }
        5 => Some(Action::Clear),
        6 => Some(Action::Reconnect),
        7 => Some(Action::Quit),
        8 => {
            show_about();
            None
        }
        101 => Some(Action::Retention(1)),
        107 => Some(Action::Retention(7)),
        114 => Some(Action::Retention(14)),
        130 => Some(Action::Retention(30)),
        _ => None,
    };
    if let Some(a) = action {
        let _ = app.tx.send(a);
    }
}
unsafe fn menu_item(menu: Id, name: &str, tag: isize, target: Id) -> Id {
    let alloc: Id = msg0(cls("NSMenuItem"), "alloc");
    let f: unsafe extern "C" fn(Id, Sel, Id, Sel, Id) -> Id =
        std::mem::transmute(objc_msgSend as *const ());
    let item = f(
        alloc,
        sel("initWithTitle:action:keyEquivalent:"),
        string(name),
        if tag == 0 {
            ptr::null_mut()
        } else {
            sel("clicked:")
        },
        string(""),
    );
    if tag != 0 {
        msg1::<_, ()>(item, "setTarget:", target);
        msg1::<_, ()>(item, "setTag:", tag);
    } else {
        msg1::<_, ()>(item, "setEnabled:", false)
    }
    msg1::<_, ()>(menu, "addItem:", item);
    msg0::<()>(item, "release");
    item
}
unsafe fn separator(menu: Id) {
    let sep: Id = msg0(cls("NSMenuItem"), "separatorItem");
    msg1::<_, ()>(menu, "addItem:", sep);
}
pub fn run_menu(app: Arc<App>) {
    LOGIN_CACHE.store(login_enabled(), Ordering::Relaxed);
    let _ = APP.set(app.clone());
    unsafe {
        let pool: Id = msg0(cls("NSAutoreleasePool"), "new");
        let ns: Id = msg0(cls("NSApplication"), "sharedApplication");
        msg1::<_, bool>(ns, "setActivationPolicy:", 1isize);
        let class = objc_allocateClassPair(
            cls("NSObject"),
            CString::new("MNNMenuTarget").unwrap().as_ptr(),
            0,
        );
        class_addMethod(
            class,
            sel("clicked:"),
            clicked as *const c_void,
            c"v@:@".as_ptr(),
        );
        class_addMethod(class, sel("tick:"), tick as *const c_void, c"v@:@".as_ptr());
        objc_registerClassPair(class);
        let target: Id = msg0(class, "new");
        let bar: Id = msg0(cls("NSStatusBar"), "systemStatusBar");
        STATUS = msg1(bar, "statusItemWithLength:", -1.0f64);
        msg0::<Id>(STATUS, "retain");
        let menu: Id = msg0(cls("NSMenu"), "new");
        #[cfg(feature = "ui-test")]
        {
            menu_test::MAIN_MENU = menu;
        }
        msg1::<_, ()>(menu, "setAutoenablesItems:", false);
        INFO = menu_item(menu, "启动中…", 0, target);
        METRICS = menu_item(menu, "", 0, target);
        separator(menu);
        ENABLE = menu_item(menu, "启用原文显示", 1, target);
        menu_item(menu, "打开 Music", 2, target);
        LOGIN = menu_item(menu, "登录时自动启动", 3, target);
        separator(menu);
        let log = menu_item(menu, "Log", 0, target);
        msg1::<_, ()>(log, "setEnabled:", true);
        let submenu: Id = msg0(cls("NSMenu"), "new");
        msg1::<_, ()>(submenu, "setAutoenablesItems:", false);
        menu_item(submenu, "打开 Log 文件夹…", 4, target);
        separator(submenu);
        for (i, days) in [1, 7, 14, 30].iter().enumerate() {
            RETENTION[i] = menu_item(
                submenu,
                &format!("保留 {days} 天{}", if *days == 7 { "（默认）" } else { "" }),
                100 + *days as isize,
                target,
            );
        }
        msg1::<_, ()>(log, "setSubmenu:", submenu);
        msg0::<()>(submenu, "release");
        menu_item(menu, "清空名称缓存", 5, target);
        menu_item(menu, "重新连接 Music", 6, target);
        separator(menu);
        menu_item(menu, "关于 Music Native Names…", 8, target);
        menu_item(menu, "退出", 7, target);
        msg1::<_, ()>(STATUS, "setMenu:", menu);
        msg0::<()>(menu, "release");
        let f: unsafe extern "C" fn(Id, Sel, f64, Id, Sel, Id, bool) -> Id =
            std::mem::transmute(objc_msgSend as *const ());
        let timer = f(
            cls("NSTimer"),
            sel("timerWithTimeInterval:target:selector:userInfo:repeats:"),
            2.0,
            target,
            sel("tick:"),
            ptr::null_mut(),
            true,
        );
        let runloop: Id = msg0(cls("NSRunLoop"), "mainRunLoop");
        let add: unsafe extern "C" fn(Id, Sel, Id, Id) =
            std::mem::transmute(objc_msgSend as *const ());
        add(
            runloop,
            sel("addTimer:forMode:"),
            timer,
            NSRunLoopCommonModes,
        );
        app.status.lock().unwrap().menu_ready = true;
        app.publish_status();
        app.log(
            "menu_ready",
            serde_json::json!({"items":13,"native":"AppKit"}),
        );
        tick(target, sel("tick:"), ptr::null_mut());
        msg0::<()>(ns, "run");
        msg0::<()>(pool, "drain");
    }
}
