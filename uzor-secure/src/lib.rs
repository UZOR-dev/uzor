//! A platform secure store behind one trait, for uzor apps on phones and desktops.
//!
//! - Secrets: [`SecureStore::secure_put`] / [`secure_get`](SecureStore::secure_get) /
//!   [`secure_delete`](SecureStore::secure_delete) with a [`Policy`] (biometry, device
//!   credential, this-device-only, file fallback opt-in).
//! - The gate: an item stored with biometry or device credential is read only with an
//!   [`UnlockHandle`] from [`SecureStore::unlock`] that this store issued and that has
//!   not expired. Anything else fails closed ([`Error::Locked`] / [`Error::Expired`]).
//! - Passkeys: [`SecureStore::passkey_create`] / [`passkey_assert`](SecureStore::passkey_assert)
//!   hand back the WebAuthn ceremony outputs the platform authenticator produced
//!   (ASAuthorization on iOS, Credential Manager on Android). This crate never signs
//!   or verifies anything itself; a relying party verifies.
//! - [`platform_caps`] says what the device has; [`random`] reads the OS CSPRNG.
//!
//! Backends: [`MemoryStore`] (tests, ephemeral), [`FileStore`] (0600 files, only for
//! items whose policy opts in with `file_fallback`), and the per-OS store from
//! [`platform`]: iOS and Android are stubs that answer [`Error::Unsupported`] and
//! document the planned approach ([`ios`], [`android`]).

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

mod file;
mod memory;
pub mod android;
pub mod ios;

pub use file::FileStore;
pub use memory::MemoryStore;

/// How an item is protected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Policy {
    /// Reading needs a biometric unlock (Face ID / Touch ID / fingerprint).
    pub biometry: bool,
    /// Reading needs the device passcode / PIN / pattern (or biometry).
    pub device_credential: bool,
    /// Never leaves this device (no iCloud Keychain / backup / transfer).
    pub this_device_only: bool,
    /// The item may live in a plain 0600 file when the platform has no secure
    /// store. Off by default: without it such a platform refuses the item.
    pub file_fallback: bool,
}

impl Policy {
    /// The item is behind the unlock gate.
    pub fn gated(&self) -> bool {
        self.biometry || self.device_credential
    }
}

/// Proof of a recent user unlock, from [`SecureStore::unlock`]. Valid for the store
/// that issued it until it expires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnlockHandle {
    store: u64,
    id: u64,
    expires: Instant,
}

impl UnlockHandle {
    /// A handle issued by store `store` (stores use [`next_store_id`]).
    pub fn issue(store: u64, ttl: Duration) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        UnlockHandle { store, id: NEXT.fetch_add(1, Ordering::Relaxed), expires: Instant::now() + ttl }
    }

    /// When it stops opening gated items.
    pub fn expires(&self) -> Instant {
        self.expires
    }

    /// Whether `store` issued it and it is still valid; the gate every store applies.
    pub fn check(&self, store: u64) -> Result<(), Error> {
        if self.store != store {
            return Err(Error::Locked);
        }
        if Instant::now() >= self.expires {
            return Err(Error::Expired);
        }
        Ok(())
    }
}

/// A fresh id for a store instance (handles are bound to it).
pub fn next_store_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Secret bytes, zeroed when dropped.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(Vec<u8>);

impl Secret {
    /// Wrap bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Secret(bytes)
    }
    /// The bytes.
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret({} bytes)", self.0.len())
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        for b in self.0.iter_mut() {
            // SAFETY: a valid, aligned, exclusive reference into the vector.
            unsafe { std::ptr::write_volatile(b, 0) };
        }
        std::sync::atomic::compiler_fence(Ordering::SeqCst);
    }
}

/// The WebAuthn user of a new passkey.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyUser {
    /// User handle (opaque, at most 64 bytes).
    pub id: Vec<u8>,
    /// Account name (`ivan@example.org`).
    pub name: String,
    /// Display name.
    pub display_name: String,
}

/// What a registration ceremony returns (send it to the relying party).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyCreated {
    pub credential_id: Vec<u8>,
    pub attestation_object: Vec<u8>,
    pub client_data_json: Vec<u8>,
}

/// What an assertion ceremony returns (send it to the relying party).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasskeyAssertion {
    pub credential_id: Vec<u8>,
    pub authenticator_data: Vec<u8>,
    pub client_data_json: Vec<u8>,
    pub signature: Vec<u8>,
    pub user_handle: Option<Vec<u8>>,
}

/// What the device offers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformCaps {
    /// Keys live in hardware (Secure Enclave, TEE / StrongBox, TPM).
    pub hw_backed: bool,
    /// Android StrongBox (a separate secure element).
    pub strongbox: bool,
    /// Apple Secure Enclave.
    pub secure_enclave: bool,
    /// iOS major version or Android API level, when known.
    pub os_api_level: Option<u32>,
    /// Items that opt in can be kept in 0600 files.
    pub file_fallback: bool,
}

