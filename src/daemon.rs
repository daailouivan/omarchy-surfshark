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
