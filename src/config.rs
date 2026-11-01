use crate::cache::{read_app_recents, read_servers};
use crate::models::{RecentEntry, Server};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn config_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/home/hed".to_string());
    let p = Path::new(&home).join(".config").join("daailouivan-surfshark");
    if !p.exists() {
        let _ = fs::create_dir_all(&p);
    }
    p
}

pub fn read_key() -> Option<String> {
    let p = config_dir().join("wg.key");
    if let Ok(s) = fs::read_to_string(p) {
        let k = s.trim().to_string();
        if !k.is_empty() {
            return Some(k);
        }
    }
    // Also check previous location ~/.config/omarchy-surfshark/wg.key as fallback
    let home = env::var("HOME").unwrap_or_else(|_| "/home/hed".to_string());
    let legacy = Path::new(&home)
        .join(".config")
        .join("omarchy-surfshark")
        .join("wg.key");
    if let Ok(s) = fs::read_to_string(legacy) {
        let k = s.trim().to_string();
        if !k.is_empty() {
            let _ = store_key(&k);
            return Some(k);
        }
    }
    None
}

pub fn store_key(key: &str) -> Result<(), std::io::Error> {
    let p = config_dir().join("wg.key");
    fs::write(p, format!("{}\n", key.trim()))
}

pub fn read_current() -> Option<String> {
    let p = config_dir().join("current");
    fs::read_to_string(p).ok().map(|s| s.trim().to_string())
}

pub fn note_current(host: &str) {
    let p = config_dir().join("current");
    let _ = fs::write(p, format!("{}\n", host.trim()));
}

pub fn read_hidden() -> HashSet<String> {
    let p = config_dir().join("hidden.json");
    if let Ok(s) = fs::read_to_string(p) {
        if let Ok(set) = serde_json::from_str::<HashSet<String>>(&s) {
            return set;
        }
    }
    HashSet::new()
}

pub fn store_hidden(hidden: &HashSet<String>) {
    let p = config_dir().join("hidden.json");
    if let Ok(s) = serde_json::to_string_pretty(hidden) {
        let _ = fs::write(p, s);
    }
}

pub fn read_own_recent() -> Vec<RecentEntry> {
    let p = config_dir().join("recent.json");
    if let Ok(s) = fs::read_to_string(p) {
        if let Ok(list) = serde_json::from_str::<Vec<RecentEntry>>(&s) {
            return list;
        }
    }
    Vec::new()
}

pub fn store_own_recent(list: &[RecentEntry]) {
    let p = config_dir().join("recent.json");
    if let Ok(s) = serde_json::to_string_pretty(list) {
        let _ = fs::write(p, s);
    }
}

pub fn push_recent(s: &Server) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let mut list = read_own_recent();
    list.retain(|i| i.connection_name != s.connection_name);
    list.insert(
        0,
        RecentEntry {
            connection_name: s.connection_name.clone(),
            country: s.country.clone(),
            country_code: s.country_code.clone(),
            location: s.location.clone(),
            used_at: now,
        },
    );

    let mut hidden = read_hidden();
    if hidden.remove(&s.connection_name) {
        store_hidden(&hidden);
    }

    if list.len() > 15 {
        list.truncate(15);
    }
    store_own_recent(&list);
}

pub fn get_merged_recent(limit: Option<usize>) -> Vec<RecentEntry> {
    let hidden = read_hidden();
    let mut entries: Vec<RecentEntry> = read_own_recent()
        .into_iter()
        .filter(|e| !hidden.contains(&e.connection_name))
        .collect();

    let existing: HashSet<String> = entries.iter().map(|e| e.connection_name.clone()).collect();
    let servers = read_servers();

    for host in read_app_recents() {
        if !existing.contains(&host) && !hidden.contains(&host) {
            if let Some(srv) = servers.iter().find(|s| s.connection_name == host) {
                entries.push(RecentEntry {
                    connection_name: srv.connection_name.clone(),
                    country: srv.country.clone(),
                    country_code: srv.country_code.clone(),
                    location: srv.location.clone(),
                    used_at: 0.0,
                });
            }
        }
    }

    if let Some(lim) = limit {
        if entries.len() > lim {
            entries.truncate(lim);
        }
    }
    entries
}

pub fn clear_recent() {
    let all: HashSet<String> = get_merged_recent(None)
        .into_iter()
        .map(|e| e.connection_name)
        .collect();
    let mut hidden = read_hidden();
    hidden.extend(all);
    store_hidden(&hidden);

    let p = config_dir().join("recent.json");
    let bak = config_dir().join("recent.json.bak");
    let _ = fs::rename(p, bak);
}

pub fn restore_recent() -> bool {
    let p = config_dir().join("recent.json");
    let bak = config_dir().join("recent.json.bak");
    let restored = fs::rename(bak, p).is_ok();
    let h = config_dir().join("hidden.json");
    let _ = fs::remove_file(h);
    restored
}
