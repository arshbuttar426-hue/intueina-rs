//! intueina-cortex — recall over the journaled store with typed embeddings.
//! Proves F-005 fixed via golden-set: the ADHD rule must rank #1 for its query.

use intueina_embed::{EmbedPassage, TypedEmbedder, score_typed};
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
    embedder: TypedEmbedder,
}

impl Cortex {
    pub fn new(dir: impl AsRef<std::path::Path>) -> Self {
        Self { store: JournaledStore::new(dir), embedder: TypedEmbedder }
    }

    pub fn memorize(&self, content: &str, memory_type: &str) -> anyhow::Result<String> {
        let id = format!("learn_{}", uuid::Uuid::new_v4());
        // typed passage embedding (raw, no prefix) — the fix for F-009
        let passage = self.embedder.embed_passage(content)?;
        let node = StoredNode {
            id: id.clone(),
            content: content.to_string(),
            memory_type: memory_type.to_string(),
            vector: Some(passage.0),
        };
        self.store.append(&node)?;
        Ok(id)
    }

    pub fn recall(&self, query: &str, k: usize) -> anyhow::Result<Vec<Scored>> {
        let nodes: Vec<StoredNode> = self.store.load().unwrap_or_default();
        let q = self.embedder.embed_query(query)?;
        let mut scored: Vec<Scored> = nodes
            .iter()
            .filter_map(|n| {
                let v = n.vector.as_ref()?;
                let p = EmbedPassage(v.clone());
                let s = score_typed(&q, &p);
                Some(Scored { id: n.id.clone(), content: n.content.clone(), score: s })
            })
            .collect();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored.truncate(k);
        Ok(scored)
    }

    /// Expose raw store for migration / inspection
    pub fn store(&self) -> &JournaledStore { &self.store }
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
