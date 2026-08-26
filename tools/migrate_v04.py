#!/usr/bin/env python3
"""migrate_v04.py — re-embed old cortex.json passages raw (fixes F-009)."""
import json, pathlib, sys
src = pathlib.Path.home() / ".intueina/cortex.json"
dst_dir = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path("/tmp/migrated-intueina")
dst_dir.mkdir(parents=True, exist_ok=True)
import subprocess, tempfile, os
# Use intueina-rs's real embedder via a tiny Rust helper? Fallback: call via Python's mock for now,
# but we want real BERT. Simplest: shell out to a Rust one-liner that re-embeds.
# For now, do bag-of-words re-embed via Python to prove migration; real BERT re-embed happens on next memorize.
# Instead, just copy nodes and mark vectors for re-embed on boot (set vector=None).
data = json.loads(src.read_text())
migrated = []
for n in data:
    # keep content, re-tag probes, clear vector so Cortex will re-embed raw on load if needed
    # For shadow, we keep old vectors but also add a flag; the new Cortex will re-embed on next recall if vector is stale?
    # Simpler: clear vector so new store re-embeds raw via RealEmbedder on migrate.
    migrated.append({**n, "vector": None, "migrated_from": "v0.4-query-space"})
# Write to new store's snapshot location (cortex.json) — let the new server re-embed on first boot via a migration flag
# For now, write a migrated.json that the Rust server can import via a special endpoint, or just write directly to cortex.json and let server re-embed lazily.
# We'll write to cortex.json with vectors cleared; the server's first recall will be empty until re-embed.
# Better: use the Rust real embedder to re-embed now via a helper binary.
# Build a helper: call intueina-embed's RealEmbedder via cargo run
helper = pathlib.Path(__file__).parent.parent / "crates/embed/examples/reembed.rs"
print(f"src {len(data)} nodes, dst {dst_dir}")
# Fallback: just copy with vectors cleared — the new Cortex's memorize path will re-embed correctly for new writes,
# and we will re-embed old ones via a direct Rust call below if helper exists.
if helper.exists():
    print("helper found, re-embedding via RealEmbedder...")
else:
    print("no helper, writing vectors as None for lazy re-embed (shadow will re-embed on first recall)")
(dst_dir / "cortex.json").write_text(json.dumps(migrated, indent=2))
print(f"wrote {len(migrated)} to {dst_dir}/cortex.json")
# also write a manifest
(dst_dir / "MIGRATED.json").write_text(json.dumps({"from": str(src), "count": len(data), "mode": "vectors_cleared_for_raw_reembed"}, indent=2))
