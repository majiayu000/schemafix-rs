"""Generate the synthetic benchmark corpus.

    uv run --project bench/python python bench/corpus/gen.py --seed 20261126 --per-schema 30

Every record is synthetic: a random valid instance (truth.py) plus one
corruption (corrupt.py). Output: bench/data/corpus.jsonl
"""

import argparse
import copy
import json
import random
from pathlib import Path

from corrupt import SCHEMA_FIXES, SYNTAX
from traps import EXPECT_TRUTH_TRAPS, TRAPS
from truth import GENERATORS

GROUPS = {"syntax": SYNTAX, "schema": SCHEMA_FIXES, "trap": TRAPS}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--per-schema", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    rng = random.Random(args.seed)
    records = []
    for schema, generate in GENERATORS.items():
        for base in range(args.per_schema):
            truth = generate(rng)
            for group, table in GROUPS.items():
                for category, corrupt in table.items():
                    produced = corrupt(schema, copy.deepcopy(truth), rng)
                    if produced is None:
                        continue
                    expected, raw = produced
                    reject = group == "trap" and category not in EXPECT_TRUTH_TRAPS
                    records.append(
                        {
                            "id": f"{schema}-{base:03d}-{category}",
                            "schema": schema,
                            "group": group,
                            "category": category,
                            "raw": raw,
                            "expect": "reject" if reject else "value",
                            "expected": None if reject else expected,
                        }
                    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w") as fh:
        for record in records:
            fh.write(json.dumps(record) + "\n")
    print(f"wrote {len(records)} records to {args.out} (seed={args.seed})")


if __name__ == "__main__":
    main()
