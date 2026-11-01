use crate::models::Server;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn surfshark_config_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| "/home/hed".to_string());
    Path::new(&home).join(".config").join("Surfshark")
}

pub fn read_app_cache() -> Option<Value> {
    let p = surfshark_config_dir().join("cache.json");
    let content = fs::read_to_string(p).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn read_servers() -> Vec<Server> {
    let cache = match read_app_cache() {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut best_val: Option<Value> = None;
    let mut best_updated: u64 = 0;

    if let Some(obj) = cache.as_object() {
        for (k, v) in obj {
            if k.ends_with("/v5/server/clusters/all") {
                let updated = v.get("updatedAt").and_then(|u| u.as_u64()).unwrap_or(0);
                if updated >= best_updated {
                    best_updated = updated;
                    best_val = v.get("value").cloned();
                }
            }
        }
    }

    if let Some(val) = best_val {
        if let Ok(servers) = serde_json::from_value::<Vec<Server>>(val) {
            return servers;
        }
    }

    Vec::new()
}

pub fn read_suggested() -> Option<Server> {
    let cache = read_app_cache()?;
    let mut best_val: Option<Value> = None;
    let mut best_updated: u64 = 0;

    if let Some(obj) = cache.as_object() {
        for (k, v) in obj {
            if k.ends_with("/v5/server/suggest?limit=1") {
                let updated = v.get("updatedAt").and_then(|u| u.as_u64()).unwrap_or(0);
                if updated >= best_updated {
                    best_updated = updated;
                    best_val = v.get("value").cloned();
                }
            }
        }
    }

    if let Some(val) = best_val {
        if let Some(arr) = val.as_array() {
            if let Some(first) = arr.first() {
                return serde_json::from_value::<Server>(first.clone()).ok();
            }
        }
    }

    None
}

pub fn read_app_recents() -> Vec<String> {
    let dir = surfshark_config_dir();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut best_file: Option<PathBuf> = None;
    let mut best_mtime = std::time::SystemTime::UNIX_EPOCH;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let s = name.to_string_lossy();
        if s.starts_with("settings-") && s.ends_with(".json") && s != "settings-undefined.json" {
            if let Ok(meta) = entry.metadata() {
                if let Ok(mtime) = meta.modified() {
                    if mtime > best_mtime {
                        best_mtime = mtime;
                        best_file = Some(entry.path());
                    }
                }
            }
        }
    }

    let file_path = match best_file {
        Some(f) => f,
        None => return Vec::new(),
    };

    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let val: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    if let Some(arr) = val.get("recents").and_then(|r| r.as_array()) {
        for item in arr {
            let host = item
                .get("connectionName")
                .or_else(|| item.get("id"))
                .or_else(|| item.get("target"))
                .and_then(|h| h.as_str());
            if let Some(h) = host {
                if h.contains('.') {
                    out.push(h.to_string());
                }
            }
        }
    }

    out
}

pub fn active_nm_endpoint() -> Option<String> {
    // Check nmcli connection show surfshark_wg to see active peer endpoint
    let out = Command::new("nmcli")
        .args(["-g", "wireguard.peers", "connection", "show", "surfshark_wg"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    // line contains "endpoint=mk-skp.prod.surfshark.com:51820"
    for part in s.split_whitespace() {
        if let Some(ep) = part.strip_prefix("endpoint=") {
            let host = ep.split(':').next().unwrap_or(ep);
            if !host.is_empty() {
                return Some(host.to_string());
            }
        }
    }
    None
}

pub fn key_from_nm() -> Option<String> {
    let out = Command::new("nmcli")
        .args(["-t", "-f", "NAME,TYPE", "connection", "show"])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    let mut names: Vec<String> = s
        .lines()
        .filter(|ln| ln.ends_with(":wireguard"))
        .filter_map(|ln| ln.rsplit_once(':').map(|(n, _)| n.to_string()))
        .collect();

    names.sort_by_key(|n| if n.to_lowercase().contains("surfshark") { 0 } else { 1 });

    for name in names {
        if let Ok(p) = Command::new("nmcli")
            .args(["-s", "-g", "wireguard.private-key", "connection", "show", &name])
            .output()
        {
            let k = String::from_utf8_lossy(&p.stdout).trim().to_string();
            if !k.is_empty() && k != "--" {
                return Some(k);
            }
        }
    }
    None
}
