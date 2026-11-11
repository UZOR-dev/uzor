//! An in-memory store: tests and ephemeral sessions. Applies the same gate as the
//! platform stores; the unlock prompt is simulated by an approver.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::{check_name, next_store_id, Error, PasskeyAssertion, PasskeyCreated, PasskeyUser, PlatformCaps, Policy, Secret, SecureStore, UnlockHandle};

type Approver = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// Items in memory; nothing persists.
pub struct MemoryStore {
    id: u64,
    items: Mutex<HashMap<String, (Secret, Policy)>>,
    approve: Approver,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    /// A store whose unlock prompts are always approved.
    pub fn new() -> Self {
        Self::with_approver(|_| true)
    }

    /// A store whose unlock prompt answers with `approve(reason)` (a user who
    /// cancels: `|_| false`).
    pub fn with_approver(approve: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        MemoryStore { id: next_store_id(), items: Mutex::new(HashMap::new()), approve: Box::new(approve) }
    }

    fn items(&self) -> std::sync::MutexGuard<'_, HashMap<String, (Secret, Policy)>> {
        self.items.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl SecureStore for MemoryStore {
    fn caps(&self) -> PlatformCaps {
        PlatformCaps::default()
    }

    fn secure_put(&self, name: &str, bytes: &[u8], policy: Policy) -> Result<(), Error> {
        check_name(name)?;
        self.items().insert(name.to_string(), (Secret::new(bytes.to_vec()), policy));
        Ok(())
    }

    fn secure_get(&self, name: &str, unlock: Option<&UnlockHandle>) -> Result<Secret, Error> {
        check_name(name)?;
        let items = self.items();
        let (secret, policy) = items.get(name).ok_or(Error::NotFound)?;
        if policy.gated() {
            unlock.ok_or(Error::Locked)?.check(self.id)?;
        }
        Ok(secret.clone())
    }

    fn secure_delete(&self, name: &str) -> Result<(), Error> {
        check_name(name)?;
        self.items().remove(name).map(|_| ()).ok_or(Error::NotFound)
    }

    fn unlock(&self, reason: &str, ttl: Duration) -> Result<UnlockHandle, Error> {
        if reason.trim().is_empty() {
            return Err(Error::Invalid("the unlock prompt needs a reason"));
        }
        if (self.approve)(reason) {
            Ok(UnlockHandle::issue(self.id, ttl))
        } else {
            Err(Error::Denied)
        }
    }

    fn passkey_create(&self, _: &str, _: &PasskeyUser, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyCreated, Error> {
        Err(Error::Unsupported("passkeys need a platform authenticator; the memory store has none"))
    }

    fn passkey_assert(&self, _: &str, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyAssertion, Error> {
        Err(Error::Unsupported("passkeys need a platform authenticator; the memory store has none"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gated() -> Policy {
        Policy { biometry: true, this_device_only: true, ..Policy::default() }
    }

    #[test]
    fn put_get_replace_delete() {
        let s = MemoryStore::new();
        s.secure_put("node.key", b"one", Policy::default()).unwrap();
        assert_eq!(s.secure_get("node.key", None).unwrap().expose(), b"one");
        s.secure_put("node.key", b"two", Policy::default()).unwrap();
        assert_eq!(s.secure_get("node.key", None).unwrap().expose(), b"two");
        s.secure_delete("node.key").unwrap();
        assert_eq!(s.secure_get("node.key", None), Err(Error::NotFound));
        assert_eq!(s.secure_delete("node.key"), Err(Error::NotFound));
        assert!(matches!(s.secure_put("../etc", b"x", Policy::default()), Err(Error::Invalid(_))));
    }

    #[test]
    fn gated_items_fail_closed_without_a_valid_handle() {
        let s = MemoryStore::new();
        s.secure_put("ssh.root", b"secret", gated()).unwrap();
        assert_eq!(s.secure_get("ssh.root", None), Err(Error::Locked), "no handle");
        let other = MemoryStore::new();
        let foreign = other.unlock("other app", Duration::from_secs(60)).unwrap();
        assert_eq!(s.secure_get("ssh.root", Some(&foreign)), Err(Error::Locked), "another store's handle");
        let expired = s.unlock("read the key", Duration::ZERO).unwrap();
        assert_eq!(s.secure_get("ssh.root", Some(&expired)), Err(Error::Expired));
        let h = s.unlock("read the key", Duration::from_secs(60)).unwrap();
        assert_eq!(s.secure_get("ssh.root", Some(&h)).unwrap().expose(), b"secret");
        // Device credential gates too.
        s.secure_put("pin.item", b"p", Policy { device_credential: true, ..Policy::default() }).unwrap();
        assert_eq!(s.secure_get("pin.item", None), Err(Error::Locked));
    }

    #[test]
    fn a_denied_unlock_gives_no_handle() {
        let s = MemoryStore::with_approver(|_| false);
        s.secure_put("ssh.root", b"secret", gated()).unwrap();
        assert_eq!(s.unlock("read the key", Duration::from_secs(60)), Err(Error::Denied));
        assert!(matches!(s.unlock("  ", Duration::from_secs(60)), Err(Error::Invalid(_))));
        assert_eq!(s.secure_get("ssh.root", None), Err(Error::Locked));
    }

    #[test]
    fn ungated_items_ignore_handles() {
        let s = MemoryStore::new();
        s.secure_put("link", b"l", Policy::default()).unwrap();
        let h = s.unlock("x", Duration::ZERO).unwrap();
        assert_eq!(s.secure_get("link", Some(&h)).unwrap().expose(), b"l");
    }

    #[test]
    fn passkeys_are_unsupported_without_an_authenticator() {
        let s = MemoryStore::new();
        let u = PasskeyUser { id: vec![1], name: "a".into(), display_name: "A".into() };
        assert!(matches!(s.passkey_create("example.org", &u, b"c", &[]), Err(Error::Unsupported(_))));
        assert!(matches!(s.passkey_assert("example.org", b"c", &[]), Err(Error::Unsupported(_))));
    }
}
