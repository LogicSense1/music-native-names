use crate::{
    atomic_json,
    catalog::{Catalog, Jobs, Results},
    now,
};
use anyhow::Result;
use regex::Regex;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio::sync::Semaphore;
pub const FIELDS: [&str; 4] = ["name", "artistName", "albumName", "albumArtistName"];
pub fn related<'a>(r: &'a Value, k: &str) -> Vec<&'a Value> {
    r["relationships"][k]["data"]
        .as_array()
        .into_iter()
        .flatten()
        .collect()
}
fn artist_ids(r: &Value) -> BTreeSet<String> {
    related(r, "artists")
        .iter()
        .filter_map(|a| a["id"].as_str().map(str::to_string))
        .collect()
}
pub fn resources(v: &Value, out: &mut Vec<Value>) {
    if out.len() >= 512 {
        return;
    }
    match v {
        Value::Object(m) => {
            if matches!(v["type"].as_str(), Some("songs" | "albums" | "artists"))
                && v["id"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                && v["attributes"].is_object()
            {
                out.push(v.clone())
            }
            for v in m.values() {
                resources(v, out)
            }
        }
        Value::Array(a) => {
            for v in a {
                resources(v, out)
            }
        }
        _ => {}
    }
}
fn language(a: &Value) -> Option<String> {
    let l = a["audioLocale"].as_str().unwrap_or("").to_lowercase();
    if l.starts_with("ja") {
        Some("jp".into())
    } else if l.starts_with("ko") {
        Some("kr".into())
    } else if ["zh", "cmn", "yue"].iter().any(|s| l.starts_with(s)) {
        Some("tw".into())
    } else {
        None
    }
}
fn genre(a: &Value) -> Option<String> {
    let g: BTreeSet<String> = a["genreNames"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_lowercase))
        .collect();
    if g.contains("j-pop") {
        Some("jp".into())
    } else if g.contains("k-pop") {
        Some("kr".into())
    } else if [
        "mandopop",
        "cantopop",
        "国语流行",
        "國語流行樂",
        "粤语流行",
        "粵語流行樂",
    ]
    .iter()
    .any(|s| g.contains(*s))
    {
        Some("tw".into())
    } else {
        None
    }
}
// A sparse artist profile can omit the regional genre carried by its own releases.
// Only solo-credited releases in Apple's artist relationship contribute evidence.
// Never infer the artist's origin from a compilation or a collaborator's album.
fn artist_release_country(r: &Value) -> Option<String> {
    let name = r["attributes"]["name"].as_str().filter(|s| !s.is_empty())?;
    let countries: BTreeSet<String> = related(r, "albums")
        .into_iter()
        .filter(|album| album["attributes"]["artistName"].as_str() == Some(name))
        .filter_map(|album| genre(&album["attributes"]))
        .collect();
    if countries.len() == 1 {
        countries.into_iter().next()
    } else {
        None
    }
}
pub fn artist_country(r: &Value) -> Option<String> {
    let g = genre(&r["attributes"]);
    if matches!(g.as_deref(), Some("jp" | "kr")) {
        return g;
    }
    type OriginPatterns = (Regex, Regex, Vec<(&'static str, Regex)>, Regex, Regex);
    static PATTERNS: OnceLock<OriginPatterns> = OnceLock::new();
    let (tags,space,origins,collab,western)=PATTERNS.get_or_init(||{
  let role=r"(?:singer|songwriter|composer|band|group|producer|musician|rapper|vocalist|duo|orchestra|pianist|guitarist|collective|video[ -]game developer)";
  let pats=[("jp","Japanese"),("kr","(?:South )?Korean"),("tw","(?:Taiwanese|Hong Kong)"),("cn","Chinese")].into_iter().map(|(c,a)|(c,Regex::new(&format!(r"(?i)\b{a}\b[^.!?;]{{0,48}}\b{role}\b")).unwrap())).collect();
  (Regex::new("<[^>]*>").unwrap(),Regex::new(r"\s+").unwrap(),pats,Regex::new(r"(?i)\b(?:with|influenced by|inspired by|collaborat\w* with)\b[^.!?;]*$").unwrap(),Regex::new(&format!(r"(?i)\b(?:American|British|English|Scottish|Welsh|Irish|Canadian|Australian|French|German|Italian|Spanish|Swedish|Norwegian|Danish|Finnish|Icelandic|Dutch|Belgian|Brazilian|Mexican|Russian|Indian|Indonesian|Malaysian|Singaporean|Filipino|Thai|Vietnamese)\b[^.!?;]{{0,48}}\b{role}\b")).unwrap())
 });
    let bio = tags.replace_all(r["attributes"]["artistBio"].as_str().unwrap_or(""), " ");
    let bio = space
        .replace_all(&bio, " ")
        .chars()
        .take(260)
        .collect::<String>();
    let mut found = Vec::new();
    for (c, p) in origins {
        if let Some(m) = p.find(&bio) {
            if !collab.is_match(&bio[..m.start()]) {
                found.push((*c, m.start()));
            }
        }
    }
    if western
        .find(&bio)
        .is_some_and(|w| found.iter().any(|(_, p)| w.start() < *p))
    {
        return g;
    }
    if found.len() == 1 {
        Some(found[0].0.into())
    } else if found.is_empty() {
        // Explicit non-native biography evidence takes precedence over release genres.
        if western.is_match(&bio) {
            g
        } else {
            g.or_else(|| artist_release_country(r))
        }
    } else {
        None
    }
}
fn country(r: &Value, artists: &HashMap<String, Value>) -> Option<String> {
    if r["type"] == "artists" {
        return artist_country(r);
    }
    let g = genre(&r["attributes"]);
    let l = language(&r["attributes"]);
    let origins: BTreeSet<String> = artist_ids(r)
        .iter()
        .filter_map(|i| artists.get(i).and_then(artist_country))
        .collect();
    if matches!(l.as_deref(), Some("jp" | "kr")) {
        return l;
    }
    if l.as_deref() == Some("tw") {
        return if origins.len() == 1 && origins.iter().all(|c| c == "tw" || c == "cn") {
            origins.into_iter().next()
        } else {
            l
        };
    }
    if matches!(g.as_deref(), Some("jp" | "kr")) {
        return g;
    }
    if origins.len() == 1 {
        return origins.into_iter().next();
    }
    if g.is_some() {
        return g;
    }
    let langs: BTreeSet<String> = related(r, "tracks")
        .iter()
        .filter_map(|t| language(&t["attributes"]))
        .collect();
    if langs.len() == 1 {
        langs.into_iter().next()
    } else {
        None
    }
}
fn same_artists(a: &Value, b: &Value) -> bool {
    let x = artist_ids(a);
    let y = artist_ids(b);
    x.is_empty() || y.is_empty() || !x.is_disjoint(&y)
}
pub fn same_recording(a: &Value, b: &Value) -> bool {
    a["attributes"]["durationInMillis"]
        .as_u64()
        .is_some_and(|n| n > 0)
        && a["attributes"]["durationInMillis"] == b["attributes"]["durationInMillis"]
        && a["attributes"]["contentRating"] == b["attributes"]["contentRating"]
        && same_artists(a, b)
}
fn same_album(a: &Value, b: &Value) -> bool {
    a["attributes"]["trackCount"]
        .as_u64()
        .is_some_and(|n| n > 0)
        && a["attributes"]["trackCount"] == b["attributes"]["trackCount"]
        && a["attributes"]["releaseDate"] == b["attributes"]["releaseDate"]
        && same_artists(a, b)
}
fn unanimous(rows: &[Value], fields: &[&str]) -> Value {
    let mut out = json!({});
    for f in fields {
        let values: BTreeSet<&str> = rows
            .iter()
            .filter_map(|r| r["attributes"][f].as_str())
            .collect();
        if values.len() == 1 && rows.iter().all(|r| r["attributes"][f].as_str().is_some()) {
            let s = *values.first().unwrap();
            if !s.is_empty() {
                out[*f] = json!(s);
            }
        }
    }
    out
}
fn job(j: &mut Jobs, c: &str, k: &str, e: bool, id: &str) {
    j.entry((c.into(), k.into(), e))
        .or_default()
        .insert(id.into());
}
fn get(r: &Results, c: &str, k: &str, e: bool, id: &str) -> Vec<Value> {
    r.get(&(c.into(), k.into(), e, id.into()))
        .cloned()
        .unwrap_or_default()
}
fn key(r: &Value) -> String {
    format!(
        "{}:{}",
        r["type"].as_str().unwrap_or(""),
        r["id"].as_str().unwrap_or("")
    )
}
type Prepared = HashMap<String, (String, Value)>;
fn add(p: &mut Prepared, c: &str, k: &str, id: &str, v: Value) {
    if v["name"].is_string() || v["artistName"].is_string() {
        p.insert(format!("{k}:{id}"), (c.into(), v));
    }
}
fn song_names(p: &mut Prepared, c: &str, original: &Value, rows: Vec<Value>) -> Vec<Value> {
    let good: Vec<Value> = rows
        .into_iter()
        .filter(|r| same_recording(original, r))
        .collect();
    if good.is_empty() {
        return good;
    }
    let mut names = unanimous(&good, &["name", "artistName"]);
    names["durationInMillis"] = original["attributes"]["durationInMillis"].clone();
    let editions: Vec<Value> = good
        .iter()
        .filter(|r| {
            related(original, "albums")
                .iter()
                .any(|a| related(r, "albums").iter().any(|b| same_album(a, b)))
        })
        .cloned()
        .collect();
    if !editions.is_empty() {
        names.as_object_mut().unwrap().extend(
            unanimous(&editions, &["albumName", "albumArtistName"])
                .as_object()
                .unwrap()
                .clone(),
        );
    }
    add(p, c, "songs", original["id"].as_str().unwrap_or(""), names);
    good
}
pub struct Resolver {
    pub catalog: Arc<Catalog>,
    cache: Mutex<HashMap<String, (f64, Value)>>,
    gate: Semaphore,
    enabled: AtomicBool,
    generation: AtomicU64,
    pub state: PathBuf,
}
impl Resolver {
    pub fn new(state: PathBuf) -> Result<Arc<Self>> {
        let saved = std::fs::read(state.join("names-cache.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<HashMap<String, (f64, Value)>>(&b).ok())
            .unwrap_or_default();
        Ok(Arc::new(Self {
            catalog: Catalog::new()?,
            cache: Mutex::new(saved),
            gate: Semaphore::new(2),
            enabled: AtomicBool::new(true),
            generation: AtomicU64::new(0),
            state,
        }))
    }
    pub fn snapshot(&self) -> HashMap<String, Value> {
        self.cache
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, (e, _))| *e > now())
            .map(|(k, (_, v))| (k.clone(), v.clone()))
            .collect()
    }
    pub fn publish(&self, enabled: bool) -> Result<usize> {
        let cache = self.cache.lock().unwrap();
        let names: HashMap<&str, &Value> = cache
            .iter()
            .filter(|(_, (e, _))| enabled && self.enabled.load(Ordering::Relaxed) && *e > now())
            .map(|(k, (_, v))| (k.as_str(), v))
            .collect();
        atomic_json(
            &self.state.join("display-cache.json"),
            &json!({"expires":now()+15.0,"names":names}),
        )?;
        Ok(names.len())
    }
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        {
            let _guard = self.cache.lock().unwrap();
            self.enabled.store(enabled, Ordering::Relaxed);
            self.generation.fetch_add(1, Ordering::Relaxed);
        }
        self.publish(enabled)?;
        Ok(())
    }
    pub fn clear(&self) -> Result<()> {
        {
            let mut cache = self.cache.lock().unwrap();
            self.generation.fetch_add(1, Ordering::Relaxed);
            cache.clear();
            atomic_json(&self.state.join("names-cache.json"), &json!({}))?;
        }
        self.publish(false)?;
        Ok(())
    }
    async fn resolve(&self, entries: Vec<Value>) -> Prepared {
        let _permit = self.gate.acquire().await.unwrap();
        let mut jobs = Jobs::new();
        for r in &entries {
            job(
                &mut jobs,
                "us",
                r["type"].as_str().unwrap(),
                false,
                r["id"].as_str().unwrap(),
            );
        }
        let fetched = self.catalog.batch(jobs).await;
        let mut originals = HashMap::new();
        for r in &entries {
            let rows = get(
                &fetched,
                "us",
                r["type"].as_str().unwrap(),
                false,
                r["id"].as_str().unwrap(),
            );
            if rows.len() == 1 {
                originals.insert(key(r), rows[0].clone());
            }
        }
        let mut all = Vec::new();
        for r in originals.values() {
            resources(r, &mut all)
        }
        let mut artists: HashMap<String, Value> = all
            .iter()
            .filter(|r| r["type"] == "artists")
            .map(|r| (r["id"].as_str().unwrap().into(), r.clone()))
            .collect();
        let mut ids = BTreeSet::new();
        for r in originals.values() {
            ids.extend(artist_ids(r));
            for t in related(r, "tracks") {
                ids.extend(artist_ids(t));
            }
        }
        let mut jobs = Jobs::new();
        for id in ids {
            if !artists.contains_key(&id) {
                job(&mut jobs, "us", "artists", false, &id)
            }
        }
        for ((_, _, _, id), rows) in self.catalog.batch(jobs).await {
            if rows.len() == 1 {
                artists.insert(id, rows[0].clone());
            }
        }
        let sources: HashMap<String, Option<String>> = originals
            .iter()
            .map(|(k, r)| (k.clone(), country(r, &artists)))
            .collect();
        let mut jobs = Jobs::new();
        for (k, r) in &originals {
            let Some(c) = sources[k].as_deref() else {
                continue;
            };
            let kind = r["type"].as_str().unwrap();
            job(
                &mut jobs,
                c,
                kind,
                kind != "artists",
                r["id"].as_str().unwrap(),
            );
            if kind == "albums" {
                for t in related(r, "tracks").iter().take(3) {
                    if let Some(id) = t["id"].as_str() {
                        job(&mut jobs, c, "songs", true, id)
                    }
                }
            }
            for id in artist_ids(r) {
                job(&mut jobs, c, "artists", false, &id)
            }
        }
        let localized = self.catalog.batch(jobs).await;
        let mut prepared = Prepared::new();
        for (k, r) in &originals {
            let Some(c) = sources[k].as_deref() else {
                continue;
            };
            let kind = r["type"].as_str().unwrap();
            let id = r["id"].as_str().unwrap();
            let rows = get(&localized, c, kind, kind != "artists", id);
            match kind {
                "songs" => {
                    song_names(&mut prepared, c, r, rows);
                }
                "artists" => {
                    if rows.len() == 1 && rows[0]["id"] == id {
                        add(&mut prepared, c, kind, id, unanimous(&rows, &["name"]));
                    }
                }
                _ => {
                    let mut matches: Vec<Value> =
                        rows.into_iter().filter(|a| same_album(r, a)).collect();
                    if matches.is_empty() {
                        let mut shared: Option<BTreeSet<String>> = None;
                        let mut by_id = HashMap::new();
                        for t in related(r, "tracks").iter().take(3) {
                            let Some(tid) = t["id"].as_str() else {
                                continue;
                            };
                            let good = song_names(
                                &mut prepared,
                                c,
                                t,
                                get(&localized, c, "songs", true, tid),
                            );
                            let mut options = BTreeSet::new();
                            for s in good {
                                if t["attributes"]["discNumber"] != s["attributes"]["discNumber"]
                                    || t["attributes"]["trackNumber"]
                                        != s["attributes"]["trackNumber"]
                                {
                                    continue;
                                }
                                for album in related(&s, "albums") {
                                    if same_album(r, album) {
                                        if let Some(aid) = album["id"].as_str() {
                                            options.insert(aid.to_string());
                                            by_id.insert(aid.to_string(), album.clone());
                                        }
                                    }
                                }
                            }
                            shared = Some(match shared {
                                None => options,
                                Some(old) => old.intersection(&options).cloned().collect(),
                            });
                        }
                        matches = shared
                            .unwrap_or_default()
                            .iter()
                            .filter_map(|id| by_id.get(id).cloned())
                            .collect();
                    }
                    if matches.len() == 1 {
                        let values = unanimous(&matches, &["name", "artistName"]);
                        add(&mut prepared, c, kind, id, values.clone());
                        for t in related(r, "tracks").iter().take(3) {
                            if let Some((_, v)) = prepared.get_mut(&key(t)) {
                                if values["name"].is_string() {
                                    v["albumName"] = values["name"].clone();
                                    if values["artistName"].is_string() {
                                        v["albumArtistName"] = values["artistName"].clone();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for aid in artist_ids(r) {
                let rows = get(&localized, c, "artists", false, &aid);
                if rows.len() == 1
                    && artists.get(&aid).and_then(artist_country).as_deref() == Some(c)
                {
                    add(
                        &mut prepared,
                        c,
                        "artists",
                        &aid,
                        unanimous(&rows, &["name"]),
                    );
                }
            }
        }
        prepared
    }
    pub async fn transform(&self, raw: &[u8]) -> Result<(Vec<u8>, Value)> {
        let start = std::time::Instant::now();
        let generation = self.generation.load(Ordering::Relaxed);
        let errors_before = self.catalog.errors.load(Ordering::Relaxed);
        let mut document: Value = serde_json::from_slice(raw)?;
        let mut entries = Vec::new();
        resources(&document, &mut entries);
        let mut known = self.snapshot();
        let mut needed = HashMap::new();
        for r in entries {
            let hit = known.get(&key(&r));
            if hit.is_none()
                || r["type"] == "songs"
                    && hit.unwrap()["durationInMillis"] != r["attributes"]["durationInMillis"]
            {
                needed.insert(key(&r), r);
            }
        }
        let count = needed.len();
        let mut sorted: Vec<_> = needed.into_iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let result = if sorted.is_empty() {
            Ok(Prepared::new())
        } else {
            tokio::time::timeout(
                Duration::from_secs(6),
                self.resolve(sorted.into_iter().take(512).map(|(_, v)| v).collect()),
            )
            .await
        };
        let timeout = result.is_err();
        if let Ok(p) = result {
            if !p.is_empty() {
                let mut cache = self.cache.lock().unwrap();
                if generation != self.generation.load(Ordering::Relaxed)
                    || !self.enabled.load(Ordering::Relaxed)
                {
                    return Ok((
                        Vec::new(),
                        json!({"edits":0,"cancelled":true,"ms":start.elapsed().as_millis()}),
                    ));
                }
                for (k, (_, v)) in p {
                    known.insert(k.clone(), v.clone());
                    cache.insert(k, (now() + 86400.0, v));
                }
                cache.retain(|_, (e, _)| *e > now());
                while cache.len() > 10000 {
                    let k = cache.keys().next().cloned().unwrap();
                    cache.remove(&k);
                }
                atomic_json(
                    &self.state.join("names-cache.json"),
                    &serde_json::to_value(&*cache)?,
                )?;
            }
        }
        if generation != self.generation.load(Ordering::Relaxed)
            || !self.enabled.load(Ordering::Relaxed)
        {
            return Ok((
                Vec::new(),
                json!({"edits":0,"cancelled":true,"ms":start.elapsed().as_millis()}),
            ));
        }
        let edits = apply(&mut document, &known);
        self.publish(true)?;
        let output = if edits > 0 {
            serde_json::to_vec(&document)?
        } else {
            Vec::new()
        };
        Ok((
            output,
            json!({"edits":edits,"resources":count,"timedOut":timeout,"catalogErrors":self.catalog.errors.load(Ordering::Relaxed).saturating_sub(errors_before),"ms":start.elapsed().as_millis()}),
        ))
    }
}
pub fn apply(v: &mut Value, names: &HashMap<String, Value>) -> usize {
    let mut count = 0;
    if v.is_object() {
        let k = key(v);
        let song = v["type"] == "songs";
        if let Some(n) = names.get(&k) {
            if !song || v["attributes"]["durationInMillis"] == n["durationInMillis"] {
                if let Some(attrs) = v.get_mut("attributes").and_then(Value::as_object_mut) {
                    for f in FIELDS {
                        if let Some(s) = n[f].as_str().filter(|s| !s.is_empty()) {
                            if attrs.get(f).is_some_and(|v| v.is_string() && v != s) {
                                attrs.insert(f.into(), json!(s));
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
        for child in v.as_object_mut().unwrap().values_mut() {
            count += apply(child, names);
        }
    } else if let Some(a) = v.as_array_mut() {
        for child in a {
            count += apply(child, names);
        }
    }
    count
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sparse_artist_uses_own_release_regional_genre() {
        let r = json!({"attributes":{"name":"Example Artist","genreNames":["Anime"],"artistBio":""},"relationships":{"albums":{"data":[{"attributes":{"artistName":"Example Artist","genreNames":["Anime","J-Pop"]}}]}}});
        assert_eq!(artist_country(&r), Some("jp".into()));
    }
    #[test]
    fn related_release_does_not_override_identity_or_biography() {
        let mut r = json!({"attributes":{"name":"Example Artist","genreNames":["Anime"]},"relationships":{"albums":{"data":[{"attributes":{"artistName":"Various Artists","genreNames":["J-Pop"]}}]}}});
        assert_eq!(artist_country(&r), None);
        r["relationships"]["albums"]["data"][0]["attributes"]["artistName"] =
            json!("Example Artist & Collaborator");
        assert_eq!(artist_country(&r), None);
        r["relationships"]["albums"]["data"][0]["attributes"]["artistName"] =
            json!("Example Artist");
        r["attributes"]["artistBio"] = json!("An American singer with a Japanese album release.");
        assert_eq!(artist_country(&r), None);
    }
    #[test]
    fn conflicting_release_origins_remain_unresolved() {
        let r = json!({"attributes":{"name":"Example Artist","genreNames":["Anime"]},"relationships":{"albums":{"data":[{"attributes":{"artistName":"Example Artist","genreNames":["J-Pop"]}},{"attributes":{"artistName":"Example Artist","genreNames":["K-Pop"]}}]}}});
        assert_eq!(artist_country(&r), None);
    }
    #[test]
    fn reference_nodes_do_not_gain_attributes() {
        let mut row = json!({"type":"artists","id":"1","href":"/v1/catalog/us/artists/1"});
        let before = row.clone();
        let names = HashMap::from([("artists:1".into(), json!({"name":"原名"}))]);
        assert_eq!(apply(&mut row, &names), 0);
        assert_eq!(row, before);
    }
    #[tokio::test]
    async fn paused_publish_cannot_restore_names() {
        let dir = std::env::temp_dir().join(format!("mnn-pause-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let r = Resolver::new(dir.clone()).unwrap();
        r.cache.lock().unwrap().insert(
            "songs:1".into(),
            (
                now() + 1000.0,
                json!({"name":"原名","durationInMillis":1000}),
            ),
        );
        assert_eq!(r.publish(true).unwrap(), 1);
        r.set_enabled(false).unwrap();
        assert_eq!(r.publish(true).unwrap(), 0);
        let raw=serde_json::to_vec(&json!({"data":[{"type":"songs","id":"1","attributes":{"name":"English","durationInMillis":1000}}]})).unwrap();
        assert!(r.transform(&raw).await.unwrap().0.is_empty());
        r.set_enabled(true).unwrap();
        assert_eq!(r.publish(true).unwrap(), 1);
        r.clear().unwrap();
        assert_eq!(r.publish(true).unwrap(), 0);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn identity_boundaries() {
        let a = json!({"attributes":{"durationInMillis":1000}});
        assert!(same_recording(&a, &a));
        assert!(!same_recording(
            &a,
            &json!({"attributes":{"durationInMillis":1001}})
        ));
        assert!(!same_recording(
            &a,
            &json!({"attributes":{"durationInMillis":1000,"contentRating":"explicit"}})
        ));
    }
    #[test]
    fn collaboration_is_not_origin() {
        assert_eq!(
            artist_country(
                &json!({"attributes":{"artistBio":"An American singer who collaborated with Korean group BTS."}})
            ),
            None
        );
        assert_eq!(
            artist_country(
                &json!({"attributes":{"artistBio":"A Chinese video game developer and music collective."}})
            ),
            Some("cn".into())
        );
    }
    #[test]
    fn changes_only_names_and_exact_recording() {
        let mut v = json!({"data":[{"id":"1","type":"songs","attributes":{"name":"English","durationInMillis":1000,"url":"https://apple.com","playParams":{"id":"1"}}}]});
        let before = v.clone();
        let names = HashMap::from([(
            "songs:1".into(),
            json!({"name":"原文","durationInMillis":1001}),
        )]);
        assert_eq!(apply(&mut v, &names), 0);
        assert_eq!(v, before);
        let names = HashMap::from([(
            "songs:1".into(),
            json!({"name":"原文","durationInMillis":1000}),
        )]);
        assert_eq!(apply(&mut v, &names), 1);
        v["data"][0]["attributes"]["name"] = json!("English");
        assert_eq!(v, before);
    }
}
