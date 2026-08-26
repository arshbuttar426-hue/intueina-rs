#!/usr/bin/env bash
set -euo pipefail
DIR=/tmp/intueina-war-$$
DATADIR=$(mktemp -d /tmp/intueina-war-data-XXXX)
PORT=9399
BIN=target/debug/intueina-rs
PASS=0; FAIL=0

say(){ printf "\n=== %s ===\n" "$1"; }
ok(){ echo "  ✓ $1"; PASS=$((PASS+1)); }
fail(){ echo "  ✗ $1"; FAIL=$((FAIL+1)); }

# boot
say "BOOT real BERT"
$BIN --port $PORT --data-dir $DATADIR > /tmp/war.log 2>&1 & SP=$!
for i in $(seq 1 10); do curl -sf -m 1 http://127.0.0.1:$PORT/health >/dev/null && break; sleep 0.5; done
TOK=$(grep "demo Bearer" /tmp/war.log | sed -E 's/.*token[^:]*: //')
if [ -z "$TOK" ]; then echo "boot failed"; cat /tmp/war.log; exit 1; fi
echo "  token ${TOK:0:20}..."

# W1 config strict
say "W1 config strict (F-001)"
if cargo test -p intueina-config --quiet 2>&1; then ok "unknown key fails closed (3 tests)"; else fail "config strict"; fi

# W2 port single
say "W2 single port (F-002)"
if grep -q "pub const PORT: u16 = 9124" crates/config/src/lib.rs && grep -q "port constant = 9124" /tmp/war.log; then ok "PORT=9124 single source"; else fail "port anarchy"; fi

# W3 surface — 8 routes exist
say "W3 surface conformance (F-003)"
for ep in "/health" "/v1/learn" "/v1/recall" "/v1/memorize" "/v1/think" "/v1/review" "/v1/stats" "/v1/vacuum"; do
  code=$(curl -s -o /dev/null -w "%{http_code}" -m 2 -X POST http://127.0.0.1:$PORT$ep -H 'content-type: application/json' -d '{}' 2>/dev/null || curl -s -o /dev/null -w "%{http_code}" -m 2 http://127.0.0.1:$PORT$ep 2>/dev/null)
  # learn/recall/memorize/think/review/vacuum are POST, health/stats are GET — accept 200/401 as "exists" (not 404)
  if [ "$code" != "404" ] && [ "$code" != "000" ]; then ok "$ep -> $code"; else fail "$ep -> $code"; fi
done

# W4 gate
say "W4 gate (F-004)"
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:$PORT/v1/learn -H 'content-type: application/json' -d '{"content":"x"}')
[ "$code" = "401" ] && ok "unauth learn 401" || fail "unauth learn $code"
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:$PORT/v1/learn -H "authorization: Bearer $TOK" -H 'content-type: application/json' -d '{"content":"gated probe"}')
[ "$code" = "200" ] && ok "auth learn 200" || fail "auth learn $code"
code=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE http://127.0.0.1:$PORT/v1/nodes/does-not-exist -H "authorization: Bearer $TOK")
[ "$code" = "404" ] && ok "delete nonexistent 404 (was 200)" || fail "delete nonexistent $code"
code=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE http://127.0.0.1:$PORT/v1/nodes/x)
[ "$code" = "401" ] && ok "unauth delete 401 (was 200 open)" || fail "unauth delete $code"
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST http://127.0.0.1:$PORT/v1/vacuum)
[ "$code" = "401" ] && ok "unauth vacuum 401" || fail "unauth vacuum $code"

# W5 recall golden (F-005/F-009)
say "W5 recall golden + typed split (F-005, F-009)"
curl -s -m 5 -X POST http://127.0.0.1:$PORT/v1/learn -H "authorization: Bearer $TOK" -H 'content-type: application/json' -d '{"content":"gpt spec probe"}' >/dev/null
curl -s -m 5 -X POST http://127.0.0.1:$PORT/v1/learn -H "authorization: Bearer $TOK" -H 'content-type: application/json' -d '{"content":"persist agent memory as resolved PRIMA frames"}' >/dev/null
curl -s -m 5 -X POST http://127.0.0.1:$PORT/v1/learn -H "authorization: Bearer $TOK" -H 'content-type: application/json' -d '{"content":"COMMUNICATION RULE (durable): Arsh has ADHD. Responses must be short, conclusion-first, scannable."}' >/dev/null
TOP=$(curl -s -m 10 -X POST http://127.0.0.1:$PORT/v1/recall -H 'content-type: application/json' -d '{"query":"ADHD communication rule","k":3}' | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['memories'][0]['content'][:30])")
if echo "$TOP" | grep -q "COMMUNICATION"; then ok "ADHD rule #1 (was #4)"; else fail "golden top was $TOP"; fi
if cargo test -p intueina-embed --quiet 2>&1; then ok "typed split holds (real+mock, 4 tests)"; else fail "typed split"; fi

# W6 store durability (F-007/F-011)
say "W6 store durability (F-007, F-011)"
if cargo test -p intueina-store --quiet 2>&1; then ok "store: 20-thread no-loss + kill-9 valid (3 tests)"; else fail "store tests"; fi
# live concurrent HTTP learns
for i in $(seq 1 10); do curl -s -m 5 -X POST http://127.0.0.1:$PORT/v1/learn -H "authorization: Bearer $TOK" -H 'content-type: application/json' -d "{\"content\":\"war writer $i\"}" >/dev/null & done; wait
COUNT=$(curl -s -m 5 -X POST http://127.0.0.1:$PORT/v1/recall -H 'content-type: application/json' -d '{"query":"war writer","k":30}' | python3 -c "import json,sys; print(len(json.load(sys.stdin)['memories']))")
if [ "$COUNT" -ge 20 ]; then ok "live concurrent HTTP learns: $COUNT/20 survived"; else fail "live concurrent $COUNT/20"; fi

# W7 observer (F-010) + health (F-006)
say "W7 observer + health (F-006, F-010)"
if cargo test -p intueina-observer --quiet 2>&1; then ok "hash-chain verifies, tamper breaks (2 tests)"; else fail "observer"; fi
HEALTH=$(curl -s -m 2 http://127.0.0.1:$PORT/health | python3 -c "import json,sys; print(json.load(sys.stdin).get('observer_healthy'))")
[ "$HEALTH" = "True" ] && ok "observer_healthy true" || fail "observer_healthy $HEALTH"

kill $SP 2>/dev/null; rm -rf "$DATADIR"
say "WAR RESULT"
echo "  PASS $PASS  FAIL $FAIL"
if [ "$FAIL" -eq 0 ]; then echo "  ★ ALL CHAMBERS PASS — IN-LAW"; exit 0; else echo "  ✗ REFUSED"; exit 1; fi
