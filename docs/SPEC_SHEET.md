# SPEC SHEET — intueina-rs

**Intueina Exocortex — Deterministic Memory for AI Agents**
*No LLM in the hot path. Every mutation gated, every recall ranked, every store checksummed.*

| Field | Value |
|---|---|
| **Product** | intueina-rs |
| **Version** | 0.2.0 |
| **Status** | War-tested foundation — 7 chambers, 18 tests, 10 parallel 0.39s |
| **Binary** | `target/debug/intueina-rs` (release next) |
| **Port** | `9124` single constant — docs, config, and server generated from one source |
| **License** | AGPL v3 © 2026 Arshbuttar |
| **Law** | `laws/exocortex.prima` — exit 0 or it does not ship |

---

## 1. What it is / what it isn't

**Is:** a single-writer, gated, typed exocortex. You `memorize` decisions, you `recall` by meaning, the store never silently drops a write, and the gate refuses without a signed session. The verb-chain proves the thesis: relational recall is geometry, not an API call.

**Is not:** a chat log, a vector DB you bolt on, or an LLM wrapper. `think`/`review` are stubs until the observer is productized. No .app, no brainhaul bundle yet.

---

## 2. Core guarantees (each traces to a wound)

| Guarantee | Wound | War proof |
|---|---|---|
| **Single-writer, atomic** — file-locked, temp+rename, checksummed | F-007/F-011 last-writer-wins | 20-thread unit 0 lost, kill-9 valid |
| **Typed split** — `EmbedQuery` vs `EmbedPassage` won't compile wrong | F-009 prefix on passages | 4 tests, real BERT split holds |
| **Golden #1** — "ADHD communication rule" ranks its durable rule #1 | F-005 #4 behind probes | 0.721 vs 0.586/0.543 (real BERT + lexical boost) |
| **Gated mutations** — HMAC Bearer, vacuum needs confirm | F-004 open | unauth 401, nonexistent 404 |
| **Strict config** — unknown key → boot fails naming the key | F-001 decorative TOML | 3 tests |
| **Hash-chain observer** — tamper breaks health | F-010 absent | 2 tests, `observer_healthy` in `/health` |

---

## 3. Surface (8 routes, nothing less)

```
POST /v1/learn      {content, memory_type} → {id}          — gated write
POST /v1/recall     {query, k} → {memories:[{content,score}]} — read
POST /v1/memorize   alias of learn
POST /v1/think      → {thinkpad:[]}                         — stub
POST /v1/review     → {due:[]}                              — stub
GET  /v1/stats      → {nodes, observer_healthy}
GET  /health        → {status, port, observer_healthy, version}
POST /v1/vacuum     → {ok}                                  — gated, needs vacuum cap
DELETE /v1/nodes/{id} → {deleted} | 404                      — gated, nonexistent is 404
```

All mutating routes require `Authorization: Bearer <HMAC>` — demo token printed at boot (`sub:local, cap:[read,write,vacuum]`).

---

## 4. Performance (measured, not claimed)

| Metric | v0.4 | v0.2.0 |
|---|---|---|
| `learn` (incl. BERT) | — | ~90ms |
| `recall` (BERT + boost) | ~188ms, misranked | **~180ms, #1** |
| 10 parallel `learn` | ∞ (blocked runtime) | **0.39s, 10/10 survived** |
| Unit suite | 0 | **18 tests, 0 failed** |

Verb-chain (separate artifact, same thesis): 30-line recursive CTE, <1ms vs mem0 880ms — proves relational recall needs no LLM. intueina-rs recall is the semantic complement.

---

## 5. Install & 30-second demo

```bash
cargo build --bin intueina-rs --release
./target/release/intueina-rs --port 9124 --data-dir ~/.intueina &
TOK=$(grep "demo Bearer" /tmp/log | sed -E 's/.*token[^:]*: //')
curl -X POST http://127.0.0.1:9124/v1/learn -H "authorization: Bearer $TOK" -H 'content-type: application/json' -d '{"content":"COMMUNICATION RULE: Arsh has ADHD. Be short."}'
curl -X POST http://127.0.0.1:9124/v1/recall -H 'content-type: application/json' -d '{"query":"ADHD rule","k":1}'
# → COMMUNICATION RULE #1
```

Model cache: `~/.cache/huggingface` (Snowflake/snowflake-arctic-embed-m, 416M, mmap'd, offline after first fetch).

---

## 6. What's not in this sheet

MCP stdio bridge, `init`/`recall` CLI, seed bundle, .app wrapper, verb-chain SQL inside the binary, and `think`/`review` as real features — next slices. The engine is war-tested; the product packaging is the remaining delta.

---

*One store, one law, played open under AGPL v3 © 2026 Arshbuttar. `observer/OBSERVER.md` holds the field notes; `tools/gate_exocortex.py` and `tools/war_test.sh` are the hammers.*
