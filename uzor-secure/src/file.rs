//! Plain files with mode 0600 in a 0700 directory: the fallback for platforms with
//! no secure store reachable without heavy dependencies (Linux without a Secret
//! Service client here: the available crates pull a D-Bus stack and an async
//! runtime). The bytes are NOT encrypted; only the file mode protects them. So it
//! is used only for items whose [`Policy::file_fallback`] opts in, and never for
//! gated items (it has no way to ask the user, so those fail closed).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::{check_name, next_store_id, Error, PasskeyAssertion, PasskeyCreated, PasskeyUser, PlatformCaps, Policy, Secret, SecureStore, UnlockHandle};

/// 0600 files under one directory.
pub struct FileStore {
    dir: PathBuf,
    #[allow(dead_code)]
    id: u64,
}

impl FileStore {
    /// A store in `dir` (made 0700 on the first write).
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        FileStore { dir: dir.into(), id: next_store_id() }
    }

    /// `$XDG_DATA_HOME/uzor-secure/<app>` (or `~/.local/share/...`).
    pub fn default_dir(app: &str) -> PathBuf {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("uzor-secure").join(app)
    }

    /// Where the items live.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.item"))
    }

    fn ensure_dir(&self) -> Result<(), Error> {
        fs::create_dir_all(&self.dir).map_err(|e| Error::Backend(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.dir, fs::Permissions::from_mode(0o700)).map_err(|e| Error::Backend(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(unix)]
fn create_private(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)
}

#[cfg(not(unix))]
fn create_private(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().write(true).create_new(true).open(path)
}

impl SecureStore for FileStore {
    fn caps(&self) -> PlatformCaps {
        PlatformCaps { file_fallback: cfg!(unix), ..PlatformCaps::default() }
    }

    fn secure_put(&self, name: &str, bytes: &[u8], policy: Policy) -> Result<(), Error> {
        check_name(name)?;
        if !cfg!(unix) {
            return Err(Error::Unsupported("file fallback needs unix file modes"));
        }
        if policy.gated() {
            return Err(Error::Unsupported("no biometry or device credential here: a gated item cannot be kept in a file"));
        }
        if !policy.file_fallback {
            return Err(Error::Unsupported("no secure store on this platform; set Policy::file_fallback to allow a 0600 file"));
        }
        self.ensure_dir()?;
        // Write a fresh 0600 file, then rename over the old one (atomic replace).
        let tmp = self.dir.join(format!(".{name}.{:016x}.tmp", u64::from_le_bytes(crate::random(8)?.try_into().unwrap_or([0; 8]))));
        let mut f = create_private(&tmp).map_err(|e| Error::Backend(e.to_string()))?;
        let res = f.write_all(bytes).and_then(|_| f.sync_all()).and_then(|_| fs::rename(&tmp, self.path(name)));
        if let Err(e) = res {
            let _ = fs::remove_file(&tmp);
            return Err(Error::Backend(e.to_string()));
        }
        Ok(())
    }

    fn secure_get(&self, name: &str, _unlock: Option<&UnlockHandle>) -> Result<Secret, Error> {
        check_name(name)?;
        match fs::read(self.path(name)) {
            Ok(b) => Ok(Secret::new(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::NotFound),
            Err(e) => Err(Error::Backend(e.to_string())),
        }
    }

    fn secure_delete(&self, name: &str) -> Result<(), Error> {
        check_name(name)?;
        match fs::remove_file(self.path(name)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Error::NotFound),
            Err(e) => Err(Error::Backend(e.to_string())),
        }
    }

    fn unlock(&self, _reason: &str, _ttl: Duration) -> Result<UnlockHandle, Error> {
        Err(Error::Unsupported("no biometry or device credential prompt in the file store"))
    }

    fn passkey_create(&self, _: &str, _: &PasskeyUser, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyCreated, Error> {
        Err(Error::Unsupported("no platform authenticator in the file store (desktop passkeys go through CTAP2 / the browser)"))
    }

    fn passkey_assert(&self, _: &str, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyAssertion, Error> {
        Err(Error::Unsupported("no platform authenticator in the file store (desktop passkeys go through CTAP2 / the browser)"))
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("uzor-secure-test-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn only_opted_in_items_and_files_are_private() {
        let dir = tmpdir("optin");
        let s = FileStore::new(&dir);
        assert!(matches!(s.secure_put("link", b"x", Policy::default()), Err(Error::Unsupported(_))), "refused without opt-in");
        assert!(!dir.exists(), "nothing written");
        let p = Policy { file_fallback: true, ..Policy::default() };
        s.secure_put("link", b"hello", p).unwrap();
        s.secure_put("link", b"hello2", p).unwrap();
        assert_eq!(s.secure_get("link", None).unwrap().expose(), b"hello2");
        let mode = fs::metadata(dir.join("link.item")).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "no temp files left");
        s.secure_delete("link").unwrap();
        assert_eq!(s.secure_get("link", None), Err(Error::NotFound));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn gated_items_are_refused() {
        let dir = tmpdir("gated");
        let s = FileStore::new(&dir);
        let p = Policy { biometry: true, file_fallback: true, ..Policy::default() };
        assert!(matches!(s.secure_put("ssh.root", b"x", p), Err(Error::Unsupported(_))));
        assert!(matches!(s.unlock("x", Duration::from_secs(1)), Err(Error::Unsupported(_))));
        assert!(!dir.exists());
    }
}
