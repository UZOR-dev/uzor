//! The discovery file: where a tool finds the port and token.
//!
//! `$XDG_RUNTIME_DIR/<app>/dev-<pid>.json` (falling back to
//! `~/.config/<app>/dev-<pid>.json`), directory `0700`, file `0600`,
//! removed when the [`Discovery`] guard drops.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

/// What the file holds.
#[derive(Clone, PartialEq, Eq)]
pub struct DiscoveryInfo {
    /// App name.
    pub app: String,
    /// Serving process.
    pub pid: u32,
    /// Loopback port.
    pub port: u16,
    /// Bearer token (hex). Never logged.
    pub token: String,
    /// Protocol revision.
    pub protocol: u32,
}

impl std::fmt::Debug for DiscoveryInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryInfo")
            .field("app", &self.app)
            .field("pid", &self.pid)
            .field("port", &self.port)
            .field("token", &"..")
            .field("protocol", &self.protocol)
            .finish()
    }
}

impl DiscoveryInfo {
    fn to_json(&self) -> String {
        json!({"app": self.app, "pid": self.pid, "port": self.port, "token": self.token, "protocol": self.protocol}).to_string()
    }

    /// Parse a discovery file's text.
    pub fn parse(text: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(text).ok()?;
        Some(Self {
            app: v.get("app")?.as_str()?.to_string(),
            pid: u32::try_from(v.get("pid")?.as_u64()?).ok()?,
            port: u16::try_from(v.get("port")?.as_u64()?).ok()?,
            token: v.get("token")?.as_str()?.to_string(),
            protocol: u32::try_from(v.get("protocol")?.as_u64()?).ok()?,
        })
    }
}

/// The directory discovery files for `app_dir` live in.
pub fn dir(app_dir: &str) -> Option<PathBuf> {
    if let Some(rt) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|p| p.is_dir()) {
        return Some(rt.join(app_dir));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".config").join(app_dir))
}

/// A published discovery file; removed on drop.
#[derive(Debug)]
pub struct Discovery {
    path: PathBuf,
}

impl Discovery {
    /// Write `info` to `dev-<pid>.json` under [`dir`]`(app_dir)`.
    pub fn publish(app_dir: &str, info: &DiscoveryInfo) -> std::io::Result<Self> {
        let d = dir(app_dir).ok_or_else(|| std::io::Error::other("no XDG_RUNTIME_DIR or HOME for the discovery file"))?;
        Self::publish_in(&d, info)
    }

    /// Write `info` into the directory `d` (created `0700`).
    pub fn publish_in(d: &Path, info: &DiscoveryInfo) -> std::io::Result<Self> {
        fs::create_dir_all(d)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(d, fs::Permissions::from_mode(0o700))?;
        }
        let path = d.join(format!("dev-{}.json", info.pid));
        let _ = fs::remove_file(&path);
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&path)?;
        f.write_all(info.to_json().as_bytes())?;
        f.sync_all()?;
        Ok(Self { path })
    }

    /// The file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Is `pid` a running process? (Linux reads `/proc`; elsewhere assumed yes.)
pub fn alive(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        Path::new(&format!("/proc/{pid}")).exists()
    } else {
        true
    }
}

/// Every live endpoint under `d`, newest file first.
pub fn find_in(d: &Path) -> Vec<(PathBuf, DiscoveryInfo)> {
    let mut out: Vec<(std::time::SystemTime, PathBuf, DiscoveryInfo)> = Vec::new();
    let Ok(rd) = fs::read_dir(d) else { return Vec::new() };
    for e in rd.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if !(name.starts_with("dev-") && name.ends_with(".json")) {
            continue;
        }
        let Some(info) = fs::read_to_string(&p).ok().and_then(|t| DiscoveryInfo::parse(&t)) else { continue };
        if !alive(info.pid) {
            continue;
        }
        let t = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        out.push((t, p, info));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0));
    out.into_iter().map(|(_, p, i)| (p, i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_find_and_remove() {
        let d = std::env::temp_dir().join(format!("uzor-devtools-test-{}", std::process::id()));
        let info = DiscoveryInfo { app: "t".into(), pid: std::process::id(), port: 4242, token: "ab".repeat(32), protocol: 1 };
        let g = Discovery::publish_in(&d, &info).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(g.path()).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(fs::metadata(&d).unwrap().permissions().mode() & 0o777, 0o700);
        }
        let found = find_in(&d);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1, info);
        assert!(!format!("{:?}", found[0].1).contains(&info.token));
        let p = g.path().to_path_buf();
        drop(g);
        assert!(!p.exists());
        let _ = fs::remove_dir(&d);
    }
}
