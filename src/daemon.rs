use serde_json::{json, Value};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

pub fn socket_path() -> PathBuf {
    if let Ok(p) = env::var("SURFSHARKD_SOCK") {
        return PathBuf::from(p);
    }
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/run/user/{}/surfsharkd.sock", uid))
}

pub fn is_alive() -> bool {
    let p = socket_path();
    if !p.exists() {
        return false;
    }
    UnixStream::connect(p).is_ok()
}

pub fn rpc_call(method: &str, params: Value) -> Result<Value, String> {
    let p = socket_path();
    let mut stream = UnixStream::connect(&p)
        .map_err(|e| format!("cannot connect to surfsharkd at {}: {}", p.display(), e))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;

    let req = json!({
        "method": method,
        "params": params
    });

    let payload = serde_json::to_string(&req).map_err(|e| e.to_string())? + "\n";
    stream
        .write_all(payload.as_bytes())
        .map_err(|e| format!("write to surfsharkd failed: {}", e))?;
    stream.flush().map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("read from surfsharkd failed: {}", e))?;

    let resp: Value = serde_json::from_str(line.trim())
        .map_err(|e| format!("invalid response from surfsharkd: {}", e))?;

    let resp_type = resp.get("type").and_then(|t| t.as_str()).unwrap_or("");
    if resp_type == "result" {
        Ok(resp.get("value").cloned().unwrap_or(Value::Null))
    } else {
        let err_msg = resp
            .get("value")
            .and_then(|v| v.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error from surfsharkd");
        Err(err_msg.to_string())
    }
}

pub fn get_state() -> Result<Value, String> {
    rpc_call("getState", json!([]))
}

pub fn is_tunnel_up(state: &Value) -> bool {
    // In NetworkManager / surfsharkd:
    // NM_ACTIVE_CONNECTION_STATE_ACTIVATING = 1
    // NM_ACTIVE_CONNECTION_STATE_ACTIVATED  = 2
    for k in ["wg_active_connection_state", "openvpn_active_connection_state"] {
        if let Some(v) = state.get(k).and_then(|x| x.as_i64()) {
            if v == 1 || v == 2 {
                return true;
            }
        }
    }
    false
}

pub fn disconnect_all() -> Result<(), String> {
    let mut last_err = None;
    let mut ok = false;
    for kind in ["wg", "openvpn"] {
        match rpc_call("disconnect", json!([kind])) {
            Ok(_) => ok = true,
            Err(e) => last_err = Some(e),
        }
    }
    if ok {
        Ok(())
    } else if let Some(e) = last_err {
        Err(e)
    } else {
        Ok(())
    }
}

pub fn add_connection(kind: &str, domain: &str, pub_key: &str, priv_key: &str) -> Result<(), String> {
    rpc_call(
        "addConnection",
        json!([kind, {
            "domain": domain,
            "pub": pub_key,
            "priv": priv_key
        }]),
    )?;
    Ok(())
}

