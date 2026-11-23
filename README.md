# intueina-rs

Rust exocortex: auditable memory judgment instead of trust-the-inference. Every
requirement traces to an observer finding, a law frame, and a war-test.

## Crates

| Crate | Owns |
| --- | --- |
| `store` | WAL journal, snapshots, file lock, checksums |
| `embed` | typed two-path embeddings (`EmbedQuery` / `EmbedPassage`) |
| `cortex` | nodes, recall scoring, golden-set evaluation |
| `gate` | HMAC sessions, mutation authz, vacuum confirm tokens |
| `observer` | hash-chain event log, drift detection |
| `server` | axum routes |
| `config` | strict TOML, fail-closed |

## Run

```sh
cargo build --release
cargo run -p intueina-server -- --port 9124
python3 tools/gate_exocortex.py   # exit 0 or it does not ship
```

Spec: `docs/SPEC.md`, `docs/SPEC_SHEET.md`, `docs/SPEC_REST.md`.
Field notes: `observer/OBSERVER.md`. Law: `laws/exocortex.prima`.
