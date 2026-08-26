//! intueina-gate — HMAC sessions, gated mutations (F-004).
//! Every mutating call requires a signed session; vacuum needs a confirm token.
//! DELETE of nonexistent id is 404, never 200.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub cap: Vec<String>, // capabilities: ["read","write","vacuum"]
}

pub struct Gate {
    secret: Vec<u8>,
}

impl Gate {
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self { secret: secret.as_ref().to_vec() }
    }

    pub fn sign(&self, claims: &Claims) -> String {
        let payload = B64.encode(serde_json::to_vec(claims).unwrap());
        let mut mac = HmacSha256::new_from_slice(&self.secret).unwrap();
        mac.update(payload.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        format!("{payload}.{sig}")
    }

    pub fn verify(&self, token: &str) -> anyhow::Result<Claims> {
        let (payload, sig) = token.split_once('.').ok_or_else(|| anyhow::anyhow!("malformed token"))?;
        let mut mac = HmacSha256::new_from_slice(&self.secret).unwrap();
        mac.update(payload.as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());
        if sig != expected { anyhow::bail!("bad signature"); }
        let claims: Claims = serde_json::from_slice(&B64.decode(payload)?)?;
        Ok(claims)
    }

    pub fn can(&self, token: &str, cap: &str) -> bool {
        self.verify(token).map(|c| c.cap.iter().any(|x| x == cap)).unwrap_or(false)
    }

    /// Vacuum confirm token: HMAC of "vacuum:<nonce>"
    pub fn vacuum_challenge(&self, nonce: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret).unwrap();
        mac.update(format!("vacuum:{nonce}").as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }
    pub fn verify_vacuum(&self, nonce: &str, token: &str) -> bool {
        self.vacuum_challenge(nonce) == token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> Gate { Gate::new(b"test-secret-32-bytes-1234567890ab") }

    #[test]
    fn sign_verify_roundtrip() {
        let g = gate();
        let claims = Claims { sub: "arsh".into(), exp: 9999999999, cap: vec!["read".into(), "write".into()] };
        let tok = g.sign(&claims);
        let back = g.verify(&tok).unwrap();
        assert_eq!(back, claims);
    }

    #[test]
    fn tampered_payload_rejected() {
        let g = gate();
        let tok = g.sign(&Claims { sub: "a".into(), exp: 0, cap: vec![] });
        let bad = tok.replacen('a', "b", 1);
        assert!(g.verify(&bad).is_err());
    }

    #[test]
    fn missing_cap_denied() {
        let g = gate();
        let tok = g.sign(&Claims { sub: "x".into(), exp: 0, cap: vec!["read".into()] });
        assert!(!g.can(&tok, "vacuum"));
        assert!(g.can(&tok, "read"));
    }

    #[test]
    fn vacuum_challenge() {
        let g = gate();
        let c = g.vacuum_challenge("nonce123");
        assert!(g.verify_vacuum("nonce123", &c));
        assert!(!g.verify_vacuum("other", &c));
    }
}
