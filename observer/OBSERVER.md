# OBSERVER — Intueina Field Notes

**Mission:** audit the existing intueina stack to failure, record every break,
then rebuild industrial-grade from zero (`intueina-rs`), prima-gated.

**Method:** no fixes during Phase A. Observe, probe, record. Every entry:
timestamp, component, severity (S0 blocker / S1 broken / S2 wrong / S3 smell),
evidence (command + output), and what it implies for the rebuild.

**Legend:** S0 = blocks shipping · S1 = feature is broken · S2 = works but
contradicts spec/design · S3 = debt/smell that will bite at scale.

---

## FIELD NOTES

<!-- entries appended below, newest at bottom -->

### F-001 · S1 · Config file is dead · 2026-08-26
`~/.intueina.toml` declares `state.db`, `chroma/`, `version 0.2.0`. Disk has neither
store — real store is `cortex.json`, which the config never mentions. `config.rs:20-23`
reads only `$HOME` and hardcodes `~/.intueina/cortex.json` + port 9124. The TOML is
never parsed. Two declared sources of truth; one of them decorative.
**Rebuild rule:** one config format, actually loaded, validated at boot, fail-closed
on unknown keys.

### F-002 · S2 · Port anarchy · 2026-08-26
Constitution says `127.0.0.1:9124`. spec.md says WS 9123 + MCP-http 9124.
`registry.toml` comment says "Rust server is REST-only on 9124". Runtime logs show
historical binds on **9125**. Four documents, three ports.
**Rebuild rule:** single port constant, defined once, cited by docs generated from it.

### F-003 · S1 · Spec↔implementation gap: 3 of 5 core ops missing · 2026-08-26
spec.md Layer 1 promises recall/memorize/think/review/learn. Rust v0.4 routes:
`/v1/learn`, `/v1/recall`, `/v1/vacuum`, `/v1/nodes/{id}` DELETE, `/v1/stats`, `/ws`.
POST `/memorize`, `/v1/think`, `/v1/review` → **404**. The Thinkpad (working memory)
and spaced-repetition review — two of the three layers in the spec's own architecture —
do not exist server-side.
**Rebuild rule:** spec and API surface generated from one source; conformance test
per promised op.

### F-004 · S0 · No gate on a brain · 2026-08-26
Any local process can read all memory, delete any node (`DELETE /v1/nodes/x`
→ **200 without existing node, without any credential**), or wipe the store
(`/v1/vacuum`). For an exocortex this violates the project's own first principle
(prima I.7: gated matter, signed sessions, refusal at the edge).
**Rebuild rule:** every mutating call HMAC-signed session; destructive calls
require explicit confirm token; unknown node deletes are 404, never 200.

### F-005 · S1 · Recall is not meaning-deep · 2026-08-26
Query `"ADHD communication rule"` against a store containing that exact durable
rule → top hit: `"gpt spec probe"` (score 0.879), second: PRIMA-frames note (0.871),
`matched_words: []` on everything. Embeddings load (22/23) but ranking behaves as
word-frequency overlap, not cosine-semantic. The spec's own diagnosis of the old
cortex ("word-match deep, not meaning-deep") still describes v0.4.
**Rebuild rule:** recall scored by embedding cosine first, lexical overlap only as
tiebreak; golden-set retrieval tests in CI (query must rank its target #1).

### F-006 · S2 · Silent partial embedding · 2026-08-26
One node (`"the sky is blue"`) has `vector:null, model:null`; `/health` reports
`embeddings:22 nodes:23` and status `healthy`. No quarantine, no repair pass,
no alert. A memory that can never be semantically recalled is invisible rot.
**Rebuild rule:** ingest is transactional — unembedded writes fail closed or get
repaired on next boot; health distinguishes degraded from healthy.

### F-007 · S3 · Whole-blob JSON persistence · 2026-08-26
Store is a 241KB JSON array rewritten wholesale per mutation (load → mutate → dump).
Crash mid-write corrupts the brain. Spec itself calls for SQLite + Chroma pluggable
backends; neither exists on disk.
**Rebuild rule:** append-only journal + periodic snapshot, atomic rename, checksummed;
backend trait from day one.

### F-008 · S3 · Test residue in the living brain · 2026-08-26
`"the sky is blue"`, `"gpt spec probe"`, `"intueina healthcheck probe"` sit among
durable rules with identical schema/status. No memory_type distinction between
probe and principle.
**Rebuild rule:** probe writes require `memory_type:"test"` and are excluded from
recall unless explicitly asked.

---

## WHAT WORKS (honesty column)

- P-001 · `cargo check` + build clean, zero warnings surfaced; cold boot to healthy ~2s.
- P-002 · Single consistent schema across all 23 entries — disciplined writer.
- P-003 · `/health` reports truthful internal counts rather than a blanket ok.

