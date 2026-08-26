# SPEC — intueina-rs

**Doctrine:** auditable judgment replaces trust-the-inference. Every requirement
below traces to an OBSERVER.md finding. No requirement without a wound, no wound
without a law frame, no law frame without a war-test.

**Law:** `laws/exocortex.prima` · **Field notes:** `observer/OBSERVER.md`
**Gate:** `tools/gate_exocortex.py` — exit 0 or it does not ship.

---

## 1. Crates

| Crate | Owns | Traces to |
|---|---|---|
| `store` | WAL journal + snapshots, file lock, checksums | F-007, F-011 |
| `embed` | Typed two-path embedding (`EmbedQuery` / `EmbedPassage`) | F-009 |
| `cortex` | nodes, recall scoring, golden-set evaluation | F-005 |
| `gate` | HMAC sessions, mutation authz, vacuum confirm tokens | F-004 |
| `observer` | hash-chain event log, drift detection, WS alerts | F-010 |
| `server` | axum routes = spec surface, nothing less | F-003 |
| `config` | strict TOML, fail-closed, port constant exported | F-001, F-002 |

## 2. Store (store)

- Append-only JSONL journal; snapshot + SHA-256 every N writes; atomic rename.
- Boot: load snapshot → replay journal → verify checksums → repair-or-refuse.
- Single-writer: advisory file lock; second process **waits with log**, never forks
  a second brain. Multi-reader fine.
- Adopt protocol: if a journal from another instance is found unmerged, merge by
  node id + created ts; conflicts raise observer events, never silence.

War-tests: kill -9 mid-storm ⇒ valid store; two writers ⇒ both entries present.

## 3. Embedding (embed)

```rust
pub struct EmbedQuery(Vec<f32>);   // constructed ONLY via query path (prefixed)
pub struct EmbedPassage(Vec<f32>); // constructed ONLY via passage path (raw)
```

- arctic-embed-m via candle, CPU. Model files resolved once, cached, offline-first.
- The compiler enforces the asymmetric contract: `score(query: &EmbedQuery,
  passage: &EmbedPassage)` — wrong-path values do not compile.
- Ingest is transactional: a node whose passage embedding fails is NOT stored as
  healthy; boot runs an embed-repair pass for any `vector: null`.

War-tests: golden set ranks target #1; null-vector writes are impossible by API.

## 4. Cortex / recall

- Score = cosine(`EmbedQuery`, `EmbedPassage`) primary; lexical overlap tiebreak;
  NO length penalty until data says otherwise.
- Golden set lives in `tests/golden/*.json`: {query, must_rank_first}. CI fails if
  any target not ranked first. Seeded from real corpus incl. the ADHD rule that
  v0.4 buried behind probes.
- Density limits (minimal/balanced/dense) kept from v0.4 router design.

## 5. Gate (auth)

- Every request carries an HMAC-signed session (same construction as prima's own
  functions gate). Read endpoints: valid session. Mutations: session + capability.
- `DELETE /v1/nodes/{id}` of a nonexistent id → **404**, never 200.
- `/v1/vacuum` requires a second confirm token obtained from a GET challenge.
- Local-only bind default; remote bind demands explicit config opt-in + TLS note.

## 6. Surface (server)

Full conformance to intueina spec Layer 1 — each op has a route AND a
conformance test asserting its contract:

recall · memorize · think · review · learn · vacuum · stats · health · ws-events.

No op may exist in docs without a route; no route without a test. Generated
API table is built from the same source constant as the router definitions
(F-002/F-003 class errors become compile-time impossible).

## 7. Config

One `intueina.toml`: port, bind, model, paths, intervals. Strict parse — unknown
key ⇒ boot failure naming the key (fail-closed, F-001). Port constant defined
once in code; README/spec snippets are generated from it (F-002).

## 8. Observer (dreamswap-lite)

In-process: every mutation appends {ts, actor, op, content_hash} to a hash-chained
event log. Contradiction detector compares new durable claims against existing
frames (token-overlap + embedding distance thresholds); hits emit WS events and
are surfaced in `/v1/stats`. Tamper-evident: chain break ⇒ health reports
`degraded`, never healthy (F-006 principle applied to self).

## 9. Ops / succession

Release profile only; install script symlinks `target/release`. Restart protocol:
new instance boots → passes self-check → acquires lock → old instance drains and
retires. If new instance fails self-check, old stays authoritative. (Living-
computation handoff, applied to the exocortex itself.)

## 10. Data hygiene

`memory_type` taxonomy: principle | decision | fact | note | test.
`test` memories are quarantined from recall unless `include_test=true`.
Migration tool imports v0.4 cortex.json, tagging obvious probes as `test`
(F-008) and re-embedding everything raw (F-009).

## 11. Traceability

| Finding | Frame | Requirement § | War-test |
|---|---|---|---|
| F-001 config dead | config loaded | 7 | boot rejects bad toml |
| F-002 port anarchy | endpoint single | 7 | docs grep == constant |
| F-003 vaporware ops | surface promised | 6 | per-op conformance |
| F-004 no gate | mutation gated | 5 | unauth delete ⇒ 401/404 |
| F-005 misrank | recall semantic | 4 | golden set CI |
| F-006 hidden rot | health truthful | 3, 8 | degraded ≠ healthy |
| F-007 blob rewrite | memory journaled | 2 | kill -9 storm |
| F-008 residue | exocortex as memory | 10 | quarantine test |
| F-009 prefix bug | passage raw_text | 3 | type-enforced paths |
| F-010 no observer | drift detected | 8 | contradiction fires event |
| F-011 lost writes | store one_brain | 2 | dual-writer survival |
| F-012 debug prod | build release | 9 | succession drill |

---

one store, one law, played open under AGPL v3 © 2026 Arshbuttar.
