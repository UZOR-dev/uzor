//! `uzor-dev` — call a running app's dev-control RPC.
//!
//! ```text
//! uzor-dev [--app DIR] [--pid N] [--file PATH] <method> [ARGS...] [--out FILE]
//!
//!   uzor-dev ping
//!   uzor-dev dump-tree [widgets=true]
//!   uzor-dev click '#lh-bind'            (a selector, or x=10 y=20)
//!   uzor-dev type 'hello' [selector='#field']
//!   uzor-dev key Enter
//!   uzor-dev scroll '#list' dy=-120
//!   uzor-dev screenshot --out shot.png
//!   uzor-dev state
//!   uzor-dev events [since=0] [wait_ms=5000]
//!   uzor-dev raw '{"method":"ping"}'
//! ```
//!
//! ARGS are `key=value` pairs (a value that parses as JSON is taken as
//! JSON), or one JSON object, or for click/hover/focus/scroll a bare
//! selector, for type the text, for key the chord. The discovery file is
//! found under `$XDG_RUNTIME_DIR/<app>/` (else `~/.config/<app>/`); `--app`
//! defaults to `hatchery-hq`, or `UZOR_DEV_APP`.

use std::path::PathBuf;
use std::process::ExitCode;

use base64::Engine as _;
use serde_json::{json, Map, Value};
use uzor_devtools::discovery::{self, DiscoveryInfo};
use uzor_devtools::server::http_call;

fn usage() -> ExitCode {
    eprintln!("usage: uzor-dev [--app DIR] [--pid N] [--file PATH] <method> [ARGS...] [--out FILE]");
    eprintln!("methods: {} events raw", uzor_devtools::protocol::METHODS.join(" "));
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut app = std::env::var("UZOR_DEV_APP").unwrap_or_else(|_| "hatchery-hq".into());
    let mut pid: Option<u32> = None;
    let mut file: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = std::mem::take(&mut args[i]);
        let mut val = || {
            i += 1;
            args.get_mut(i).map(std::mem::take)
        };
        match a.as_str() {
            "--app" => app = val().unwrap_or_default(),
            "--pid" => pid = val().and_then(|v| v.parse().ok()),
            "--file" => file = val().map(PathBuf::from),
            "--out" | "-o" => out = val().map(PathBuf::from),
            "-h" | "--help" => return usage(),
            _ => rest.push(a),
        }
        i += 1;
    }
    let Some((method, params)) = rest.split_first() else { return usage() };

    let info = match locate(&app, pid, file) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("uzor-dev: {e}");
            return ExitCode::from(3);
        }
    };
    let auth = format!("Bearer {}", info.token);
    let host = format!("127.0.0.1:{}", info.port);
    let headers = [("Host", host.as_str()), ("Authorization", auth.as_str()), ("Content-Type", "application/json")];

    let (status, body) = if method == "events" {
        let p = match build_params(method, params) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("uzor-dev: {e}");
                return ExitCode::from(2);
            }
        };
        let since = p.get("since").and_then(Value::as_u64).unwrap_or(0);
        let wait = p.get("wait_ms").and_then(Value::as_u64).unwrap_or(0);
        http_call(info.port, "GET", &format!("/events?since={since}&wait_ms={wait}"), &headers, "")
    } else {
        let req = if method == "raw" {
            params.first().cloned().unwrap_or_default()
        } else {
            match build_params(method, params) {
                Ok(p) => json!({"id": 1, "method": method, "params": p}).to_string(),
                Err(e) => {
                    eprintln!("uzor-dev: {e}");
                    return ExitCode::from(2);
                }
            }
        };
        http_call(info.port, "POST", "/rpc", &headers, &req)
    }
    .unwrap_or_else(|e| (0, json!({"ok": false, "error": {"code": "connect_failed", "message": e.to_string()}}).to_string()));

    let mut v: Value = serde_json::from_str(&body).unwrap_or(Value::String(body.clone()));
    if let (Some(path), Some(b64)) = (out.as_ref(), v.pointer("/result/png_base64").and_then(Value::as_str)) {
        match base64::engine::general_purpose::STANDARD.decode(b64) {
            Ok(png) => {
                if let Err(e) = std::fs::write(path, png) {
                    eprintln!("uzor-dev: cannot write {}: {e}", path.display());
                    return ExitCode::from(1);
                }
                v["result"]["png_base64"] = json!(format!("<written to {}>", path.display()));
            }
            Err(e) => {
                eprintln!("uzor-dev: bad png in reply: {e}");
                return ExitCode::from(1);
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
    let ok = status == 200 && v.get("ok").and_then(Value::as_bool).unwrap_or(v.is_array());
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn locate(app: &str, pid: Option<u32>, file: Option<PathBuf>) -> Result<DiscoveryInfo, String> {
    if let Some(f) = file {
        let text = std::fs::read_to_string(&f).map_err(|e| format!("cannot read {}: {e}", f.display()))?;
        return DiscoveryInfo::parse(&text).ok_or_else(|| format!("{} is not a discovery file", f.display()));
    }
    let d = discovery::dir(app).ok_or("no XDG_RUNTIME_DIR or HOME")?;
    let found = discovery::find_in(&d);
    let pick = match pid {
        Some(p) => found.into_iter().find(|(_, i)| i.pid == p),
        None => found.into_iter().next(),
    };
    pick.map(|(_, i)| i).ok_or_else(|| format!("no running app with dev control under {} (started with --dev?)", d.display()))
}

fn build_params(method: &str, args: &[String]) -> Result<Value, String> {
    let mut m = Map::new();
    for (n, a) in args.iter().enumerate() {
        if n == 0 && a.trim_start().starts_with('{') {
            let v: Value = serde_json::from_str(a).map_err(|e| format!("params JSON: {e}"))?;
            match v {
                Value::Object(o) => m.extend(o),
                _ => return Err("params JSON must be an object".into()),
            }
            continue;
        }
        let kv = a.split_once('=').filter(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        match kv {
            Some((k, v)) => {
                let val = serde_json::from_str(v).unwrap_or_else(|_| Value::String(v.to_string()));
                m.insert(k.to_string(), val);
            }
            None if n == 0 => {
                let key = match method {
                    "click" | "hover" | "focus" | "scroll" => "selector",
                    "type" => "text",
                    "key" => "key",
                    _ => return Err(format!("unexpected argument '{a}' (use key=value)")),
                };
                m.insert(key.into(), Value::String(a.clone()));
            }
            None => return Err(format!("unexpected argument '{a}' (use key=value)")),
        }
    }
    Ok(Value::Object(m))
}
