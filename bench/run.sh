#!/usr/bin/env bash
# Reproduce the whole benchmark:  bench/run.sh <label> [reps]
# Results land in bench/results/<label>/ (raw outputs, timings, summary.json, env.json).
set -euo pipefail

LABEL="${1:?usage: bench/run.sh <label> [reps]}"
REPS="${2:-5}"
SEED=20261126
PER_SCHEMA=30
BENCH="$(cd "$(dirname "$0")" && pwd)"
OUT="$BENCH/results/$LABEL"
CORPUS="$BENCH/data/corpus.jsonl"
SCHEMAS="$BENCH/schemas"
mkdir -p "$OUT"

(cd "$BENCH/python" && uv sync -q)
(cd "$BENCH/node" && npm ci --silent)
(cd "$BENCH/rust" && cargo build --release -q)

(cd "$BENCH/corpus" && uv run --project "$BENCH/python" python gen.py \
  --seed "$SEED" --per-schema "$PER_SCHEMA" --out "$CORPUS")

for tool in schemafix jsonrepair_jsonschema rstructor_mock rstructor_decode; do
  echo "== $tool"
  "$BENCH/rust/target/release/schemafix-bench" "$tool" "$CORPUS" "$SCHEMAS" "$OUT" "$REPS"
done
for tool in json_repair_schema json_repair_salvage json_repair_plain instructor; do
  echo "== $tool"
  (cd "$BENCH/python" && uv run python harness.py --tool "$tool" --corpus "$CORPUS" \
    --schemas "$SCHEMAS" --out "$OUT" --reps "$REPS")
done
for tool in ajv_strict ajv_coerce; do
  echo "== $tool"
  (cd "$BENCH/node" && node harness.mjs --tool "$tool" --corpus "$CORPUS" \
    --schemas "$SCHEMAS" --out "$OUT" --reps "$REPS")
done

cat > "$OUT/env.json" <<JSON
{
  "machine": "$(sysctl -n hw.model 2>/dev/null || uname -m)",
  "cpu": "$(sysctl -n machdep.cpu.brand_string 2>/dev/null || uname -p)",
  "cores": "$(sysctl -n hw.ncpu 2>/dev/null || nproc)",
  "memory_bytes": "$(sysctl -n hw.memsize 2>/dev/null || echo unknown)",
  "os": "$(uname -sr)",
  "rustc": "$(rustc --version)",
  "python": "$(cd "$BENCH/python" && uv run python --version)",
  "node": "$(node --version)",
  "seed": $SEED,
  "per_schema": $PER_SCHEMA,
  "reps": $REPS,
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
JSON

(cd "$BENCH" && uv run --project python python score.py --results "$OUT" --corpus "$CORPUS")
