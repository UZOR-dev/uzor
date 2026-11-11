# uzor-secure

A platform secure store behind one trait: secrets with biometry / device-credential
policies and a fail-closed unlock gate, passkey ceremonies, platform capabilities and
OS randomness. It never signs or verifies anything.

Backends: `MemoryStore` (tests), `FileStore` (0600 files, only for items that opt in
with `Policy::file_fallback`, never for gated items), and iOS / Android stubs that
answer `Unsupported` and document the planned design (Secure Enclave + ECIES-wrapped
Keychain items; Keystore / StrongBox AES-GCM).
