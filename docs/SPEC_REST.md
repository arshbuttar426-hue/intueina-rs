# SPEC SHEET — Remaining Build

**intueina-rs 0.2.0 → 1.0 Flagship**
*Foundation is war-tested. Product is not yet downloadable.*

| Field | Value |
|---|---|
| **Current** | 0.2.0 — 6 crates, 18 tests, 7 chambers, 10 parallel 0.39s, hell IN-LAW |
| **Target** | 1.0 — one-command install, 30s demo, MCP for Claude/Code |
| **Delta** | 5 slices, 3.5–5d realistic, **1d lean slice** useful |
| **Law** | `laws/exocortex.prima` — still the gate. No slice ships without exit 0. |
| **Gaps it closes** | SYNTHESIS V.1–6 (real workload, DB-backed, resolver catalog, multi-node, policy) — partially |

---

## 0. Where we stand

War-tested: store locked, embed typed, recall golden #1 (real BERT 0.721), gate 401/404, config strict, observer hash-chain. Binary boots real BERT in ~1s, smoke 7/7.

Not yet product: `think`/`review` are stubs, no MCP stdio, no CLI, no seed, no README, no `cargo install`, no migration from `~/.intueina/cortex.json` (23 memories, vectors need re-embed raw).

---

## 1. Slices (each has a stop condition)

### S1 — CLI + Seed + README (0.5–1d) · *Lean flagship*
- **Goal:** stranger gets a #1 recall in 30s.
- **Files:** `crates/cli` (clap subcommands `init`, `memorize`, `recall`, `serve`), `assets/seed.json` (ADHD rule + 2 probes + 5 verb-chain samples), `README.md` (install + demo).
- **Stop:** `cargo install intueina-rs && intueina init && intueina recall "ADHD rule" --k 1` prints the durable rule #1, no token needed for read.
- **Risk:** low.

### S2 — MCP stdio bridge (1–2d) · *Spec promise*
- **Goal:** "one config line" — Claude/Code mounts `intueina` via MCP `recall`/`memorize`/`think`/`review`.
- **Files:** `crates/mcp` (rmcp or json-rpc stdio), `server` route parity, `intueina.toml` `mcp_transport` actually wired.
- **Stop:** `claude mcp add intueina -- intueina-rs --mcp-stdio` → `recall` tool visible in Claude, round-trips.
- **Risk:** high — protocol, auth over stdio, test with real host.

### S3 — Real `think` / `review` (0.5–1d)
- **Goal:** close spec Layer 1 vaporware.
- **Files:** `crates/cortex` thinkpad (append-only, survives compaction), `crates/store` review table (`reviews` SQLite or JSONL), spaced repetition (3d doubling).
- **Stop:** `POST /v1/think` persists, `POST /v1/review` returns due items, unit tests for interval doubling.
- **Risk:** medium — schema, not algorithm.

### S4 — Verb-chain SQL inside the binary (0.5d)
- **Goal:** the 30-line recursive CTE is not a separate HTML — `recall --depth 3` walks `entities`/`relations` in SQLite, <1ms.
- **Files:** `crates/verb` (SQLite `entities`/`relations` + CTE), `assets/verb_chain_benchmark.gif` linked in README.
- **Stop:** `intueina recall --verb-depth 3 "prima"` returns `prima → turning → recall` chain, p50 <2ms.
- **Risk:** low — SQL already proven in the benchmark page.

### S5 — Release + .app/brainhaul bundle (0.5–1d)
- **Goal:** `cargo install` and a signed `.app` that embeds the 8 brainhaul skills (754M) as optional bundle.
- **Files:** `release.sh`, `crates/server` release profile, `app/` wrapper (reuse `intueina/app`).
- **Stop:** `cargo install --path .` succeeds on clean machine; `.app` launches and `health` is healthy.
- **Risk:** medium — code signing, model cache (416M) offline handling.

### S6 — Migration + Cutover (0.5d)
- **Goal:** existing brain (23 memories, vectors in query-space) re-embedded raw and imported.
- **Files:** `tools/migrate_v04.py` (read `cortex.json`, re-embed passages raw, tag probes as `test`, write to new store).
- **Stop:** `migrate --from ~/.intueina/cortex.json --to ~/.intueina-rs` → `recall "ADHD rule"` still #1, `stats` shows 23 nodes, `observer_healthy` true.
- **Risk:** low — one-shot, verified by golden.

---

## 2. Timeline

| Path | Scope | Calendar |
|---|---|---|
| **Lean** | S1 only | **1d** — useful download |
| **Full** | S1→S6 | **3.5–5d** — flagship 1.0 |

The 2d estimate earlier was foundation pace projected onto UX work. UX slips on MCP and signing.

---

## 3. War for the rest

Current war is 7/7 unit, live 10/10 after `spawn_blocking` patch. Remaining war for flagship:

- 20 parallel real BERT `learn` sustained (not just 10)
- Kill-9 mid-20-storm leaves valid store at 50+ nodes (scale)
- `think`/`review` still stubs — war will be red until S3 lands
- MCP round-trip under Claude — new chamber

No slice ships unless its war sub-test is green.

---

## 4. Definition of Done (1.0)

A stranger on a clean Mac:

```bash
cargo install intueina-rs
intueina init
intueina memorize "COMMUNICATION RULE: Be short." --type principle
intueina recall "ADHD rule"  # → #1, <200ms, no token
# and in Claude: one config line → recall tool appears
```

Plus `gate_exocortex.py` still exits 0 and `war_test.sh` (now 10 parallel) passes.

---

*Next action: pick Lean (S1, 1d) or Full (S1→S6, 3.5–5d). No code without a slice and its stop condition.*
