//! Android store: a stub that answers [`Error::Unsupported`] for now.
//!
//! Planned approach (no signing in this crate; all of it is platform API through
//! JNI from the Kotlin host):
//! - **Secrets**: an AES-256-GCM key in the Android Keystore per app
//!   (`KeyGenParameterSpec`, `setIsStrongBoxBacked(true)` when StrongBox exists, else
//!   the TEE; `setUserAuthenticationRequired` + `setUserAuthenticationParameters(ttl,
//!   AUTH_BIOMETRIC_STRONG | AUTH_DEVICE_CREDENTIAL)` per [`Policy`];
//!   `setUnlockedDeviceRequired(true)`). Each item is wrapped with that key (random
//!   96-bit IV, the item name as AAD) and the ciphertext kept in app-private storage,
//!   excluded from backup (`android:allowBackup` rules), so it is this-device-only.
//! - **Gate**: [`SecureStore::unlock`] shows `BiometricPrompt` with the reason; the
//!   Keystore key only decrypts within the authentication window, so a missing or
//!   expired handle makes the Keystore refuse: fail closed in hardware.
//! - **Passkeys**: Credential Manager (`CreatePublicKeyCredentialRequest` /
//!   `GetPublicKeyCredentialOption`, Digital Asset Links for `rp_id`), returning the
//!   registration / authentication response fields.
//! - **Caps**: `strongbox` from `PackageManager.FEATURE_STRONGBOX_KEYSTORE`,
//!   `hw_backed` from `KeyInfo.securityLevel`, `os_api_level` `Build.VERSION.SDK_INT`.

use std::time::Duration;

use crate::{Error, PasskeyAssertion, PasskeyCreated, PasskeyUser, PlatformCaps, Policy, Secret, SecureStore, UnlockHandle};

const TODO: &str = "Android secure store not implemented yet (planned: Keystore / StrongBox AES-GCM wrapped items)";

/// The Android store (stub).
#[derive(Default)]
pub struct AndroidStore;

impl AndroidStore {
    pub fn new() -> Self {
        AndroidStore
    }
}

impl SecureStore for AndroidStore {
    fn caps(&self) -> PlatformCaps {
        PlatformCaps::default()
    }
    fn secure_put(&self, _: &str, _: &[u8], _: Policy) -> Result<(), Error> {
        Err(Error::Unsupported(TODO))
    }
    fn secure_get(&self, _: &str, _: Option<&UnlockHandle>) -> Result<Secret, Error> {
        Err(Error::Unsupported(TODO))
    }
    fn secure_delete(&self, _: &str) -> Result<(), Error> {
        Err(Error::Unsupported(TODO))
    }
    fn unlock(&self, _: &str, _: Duration) -> Result<UnlockHandle, Error> {
        Err(Error::Unsupported("Android unlock not implemented yet (planned: BiometricPrompt + Keystore auth window)"))
    }
    fn passkey_create(&self, _: &str, _: &PasskeyUser, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyCreated, Error> {
        Err(Error::Unsupported("Android passkeys not implemented yet (planned: Credential Manager)"))
    }
    fn passkey_assert(&self, _: &str, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyAssertion, Error> {
        Err(Error::Unsupported("Android passkeys not implemented yet (planned: Credential Manager)"))
    }
}
