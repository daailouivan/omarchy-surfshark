mod cache;
mod config;
mod daemon;
mod geo;
mod models;
mod picker;

use cache::{active_nm_endpoint, key_from_nm, read_servers, read_suggested};
use config::{
    clear_recent, get_merged_recent, note_current, push_recent, read_current, read_key,
    restore_recent, store_key,
};
use daemon::{
    add_connection, connect_kind, disconnect_all, get_state, is_alive, is_tunnel_up,
    trigger_app_action, verify_network_connectivity,
};
use geo::fetch_geo;
use models::{LocationItem, Server, WidgetStatus};
use picker::{pick_fastest, pick_nearest, pick_random, pick_same, pick_usa};
use std::env;
use std::process::exit;

fn ensure_key() -> Option<String> {
    if let Some(k) = read_key() {
        return Some(k);
    }
    if let Some(k) = key_from_nm() {
        let _ = store_key(&k);
        return Some(k);
    }
    None
}

fn cmd_status() {
    let daemon_up = is_alive();
    let state = if daemon_up {
        get_state().unwrap_or_default()
    } else {
        serde_json::Value::Null
    };

    let connected = is_tunnel_up(&state);
    let key_avail = ensure_key().is_some();
    let servers = read_servers();
    let has_servers = !servers.is_empty();

    let endpoint = if connected {
        active_nm_endpoint().or_else(read_current)
    } else {
        None
    };

    let active_server = endpoint
        .as_ref()
        .and_then(|ep| servers.iter().find(|s| &s.connection_name == ep));

    let geo = fetch_geo();

    let status = WidgetStatus {
        daemon: daemon_up,
        configured: key_avail && has_servers,
        connected,
        endpoint: endpoint.clone(),
        location: if connected {
            active_server.map(|s| s.location.clone()).unwrap_or_default()
        } else {
            String::new()
        },
        country: if connected {
            active_server.map(|s| s.country.clone()).unwrap_or_default()
        } else {
            String::new()
        },
        country_code: if connected {
            active_server.map(|s| s.country_code.clone()).unwrap_or_default()
        } else {
            String::new()
        },
        load: active_server.and_then(|s| s.load),
        ip: geo.ip,
        secured: geo.secured,
        my_country: geo.country,
        my_country_code: geo.country_code,
        my_city: geo.city,
        recent: get_merged_recent(Some(5)),
    };

    println!("{}", serde_json::to_string_pretty(&status).unwrap());
}

fn cmd_locations() {
    let servers = read_servers();
    let items: Vec<LocationItem> = servers
        .into_iter()
        .map(|s| LocationItem {
            connection_name: s.connection_name,
            country: s.country,
            country_code: s.country_code,
            location: s.location,
            load: s.load,
        })
        .collect();
    println!("{}", serde_json::to_string(&items).unwrap());
}

fn cmd_list(filter: Option<&str>) {
    let servers = read_servers();
    let f = filter.map(|s| s.to_lowercase());
    for s in servers {
        if let Some(ref pat) = f {
            let hay = format!("{} {} {}", s.connection_name, s.country, s.location).to_lowercase();
            if !hay.contains(pat) {
                continue;
            }
        }
        println!(
            "{:<35} {:<24} {:<18} load={}",
            s.connection_name,
            s.country,
            s.location,
            s.load.unwrap_or(0)
        );
    }
}

fn do_connect(s: &Server) {
    let pub_key = match &s.pub_key {
        Some(k) => k,
        None => {
            eprintln!("server {} has no WireGuard key in cache", s.connection_name);
            exit(5);
        }
    };

    let priv_key = match ensure_key() {
        Some(k) => k,
        None => {
            eprintln!("No WireGuard key yet. Connect once through Surfshark app, then run: omarchy-surfshark key-import");
            let _ = trigger_app_action("open");
            exit(7);
        }
    };

    if let Err(e) = add_connection("wg", &s.connection_name, pub_key, &priv_key) {
        eprintln!("addConnection failed: {}", e);
        exit(4);
    }

    if let Err(e) = connect_kind("wg") {
        eprintln!("connect failed: {}", e);
        exit(4);
    }

    // Fail-Safe Watchdog:
    // If WireGuard brought default routing up but handshake failed (e.g. PQ preshared key missing),
    // immediately tear down connection to prevent freezing network connectivity.
    if !verify_network_connectivity(std::time::Duration::from_millis(3200)) {
        let _ = disconnect_all();
        eprintln!("Handshake failed (Surfshark Post-Quantum key authorization required). Reverted connection to protect network.");
        let _ = trigger_app_action("open");
        exit(8);
    }

    push_recent(s);
    note_current(&s.connection_name);
    println!("{} · {}", s.location, s.country);
}

fn cmd_on() {
    // If the official Surfshark desktop client is running, trigger connection via D-Bus tray
    // so the client authenticates, requests PQS preshared keys, and establishes tunnel safely.
    if trigger_app_action("quick_connect").is_ok() {
        println!("Connecting via Surfshark...");
        return;
    }

    let recents = get_merged_recent(Some(1));
    let servers = read_servers();
    if let Some(first) = recents.first() {
        if let Some(s) = servers.iter().find(|s| s.connection_name == first.connection_name) {
            do_connect(s);
            return;
        }
    }
    let geo = fetch_geo();
    let suggested = read_suggested();
    if let Some(s) = pick_fastest(&servers, &geo, suggested.as_ref()) {
        do_connect(s);
    } else {
        eprintln!("no available server found");
        exit(5);
    }
}

