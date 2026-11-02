//! The per-run bearer token and its constant-time check.

use std::fmt;

/// 32 random bytes; shown as 64 hex chars. `Debug` never prints it.
#[derive(Clone, PartialEq, Eq)]
pub struct Token([u8; 32]);

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(..)")
    }
}

impl Token {
    /// A fresh token from the OS random source.
    pub fn generate() -> std::io::Result<Self> {
        let mut b = [0u8; 32];
        getrandom::getrandom(&mut b).map_err(|e| std::io::Error::other(format!("random source: {e}")))?;
        Ok(Self(b))
    }

    /// A token from known bytes (tests).
    pub fn from_bytes(b: [u8; 32]) -> Self {
        Self(b)
    }

    /// Lower-case hex.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Does `presented` (the hex a client sent) equal this token? Runs in
    /// time independent of where the strings differ.
    pub fn verify(&self, presented: &str) -> bool {
        ct_eq(self.to_hex().as_bytes(), presented.trim().as_bytes())
    }
}

/// Constant-time equality for byte strings (length is not secret).
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    std::hint::black_box(diff) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_hex_and_verify() {
        let a = Token::generate().unwrap();
        let b = Token::generate().unwrap();
        assert_ne!(a, b);
        let h = a.to_hex();
        assert_eq!(h.len(), 64);
        assert!(a.verify(&h));
        assert!(!a.verify(&b.to_hex()));
        assert!(!a.verify(&h[..63]));
        assert!(!a.verify(""));
        assert_eq!(format!("{a:?}"), "Token(..)");
    }

    #[test]
    fn ct_eq_basic() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }
}