pub fn connect_kind(kind: &str) -> Result<(), String> {
    rpc_call("connect", json!([kind]))?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct DbusMenuItem {
    pub id: i32,
    pub label: String,
    pub visible: bool,
    pub toggle_state: Option<i32>,
}

pub fn decode_octal(s: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut oct = String::new();
            while let Some(&next) = chars.peek() {
                if ('0'..='7').contains(&next) && oct.len() < 3 {
                    oct.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if !oct.is_empty() {
                if let Ok(b) = u8::from_str_radix(&oct, 8) {
                    bytes.push(b);
                    continue;
                }
            }
            bytes.push(b'\\');
        } else {
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(encoded.as_bytes());
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn parse_dbusmenu_layout(raw: &str) -> Vec<DbusMenuItem> {
    let mut items = Vec::new();
    let chunks: Vec<&str> = raw.split("(ia{sv}av)").collect();
    if chunks.len() < 3 {
        return items;
    }
    for chunk in &chunks[2..] {
        let trimmed = chunk.trim();
        let id_str = match trimmed.split_whitespace().next() {
            Some(s) => s,
            None => continue,
        };
        let id: i32 = match id_str.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let mut label = String::new();
        if let Some(pos) = chunk.find("\"label\" s \"") {
            let rest = &chunk[pos + 11..];
            if let Some(end) = rest.find('\"') {
                label = decode_octal(&rest[..end]);
            }
        }

        let mut visible = true;
        if chunk.contains("\"visible\" b false") {
            visible = false;
        }

        let mut toggle_state = None;
        if let Some(pos) = chunk.find("\"toggle-state\" i ") {
            let rest = &chunk[pos + 17..];
            let num_str = rest.split_whitespace().next().unwrap_or("");
            if let Ok(v) = num_str.parse::<i32>() {
                toggle_state = Some(v);
            }
        }

        items.push(DbusMenuItem {
            id,
            label,
            visible,
            toggle_state,
        });
    }
    items
}

pub fn find_surfshark_tray_bus() -> Option<String> {
    let output = std::process::Command::new("busctl")
        .args(["--user", "list"])
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut candidates = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[1].chars().all(|c| c.is_ascii_digit()) && parts[2] == "surfshark" {
            candidates.push(parts[0].to_string());
        }
    }

    for bus in candidates {
        let check = std::process::Command::new("busctl")
            .args([
                "--user",
                "call",
                &bus,
                "/com/canonical/dbusmenu",
                "com.canonical.dbusmenu",
                "GetLayout",
                "iias",
                "0",
                "--",
                "-1",
                "0",
            ])
            .output();

        if let Ok(out) = check {
            if out.status.success() {
                return Some(bus);
            }
        }
    }
    None
}

pub fn get_tray_menu(bus: &str) -> Result<Vec<DbusMenuItem>, String> {
    let output = std::process::Command::new("busctl")
        .args([
            "--user",
            "call",
            bus,
            "/com/canonical/dbusmenu",
            "com.canonical.dbusmenu",
            "GetLayout",
            "iias",
            "0",
            "--",
            "-1",
            "0",
        ])
        .output()
        .map_err(|e| format!("failed to call dbusmenu: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_dbusmenu_layout(&text))
}

pub fn trigger_menu_event(bus: &str, item_id: i32) -> Result<(), String> {
    let output = std::process::Command::new("busctl")
        .args([
            "--user",
            "call",
            bus,
            "/com/canonical/dbusmenu",
            "com.canonical.dbusmenu",
            "Event",
            "isvu",
            &item_id.to_string(),
            "clicked",
            "s",
            "",
            "0",
        ])
        .output()
        .map_err(|e| format!("failed to trigger dbusmenu event: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn trigger_app_action(action: &str) -> Result<(), String> {
    let bus = find_surfshark_tray_bus();

    if action == "open" || action == "open_app" {
        if let Some(ref b) = bus {
            if let Ok(menu) = get_tray_menu(b) {
                if let Some(item) = menu.iter().find(|i| i.label.starts_with("Open Surfshark")) {
                    return trigger_menu_event(b, item.id);
                }
            }
        }
        // Fallback spawn app directly
        std::process::Command::new("/opt/Surfshark/surfshark")
            .spawn()
            .map_err(|e| format!("failed to launch Surfshark GUI: {}", e))?;
        return Ok(());
    }

    let b = match bus {
        Some(b) => b,
        None => {
            if action == "disconnect" || action == "off" {
                return disconnect_all();
            }
            return Err("Surfshark desktop client is not running. Please open Surfshark first.".to_string());
        }
    };

    let menu = get_tray_menu(&b)?;

    match action {
        "quick_connect" | "on" => {
            if let Some(item) = menu.iter().find(|i| i.label == "Quick connect" && i.visible) {
                trigger_menu_event(&b, item.id)
            } else if let Some(recent) = menu.iter().find(|i| i.toggle_state.is_some()) {
                trigger_menu_event(&b, recent.id)
            } else {
                Err("No quick connect item available in Surfshark tray menu".to_string())
            }
        }
        "disconnect" | "off" => {
            let mut res = Ok(());
            if let Some(item) = menu.iter().find(|i| i.label == "Disconnect" && i.visible) {
                res = trigger_menu_event(&b, item.id);
            }
            // Always ensure local daemon tear-down as fallback safety
            let _ = disconnect_all();
            res
        }
        "recent" => {
            if let Some(recent) = menu.iter().find(|i| i.toggle_state.is_some()) {
                trigger_menu_event(&b, recent.id)
            } else {
                Err("No recent location found in Surfshark tray menu".to_string())
            }
        }
        other => Err(format!("unknown app action: {}", other)),
    }
}

pub fn verify_network_connectivity(timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        // Fast test against Cloudflare DNS and Surfshark API
        if ureq::get("https://1.1.1.1")
            .timeout(Duration::from_millis(1500))
            .call()
            .is_ok()
        {
            return true;
        }
        if ureq::get("https://api.surfshark.com/v1/server/user")
            .timeout(Duration::from_millis(1500))
            .call()
            .is_ok()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}
