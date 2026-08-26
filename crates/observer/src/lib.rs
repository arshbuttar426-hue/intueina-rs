//! intueina-observer — hash-chained event log, tamper-evident (F-010).
//! Every mutation appends an event; chain break => health degraded.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub ts: u64,
    pub actor: String,
    pub op: String,
    pub content_hash: String,
    pub prev_hash: String,
    pub hash: String, // sha256(prev_hash + ts + actor + op + content_hash)
}

fn hash_event(prev: &str, ts: u64, actor: &str, op: &str, ch: &str) -> String {
    let mut h = Sha256::new();
    h.update(prev.as_bytes());
    h.update(ts.to_le_bytes());
    h.update(actor.as_bytes());
    h.update(op.as_bytes());
    h.update(ch.as_bytes());
    hex::encode(h.finalize())
}

pub struct Observer {
    inner: Mutex<Vec<Event>>,
}

impl Observer {
    pub fn new() -> Self { Self { inner: Mutex::new(Vec::new()) } }

    pub fn record(&self, actor: &str, op: &str, content: &str) -> Event {
        let ch = hex::encode(Sha256::digest(content.as_bytes()));
        let mut evs = self.inner.lock();
        let prev = evs.last().map(|e| e.hash.clone()).unwrap_or_else(|| "0".repeat(64));
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let hash = hash_event(&prev, ts, actor, op, &ch);
        let ev = Event { ts, actor: actor.into(), op: op.into(), content_hash: ch, prev_hash: prev, hash };
        evs.push(ev.clone());
        ev
    }

    pub fn verify_chain(&self) -> Result<(), String> {
        let evs = self.inner.lock();
        let mut prev = "0".repeat(64);
        for e in evs.iter() {
            let expect = hash_event(&prev, e.ts, &e.actor, &e.op, &e.content_hash);
            if e.hash != expect { return Err(format!("chain break at {}", e.hash)); }
            if e.prev_hash != prev { return Err(format!("prev mismatch at {}", e.hash)); }
            prev = e.hash.clone();
        }
        Ok(())
    }

    pub fn len(&self) -> usize { self.inner.lock().len() }
    pub fn is_healthy(&self) -> bool { self.verify_chain().is_ok() }
}

impl Default for Observer { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_verifies() {
        let o = Observer::new();
        o.record("arsh", "memorize", "hello");
        o.record("arsh", "memorize", "world");
        assert!(o.verify_chain().is_ok());
        assert_eq!(o.len(), 2);
    }

    #[test]
    fn tamper_breaks_chain() {
        let o = Observer::new();
        o.record("a", "op", "x");
        o.record("a", "op", "y");
        { let mut evs = o.inner.lock(); evs[0].content_hash = "bad".into(); }
        assert!(o.verify_chain().is_err());
        assert!(!o.is_healthy());
    }
}
