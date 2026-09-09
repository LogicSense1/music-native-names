use crate::now;
use anyhow::{bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use futures::{stream, StreamExt};
use regex::Regex;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{Mutex, Semaphore};
pub type Group = (String, String, bool);
pub type Jobs = BTreeMap<Group, BTreeSet<String>>;
type ResourceKey = (String, String, bool, String);
type CachedResponse = (f64, Vec<Value>, usize);
pub type Results = HashMap<ResourceKey, Vec<Value>>;
const ORIGIN: &str = "https://music.apple.com";
const API: &str = "https://amp-api.music.apple.com/v1/";
const MAX: usize = 12 * 1024 * 1024;
struct Token {
    text: String,
    expires: f64,
}
pub struct Catalog {
    client: reqwest::Client,
    token: Mutex<Option<Token>>,
    cache: Mutex<HashMap<ResourceKey, CachedResponse>>,
    cache_bytes: AtomicUsize,
    permits: Semaphore,
    pub requests: AtomicU64,
    pub errors: AtomicU64,
}
impl Catalog {
    pub fn new() -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(4))
                .connect_timeout(Duration::from_secs(3))
                .pool_max_idle_per_host(6)
                .pool_idle_timeout(Duration::from_secs(60))
                .user_agent("Mozilla/5.0")
                .build()?,
            token: Mutex::new(None),
            cache: Mutex::new(HashMap::new()),
            cache_bytes: AtomicUsize::new(0),
            permits: Semaphore::new(6),
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }))
    }
    async fn get(&self, url: &str, token: Option<&str>) -> Result<Vec<u8>> {
        let _permit = self.permits.acquire().await?;
        let mut r = self.client.get(url);
        if let Some(t) = token {
            r = r.bearer_auth(t).header("Origin", ORIGIN);
        }
        let mut response = r.send().await?.error_for_status()?;
        let mut out = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if out.len() + chunk.len() > MAX {
                bail!("catalog response too large")
            }
            out.extend_from_slice(&chunk);
        }
        Ok(out)
    }
    pub async fn warm(&self) -> Result<String> {
        let mut auth = self.token.lock().await;
        if let Some(t) = auth.as_ref() {
            if t.expires > now() {
                return Ok(t.text.clone());
            }
        }
        let page = String::from_utf8(self.get(&format!("{ORIGIN}/"), None).await?)?;
        let scripts = Regex::new(r#"src=["'](/assets/index[^/"']*\.js)["']"#)?;
        let tokens = Regex::new(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+")?;
        for path in scripts.captures_iter(&page).take(2) {
            let script =
                String::from_utf8(self.get(&format!("{ORIGIN}{}", &path[1]), None).await?)?;
            for candidate in tokens
                .find_iter(&script)
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
            {
                let Some(payload) = candidate
                    .split('.')
                    .nth(1)
                    .and_then(|p| URL_SAFE_NO_PAD.decode(p).ok())
                    .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                else {
                    continue;
                };
                let exp = payload["exp"].as_f64().unwrap_or(0.0);
                if exp < now() + 60.0 {
                    continue;
                }
                let Ok(bytes) = self
                    .get(&format!("{API}catalog/us/genres"), Some(candidate))
                    .await
                else {
                    continue;
                };
                let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
                    continue;
                };
                if v["data"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().any(|r| r["type"] == "genres"))
                {
                    *auth = Some(Token {
                        text: candidate.into(),
                        expires: (exp - 60.0).min(now() + 3600.0),
                    });
                    return Ok(candidate.into());
                }
            }
        }
        bail!("public catalog bootstrap unavailable")
    }
    async fn request(&self, g: &Group, ids: &[String]) -> Result<Vec<(String, Vec<Value>)>> {
        let token = self.warm().await?;
        let mut url = reqwest::Url::parse(&format!("{API}catalog/{}/{}", g.0, g.1))?;
        {
            let mut q = url.query_pairs_mut();
            q.append_pair(
                if g.2 { "filter[equivalents]" } else { "ids" },
                &ids.join(","),
            );
            q.append_pair(
                "include",
                match g.1.as_str() {
                    "songs" => "artists,albums",
                    "albums" => "artists,tracks",
                    _ => "albums",
                },
            );
            q.append_pair("limit[tracks]", "3")
                .append_pair("limit[albums]", "1")
                .append_pair("extend", "artistBio");
            q.append_pair("fields[songs]","name,artistName,albumName,albumArtistName,audioLocale,isrc,durationInMillis,discNumber,trackNumber,releaseDate,genreNames,contentRating");
            q.append_pair(
                "fields[albums]",
                "name,artistName,trackCount,genreNames,releaseDate,upc,contentRating",
            );
            q.append_pair("fields[artists]", "name,genreNames,artistBio");
        }
        self.requests.fetch_add(1, Ordering::Relaxed);
        let bytes = match self.get(url.as_str(), Some(&token)).await {
            Ok(bytes) => bytes,
            Err(error) => {
                if error
                    .downcast_ref::<reqwest::Error>()
                    .and_then(|e| e.status())
                    .is_some_and(|s| s == reqwest::StatusCode::UNAUTHORIZED)
                {
                    *self.token.lock().await = None;
                }
                return Err(error);
            }
        };
        let v: Value = serde_json::from_slice(&bytes)?;
        let rows: HashMap<String, Value> = v["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|r| Some((r["id"].as_str()?.to_string(), r.clone())))
            .collect();
        if std::env::var_os("MNN_DEBUG_CATALOG").is_some() {
            eprintln!(
                "catalog {} {} equivalent={} data={} ids={}",
                g.0,
                g.1,
                g.2,
                rows.len(),
                ids.len()
            );
        }
        let mut out = Vec::new();
        let mut cache = self.cache.lock().await;
        for id in ids {
            let values = if g.2 {
                v["meta"]["filters"]["equivalents"][id]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|r| rows.get(r["id"].as_str()?))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                rows.get(id).cloned().into_iter().collect()
            };
            let size = values.iter().map(|r| r.to_string().len()).sum::<usize>();
            let old = cache.insert(
                (g.0.clone(), g.1.clone(), g.2, id.clone()),
                (
                    now() + if values.is_empty() { 300.0 } else { 86400.0 },
                    values.clone(),
                    size,
                ),
            );
            self.cache_bytes.fetch_add(size, Ordering::Relaxed);
            if let Some((_, _, old_size)) = old {
                self.cache_bytes.fetch_sub(old_size, Ordering::Relaxed);
            }
            out.push((id.clone(), values));
        }
        // Incremental accounting avoids rescanning all cached biographies on each request.
        if cache.len() > 3000 || self.cache_bytes.load(Ordering::Relaxed) > 8 * 1024 * 1024 {
            cache.retain(|_, (expires, _, size)| {
                if *expires <= now() {
                    self.cache_bytes.fetch_sub(*size, Ordering::Relaxed);
                    false
                } else {
                    true
                }
            });
            while cache.len() > 2500 || self.cache_bytes.load(Ordering::Relaxed) > 6 * 1024 * 1024 {
                let Some(key) = cache.keys().next().cloned() else {
                    break;
                };
                if let Some((_, _, size)) = cache.remove(&key) {
                    self.cache_bytes.fetch_sub(size, Ordering::Relaxed);
                }
            }
        }
        Ok(out)
    }
    pub async fn batch(&self, jobs: Jobs) -> Results {
        let mut out = HashMap::new();
        let mut pending = Vec::new();
        {
            let cache = self.cache.lock().await;
            for (g, ids) in jobs {
                let mut missing = Vec::new();
                for id in ids {
                    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                        continue;
                    }
                    let key = (g.0.clone(), g.1.clone(), g.2, id.clone());
                    if let Some((e, v, _)) = cache.get(&key).filter(|(e, _, _)| *e > now()) {
                        let _ = e;
                        out.insert(key, v.clone());
                    } else {
                        missing.push(id)
                    }
                }
                for chunk in missing.chunks(100) {
                    pending.push((g.clone(), chunk.to_vec()));
                }
            }
        }
        let results = stream::iter(pending.into_iter().map(|(g, ids)| async move {
            let r = self.request(&g, &ids).await;
            (g, r)
        }))
        .buffer_unordered(6)
        .collect::<Vec<_>>()
        .await;
        for (g, result) in results {
            if result.is_err() {
                self.errors.fetch_add(1, Ordering::Relaxed);
            }
            if std::env::var_os("MNN_DEBUG_CATALOG").is_some() {
                if let Err(e) = &result {
                    eprintln!("catalog {} {} error: {}", g.0, g.1, e);
                }
            }
            if let Ok(rows) = result {
                for (id, v) in rows {
                    out.insert((g.0.clone(), g.1.clone(), g.2, id), v);
                }
            }
        }
        out
    }
}