fn cmd_fastest() {
    let servers = read_servers();
    let geo = fetch_geo();
    let suggested = read_suggested();
    if let Some(s) = pick_fastest(&servers, &geo, suggested.as_ref()) {
        do_connect(s);
    } else {
        eprintln!("no available server found");
        exit(5);
    }
}

fn cmd_nearest() {
    let servers = read_servers();
    let geo = fetch_geo();
    if let Some(s) = pick_nearest(&servers, &geo, false, true) {
        do_connect(s);
    } else {
        eprintln!("no available server found");
        exit(5);
    }
}

fn cmd_same() {
    let servers = read_servers();
    let geo = fetch_geo();
    if let Some(s) = pick_same(&servers, &geo) {
        do_connect(s);
    } else {
        eprintln!("no available server found in current country");
        exit(5);
    }
}

fn cmd_usa(preferred: Option<&str>) {
    let servers = read_servers();
    if let Some(s) = pick_usa(&servers, preferred) {
        do_connect(s);
    } else {
        eprintln!("no available US server found");
        exit(5);
    }
}

fn cmd_random() {
    let servers = read_servers();
    let geo = fetch_geo();
    let cur = read_current();
    if let Some(s) = pick_random(&servers, &geo, cur.as_deref()) {
        do_connect(s);
    } else {
        eprintln!("no available random server found");
        exit(5);
    }
}

fn cmd_connect(host: &str) {
    let servers = read_servers();
    if let Some(s) = servers.iter().find(|s| s.connection_name == host) {
        do_connect(s);
    } else {
        eprintln!("server not found: {}", host);
        exit(5);
    }
}

fn cmd_off() {
    let _ = trigger_app_action("disconnect");
    if let Err(e) = disconnect_all() {
        eprintln!("disconnect error: {}", e);
        exit(4);
    }
    println!("disconnected");
}

fn cmd_open() {
    if let Err(e) = trigger_app_action("open") {
        eprintln!("open error: {}", e);
        exit(1);
    }
    println!("opened Surfshark client");
}

fn cmd_recent(n: Option<usize>) {
    let recents = get_merged_recent(n.or(Some(10)));
    for r in recents {
        println!("{:<32} {:<20} {}", r.connection_name, r.country, r.location);
    }
}

fn cmd_recent_clear() {
    clear_recent();
    println!("recent list cleared (undo with: omarchy-surfshark recent-restore)");
}

fn cmd_recent_restore() {
    if restore_recent() {
        println!("recent list restored");
    } else {
        println!("nothing set aside; un-hid entries");
    }
}

fn cmd_key_import() {
    if let Some(k) = key_from_nm() {
        if store_key(&k).is_ok() {
            println!("WireGuard key successfully imported and stored");
            return;
        }
    }
    eprintln!("could not find WireGuard private key in NetworkManager");
    exit(1);
}

fn cmd_state() {
    match get_state() {
        Ok(s) => println!("{}", serde_json::to_string_pretty(&s).unwrap()),
        Err(e) => {
            eprintln!("getState error: {}", e);
            exit(1);
        }
    }
}

fn print_help() {
    println!(
        "omarchy-surfshark (Rust) - Backend for the Surfshark Omarchy bar widget

Commands:
  status            JSON state (consumed by the widget)
  on                connect to the last used location, else fastest
  off               tear the tunnel down
  fastest           connect to fastest location
  nearest           connect to nearest abroad location
  same              connect to server in same country
  usa [host]        connect to US server
  random            connect to random country
  connect <host>    connect to a specific server by hostname
  locations         JSON list of all locations (for search)
  list [filter]     plain-text list of servers
  recent [n]        recently used locations
  recent-clear      clear the recent list
  recent-restore    restore recent list
  key-import        import WireGuard key from NetworkManager
  state             raw getState() JSON
  open              open / focus the official Surfshark GUI
"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        cmd_status();
        return;
    }

    match args[1].as_str() {
        "status" => cmd_status(),
        "locations" => cmd_locations(),
        "list" => cmd_list(args.get(2).map(|s| s.as_str())),
        "fastest" => cmd_fastest(),
        "nearest" => cmd_nearest(),
        "same" => cmd_same(),
        "usa" => cmd_usa(args.get(2).map(|s| s.as_str())),
        "random" => cmd_random(),
        "connect" => {
            if let Some(host) = args.get(2) {
                cmd_connect(host);
            } else {
                eprintln!("usage: omarchy-surfshark connect <connectionName>");
                exit(1);
            }
        }
        "on" | "quick-connect" => cmd_on(),
        "off" | "disconnect" => cmd_off(),
        "open" | "gui" => cmd_open(),
        "recent" => {
            let n = args.get(2).and_then(|s| s.parse::<usize>().ok());
            cmd_recent(n);
        }
        "recent-clear" => cmd_recent_clear(),
        "recent-restore" => cmd_recent_restore(),
        "key-import" => cmd_key_import(),
        "state" => cmd_state(),
        "-h" | "--help" | "help" => print_help(),
        other => {
            eprintln!("unknown command: {}", other);
            print_help();
            exit(1);
        }
    }
}
