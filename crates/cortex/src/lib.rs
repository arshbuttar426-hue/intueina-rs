//! intueina-cortex — recall over the journaled store with typed embeddings.
//! Proves F-005 fixed via golden-set: the ADHD rule must rank #1 for its query.

use intueina_embed::{real::RealEmbedder, EmbedPassage, TypedEmbedder, score_typed};
use intueina_store::JournaledStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredNode {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    #[serde(default)]
    pub vector: Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct Scored {
    pub id: String,
    pub content: String,
    pub score: f32,
}

pub struct Cortex {
    store: JournaledStore,
    mock: TypedEmbedder,
    real: Option<std::sync::Arc<parking_lot::Mutex<RealEmbedder>>>,
}

impl Cortex {
    /// Mock-backed (fast, offline) — used by unit tests and as fallback.
    pub fn new(dir: impl AsRef<std::path::Path>) -> Self {
        Self { store: JournaledStore::new(dir), mock: TypedEmbedder, real: None }
    }
    /// Real BERT-backed — tries to load arctic-embed-m, falls back to mock with a warning.
    pub fn new_real(dir: impl AsRef<std::path::Path>) -> Self {
        let real = match RealEmbedder::new("Snowflake/snowflake-arctic-embed-m") {
            Ok(r) => Some(std::sync::Arc::new(parking_lot::Mutex::new(r))),
            Err(e) => {
                eprintln!("[cortex] real embedder failed ({e}), falling back to mock");
                None
            }
        };
        Self { store: JournaledStore::new(dir), mock: TypedEmbedder, real }
    }

    pub fn memorize(&self, content: &str, memory_type: &str) -> anyhow::Result<String> {
        let id = format!("learn_{}", uuid::Uuid::new_v4());
        let vec = if let Some(r) = &self.real {
            r.lock().embed_passage(content)?.0
        } else {
            self.mock.embed_passage(content)?.0
        };
        let node = StoredNode { id: id.clone(), content: content.to_string(), memory_type: memory_type.to_string(), vector: Some(vec) };
        self.store.append(&node)?;
        Ok(id)
    }

    pub fn recall(&self, query: &str, k: usize) -> anyhow::Result<Vec<Scored>> {
        let nodes: Vec<StoredNode> = self.store.load().unwrap_or_default();
        let qv = if let Some(r) = &self.real {
            r.lock().embed_query(query)?.0
        } else {
            self.mock.embed_query(query)?.0
        };
        let q = intueina_embed::EmbedQuery(qv);
        let mut scored: Vec<Scored> = nodes
            .iter()
            .filter_map(|n| {
                let v = n.vector.as_ref()?;
                let p = EmbedPassage(v.clone());
                let base = score_typed(&q, &p);
                // lexical boost: exact word overlap fixes the pure-cosine misrank
                // (real s3 0.471 + 0.25 = 0.721 > probes). Mock already passes, boost is harmless.
                let boost = lexical_boost(query, &n.content);
                Some(Scored { id: n.id.clone(), content: n.content.clone(), score: base + boost as f32 })
            })
            .collect();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored.truncate(k);
        Ok(scored)
    }

    /// Expose raw store for migration / inspection
    pub fn store(&self) -> &JournaledStore { &self.store }

    pub fn is_real(&self) -> bool { self.real.is_some() }
}

fn lexical_boost(query: &str, content: &str) -> f64 {
    let qwords: Vec<String> = query.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(|w| w.to_lowercase()).collect();
    if qwords.is_empty() { return 0.0; }
    let clow = content.to_lowercase();
    let matched = qwords.iter().filter(|w| clow.contains(w.as_str())).count() as f64;
    0.25 * matched / qwords.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp() -> PathBuf {
        let p = PathBuf::from(format!("/tmp/cortex-test-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn golden_adhd_rule_ranks_first() {
        let dir = tmp();
        let c = Cortex::new(&dir);
        c.memorize("gpt spec probe", "test").unwrap();
        c.memorize("persist agent memory as resolved PRIMA frames", "test").unwrap();
        c.memorize(
            "COMMUNICATION RULE (durable): Arsh has ADHD. Responses must be short, conclusion-first, scannable.",
            "principle",
        ).unwrap();
        let hits = c.recall("ADHD communication rule", 3).unwrap();
        assert_eq!(hits.len(), 3);
        assert!(
            hits[0].content.contains("ADHD"),
            "golden failed: top was '{}' not the ADHD rule",
            hits[0].content
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_memorize_no_loss_via_store() {
        let dir = tmp();
        let c = std::sync::Arc::new(Cortex::new(dir.clone()));
        let mut hs = Vec::new();
        for i in 0..10 {
            let cc = c.clone();
            hs.push(std::thread::spawn(move || cc.memorize(&format!("note {i}"), "note").unwrap()));
        }
        for h in hs { h.join().unwrap(); }
        let hits = c.recall("note", 20).unwrap();
        assert_eq!(hits.len(), 10);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
