"""Score every <results>/<tool>.jsonl against the corpus with one referee
validator (Python jsonschema, Draft 2020-12) and summarise timings.

    uv run --project bench/python python bench/score.py --results bench/results
"""

import argparse
import json
import statistics
import tomllib
from collections import defaultdict
from pathlib import Path

import jsonschema

BENCH = Path(__file__).parent


def same(a, b) -> bool:
    """JSON equality that keeps bool distinct from numbers."""
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return type(a) is type(b) and a == b


def rust_versions(names):
    lock = tomllib.loads((BENCH / "rust" / "Cargo.lock").read_text())
    found = {p["name"]: p["version"] for p in lock["package"]}
    return {n: found.get(n, "?") for n in names}


def percentile(sorted_ns, q):
    return sorted_ns[min(len(sorted_ns) - 1, int(q * len(sorted_ns)))]


def score_tool(path, cases, validators):
    outputs = {r["id"]: r["output"] for r in map(json.loads, path.open())}
    totals = defaultdict(int)
    per_category = defaultdict(lambda: [0, 0, 0])  # correct, wrong, total
    for case in cases:
        out = outputs[case["id"]]
        produced = out is not None and validators[case["schema"]].is_valid(out)
        if case["expect"] == "value":
            totals["value_cases"] += 1
            totals["passed"] += produced
            ok = produced and same(out, case["expected"])
        else:
            totals["reject_cases"] += 1
            ok = not produced
        wrong = produced and not ok
        totals["correct"] += ok
        totals["wrong_fix"] += wrong
        cat = per_category[(case["group"], case["category"])]
        cat[0] += ok
        cat[1] += wrong
        cat[2] += 1
    return dict(totals), {f"{g}/{c}": v for (g, c), v in per_category.items()}


def by_group(samples, cases):
    groups = defaultdict(list)
    for i, sample in enumerate(samples):
        groups[cases[i % len(cases)]["group"]].append(sample)
    return {
        g: {"p50_us": percentile(sorted(v), 0.5) / 1e3, "p99_us": percentile(sorted(v), 0.99) / 1e3}
        for g, v in groups.items()
    }


def timing_summary(path, cases):
    t = json.loads(path.read_text())
    ns = sorted(t["samples_ns"])
    total = sum(ns)
    versions = t["versions"]
    if t["language"] == "rust":
        versions = rust_versions(versions)
    return {
        "language": t["language"],
        "versions": versions,
        "reps": t["reps"],
        "calls": len(ns),
        "ops_per_sec": len(ns) / total * 1e9,
        "mb_per_sec": t["bytes_per_pass"] * t["reps"] / total * 1e3,
        "p50_us": percentile(ns, 0.50) / 1e3,
        "p99_us": percentile(ns, 0.99) / 1e3,
        "mean_us": statistics.fmean(ns) / 1e3,
        "by_group": by_group(t["samples_ns"], cases),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, default=BENCH / "data" / "corpus.jsonl")
    args = parser.parse_args()

    cases = [json.loads(line) for line in args.corpus.open()]
    schemas = {p.stem: json.loads(p.read_text()) for p in (BENCH / "schemas").glob("*.json")}
    validators = {k: jsonschema.Draft202012Validator(v) for k, v in schemas.items()}

    summary = {}
    for path in sorted(args.results.glob("*.jsonl")):
        totals, categories = score_tool(path, cases, validators)
        summary[path.stem] = {
            "totals": totals,
            "categories": categories,
            "timing": timing_summary(path.with_suffix(".timing.json"), cases),
        }
    (args.results / "summary.json").write_text(json.dumps(summary, indent=2))
    for tool, s in summary.items():
        t, tm = s["totals"], s["timing"]
        print(
            f"{tool:24} pass {t['passed']}/{t['value_cases']}  correct {t['correct']}/{len(cases)}"
            f"  wrong {t['wrong_fix']}  {tm['ops_per_sec']:>9.0f} ops/s  {tm['mb_per_sec']:7.2f} MB/s"
            f"  p50 {tm['p50_us']:8.1f}us  p99 {tm['p99_us']:8.1f}us"
        )


if __name__ == "__main__":
    main()
