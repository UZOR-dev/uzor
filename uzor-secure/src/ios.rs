//! iOS store: a stub that answers [`Error::Unsupported`] for now.
//!
//! Planned approach (no signing in this crate; all of it is Apple API):
//! - **Secrets**: a Secure Enclave P-256 key per app
//!   (`SecKeyCreateRandomKey`, `kSecAttrTokenIDSecureEnclave`, access control
//!   `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` plus `.biometryCurrentSet` /
//!   `.devicePasscode` per [`Policy`]). Each item is ECIES-wrapped to that key
//!   (`SecKeyCreateEncryptedData`, `eciesEncryptionCofactorVariableIVX963SHA256AESGCM`)
//!   and the ciphertext kept as a generic-password Keychain item, `ThisDeviceOnly`,
//!   so it never syncs to iCloud nor restores to another device.
//! - **Gate**: [`SecureStore::unlock`] evaluates an `LAContext` with the reason and
//!   keeps it for the TTL (`touchIDAuthenticationAllowableReuseDuration`);
//!   unwrapping a gated item passes that context (`kSecUseAuthenticationContext`),
//!   so a missing or expired handle makes the Enclave refuse: fail closed in hardware.
//! - **Passkeys**: `ASAuthorizationPlatformPublicKeyCredentialProvider`
//!   registration / assertion requests (associated domain `webcredentials:<rp_id>`),
//!   returning the raw attestation object, authenticator data, client data JSON,
//!   signature and user handle.
//! - **Caps**: `secure_enclave` from `SecureEnclave.isAvailable`, `os_api_level` the
//!   iOS major version.

use std::time::Duration;

use crate::{Error, PasskeyAssertion, PasskeyCreated, PasskeyUser, PlatformCaps, Policy, Secret, SecureStore, UnlockHandle};

const TODO: &str = "iOS secure store not implemented yet (planned: Secure Enclave P-256 + ECIES-wrapped Keychain items, ThisDeviceOnly)";

/// The iOS store (stub).
#[derive(Default)]
pub struct IosStore;

impl IosStore {
    pub fn new() -> Self {
        IosStore
    }
}

impl SecureStore for IosStore {
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
        Err(Error::Unsupported("iOS unlock not implemented yet (planned: LAContext reused for the TTL)"))
    }
    fn passkey_create(&self, _: &str, _: &PasskeyUser, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyCreated, Error> {
        Err(Error::Unsupported("iOS passkeys not implemented yet (planned: ASAuthorizationPlatformPublicKeyCredentialProvider)"))
    }
    fn passkey_assert(&self, _: &str, _: &[u8], _: &[Vec<u8>]) -> Result<PasskeyAssertion, Error> {
        Err(Error::Unsupported("iOS passkeys not implemented yet (planned: ASAuthorizationPlatformPublicKeyCredentialProvider)"))
    }
}