/// Why a call failed. Every failure leaves the item unread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// This platform / backend cannot do it (yet); the text says what is missing.
    Unsupported(&'static str),
    /// No item by that name.
    NotFound,
    /// A gated item and no valid handle from this store.
    Locked,
    /// The handle expired.
    Expired,
    /// The user cancelled or failed the unlock / ceremony.
    Denied,
    /// A bad name or argument.
    Invalid(&'static str),
    /// The OS store failed.
    Backend(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unsupported(w) => write!(f, "unsupported: {w}"),
            Error::NotFound => f.write_str("no such item"),
            Error::Locked => f.write_str("locked: unlock first"),
            Error::Expired => f.write_str("unlock expired"),
            Error::Denied => f.write_str("denied by the user"),
            Error::Invalid(w) => write!(f, "invalid: {w}"),
            Error::Backend(e) => write!(f, "secure store: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// The secure store.
pub trait SecureStore: Send + Sync {
    /// What this store and device offer.
    fn caps(&self) -> PlatformCaps;
    /// Store `bytes` under `name` (replaces an existing item).
    fn secure_put(&self, name: &str, bytes: &[u8], policy: Policy) -> Result<(), Error>;
    /// Read `name`. A gated item needs `unlock`, issued by this store and unexpired.
    fn secure_get(&self, name: &str, unlock: Option<&UnlockHandle>) -> Result<Secret, Error>;
    /// Remove `name` (removing a missing item is `NotFound`).
    fn secure_delete(&self, name: &str) -> Result<(), Error>;
    /// Ask the user to unlock (biometry / device credential) with this prompt; the
    /// handle opens gated items for `ttl`.
    fn unlock(&self, reason: &str, ttl: Duration) -> Result<UnlockHandle, Error>;
    /// Register a platform passkey for `rp_id`.
    fn passkey_create(&self, rp_id: &str, user: &PasskeyUser, challenge: &[u8], exclude_ids: &[Vec<u8>]) -> Result<PasskeyCreated, Error>;
    /// Sign in with a platform passkey for `rp_id` (any of `allow_ids`, or a
    /// discoverable one when empty).
    fn passkey_assert(&self, rp_id: &str, challenge: &[u8], allow_ids: &[Vec<u8>]) -> Result<PasskeyAssertion, Error>;
}

/// Item names: 1..=128 bytes of `[A-Za-z0-9._-]`, not starting with a dot (they
/// become Keychain account names, Keystore aliases and file names).
pub fn check_name(name: &str) -> Result<(), Error> {
    let ok = !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
    if ok { Ok(()) } else { Err(Error::Invalid("name must be 1..=128 of [A-Za-z0-9._-], not starting with '.'")) }
}

/// `n` bytes from the OS CSPRNG.
pub fn random(n: usize) -> Result<Vec<u8>, Error> {
    let mut buf = vec![0u8; n];
    getrandom::getrandom(&mut buf).map_err(|e| Error::Backend(format!("OS randomness: {e}")))?;
    Ok(buf)
}

/// This platform's store: the iOS / Android stubs on phones; elsewhere a store with
/// no secure backend that keeps only `file_fallback` items in `dir` as 0600 files.
pub fn platform(app: &str) -> Box<dyn SecureStore> {
    #[cfg(target_os = "ios")]
    {
        let _ = app;
        Box::new(ios::IosStore::new())
    }
    #[cfg(target_os = "android")]
    {
        let _ = app;
        Box::new(android::AndroidStore::new())
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    {
        Box::new(FileStore::new(FileStore::default_dir(app)))
    }
}

/// What [`platform`]'s store offers on this device.
pub fn platform_caps() -> PlatformCaps {
    platform("caps").caps()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_is_sized_and_not_constant() {
        let a = random(32).unwrap();
        let b = random(32).unwrap();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(random(0).unwrap().is_empty());
    }

    #[test]
    fn names_are_checked() {
        assert!(check_name("ssh.node-key_1").is_ok());
        for bad in ["", ".hidden", "a/b", "../x", "пароль", &"x".repeat(129)] {
            assert!(check_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn secret_debug_hides_bytes() {
        assert_eq!(format!("{:?}", Secret::new(b"hunter2".to_vec())), "Secret(7 bytes)");
    }

    #[test]
    fn handles_are_bound_to_their_store_and_expire() {
        let h = UnlockHandle::issue(7, Duration::from_secs(60));
        assert_eq!(h.check(7), Ok(()));
        assert_eq!(h.check(8), Err(Error::Locked));
        let old = UnlockHandle::issue(7, Duration::ZERO);
        assert_eq!(old.check(7), Err(Error::Expired));
    }
}
