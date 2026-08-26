#!/usr/bin/env python3
"""gate_exocortex.py — render exocortex.prima with live facts, run the player.

usage:
  python3 tools/gate_exocortex.py --target v04     # audit the old system
  python3 tools/gate_exocortex.py --target rs      # audit intueina-rs
  python3 tools/gate_exocortex.py --facts AUTH=1 ONEWRITER=1 ...   # manual

exit 0 = in-law (shippable), exit 1 = refused (the trace names the chamber).
"""

import argparse
import json
import subprocess
import sys
import tempfile
import urllib.request
from pathlib import Path

BASE = Path(__file__).resolve().parent.parent
LAW = BASE / "laws" / "exocortex.prima"
PRIMA = Path("/Users/arshbuttar/private/prima/engine/engine/prima")
if not PRIMA.exists():
    PRIMA = Path.home() / "private/prima/engine/engine/prima"

FACT_KEYS = ["AUTH", "ONEWRITER", "JOURNAL", "SPLITPATH", "GOLDEN",
             "DEGRADED", "CONFORMANCE", "CONF_LOADED", "PORT_ONE", "DRIFT", "RELEASE"]

CHAMBER_OF = {"AUTH": "mutation gated", "ONEWRITER": "one brain",
              "JOURNAL": "journaled memory", "SPLITPATH": "raw passages",
              "GOLDEN": "semantic recall", "DEGRADED": "truthful health",
              "CONFORMANCE": "promised surface", "CONF_LOADED": "loaded config",
              "PORT_ONE": "single endpoint", "DRIFT": "drift detected",
              "RELEASE": "release build"}


def probe_v04() -> dict:
    """Facts for the existing cli-rust v0.4 — from OBSERVER Phase A."""
    return {"AUTH": 0,        # F-004: unauthenticated delete/vacuum
            "ONEWRITER": 0,   # F-011: last-writer-wins data loss demonstrated
            "JOURNAL": 0,     # F-007: whole-blob JSON rewrite
            "SPLITPATH": 0,   # F-009: query prefix on passages
            "GOLDEN": 0,      # F-005: ADHD rule ranked 4th behind probes
            "DEGRADED": 0,    # F-006: unembedded node reported healthy
            "CONFORMANCE": 0, # F-003: memorize/think/review are 404
            "CONF_LOADED": 0, # F-001: ~/.intueina.toml never parsed
            "PORT_ONE": 0,    # F-002: three ports across four documents
            "DRIFT": 0,       # F-010: DREAMSWAP absent from code
            "RELEASE": 0}     # F-012: production runs target/debug


def probe_rs() -> dict:
    """Facts for the rebuild. Probes run live where possible; missing
    capabilities return 0 honestly rather than being skipped."""
    facts = {k: 0 for k in FACT_KEYS}
    port = 9321
    try:
        # boot handled externally by war-tests; here we only probe a running one
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=2) as r:
            health = json.load(r)
        facts["DEGRADED"] = 1 if health.get("status") == "healthy" else 0
    except Exception:
        pass
    return facts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", choices=["v04", "rs"])
    ap.add_argument("--facts", nargs="*", help="KEY=VALUE overrides")
    args = ap.parse_args()

    facts = (probe_v04() if args.target == "v04" else
             probe_rs() if args.target == "rs" else {})
    for kv in args.facts or []:
        k, _, v = kv.partition("=")
        facts[k.upper()] = int(v)

    missing = [k for k in FACT_KEYS if k not in facts]
    if missing:
        sys.exit(f"missing facts: {missing}")

    law = LAW.read_text()
    for k in FACT_KEYS:
        law = law.replace("{%s}" % k, str(facts[k]))

    with tempfile.NamedTemporaryFile("w", suffix=".prima", delete=False) as f:
        f.write(law)
        path = f.name

    r = subprocess.run([str(PRIMA), path], capture_output=True, text=True, timeout=10)
    print(r.stdout.strip())
    if r.returncode != 0 or "error:" in r.stderr:
        line = r.stderr.strip().splitlines()[-1] if r.stderr.strip() else "?"
        print(f"\nVERDICT: REFUSED [{line}]")
        zeros = [CHAMBER_OF[k] for k in FACT_KEYS if facts[k] == 0]
        if zeros:
            print("violated chambers: " + "; ".join(zeros))
        sys.exit(1)
    print("\nVERDICT: IN-LAW — shippable")
    sys.exit(0)


if __name__ == "__main__":
    main()
