"""Python competitors. Writes <out>/<tool>.jsonl and <out>/<tool>.timing.json.

    uv run python harness.py --tool json_repair_schema --corpus ../data/corpus.jsonl \
        --schemas ../schemas --out ../results --reps 5
"""

import argparse
import json
import time
from importlib.metadata import version
from pathlib import Path

import json_repair
import jsonschema
from instructor.v2.core.json import extract_json_from_codeblock

from models import MODELS


def json_repair_schema(mode):
    def run(raw, schema, validator, model):
        return json_repair.loads(raw, schema=schema, schema_repair_mode=mode)

    return run


def json_repair_plain(raw, schema, validator, model):
    value = json_repair.loads(raw)
    return value if validator.is_valid(value) else None


def instructor_layer(raw, schema, validator, model):
    # instructor JSON mode: extract the JSON span, json.loads(strict=False),
    # then pydantic lax validation (instructor.v2.core.function_calls).
    parsed = json.loads(extract_json_from_codeblock(raw), strict=False)
    return model.model_validate(parsed, strict=False).model_dump(mode="json")


TOOLS = {
    "json_repair_schema": (json_repair_schema("standard"), ["json-repair", "jsonschema"]),
    "json_repair_salvage": (json_repair_schema("salvage"), ["json-repair", "jsonschema"]),
    "json_repair_plain": (json_repair_plain, ["json-repair", "jsonschema"]),
    "instructor": (instructor_layer, ["instructor", "pydantic"]),
}


def call(fn, case, schema, validator, model):
    try:
        return fn(case["raw"], schema, validator, model)
    except Exception:  # every failure mode counts as "no value"
        return None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tool", choices=TOOLS, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--schemas", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--reps", type=int, required=True)
    args = parser.parse_args()

    fn, packages = TOOLS[args.tool]
    cases = [json.loads(line) for line in args.corpus.open()]
    schemas = {p.stem: json.loads(p.read_text()) for p in args.schemas.glob("*.json")}
    validators = {k: jsonschema.Draft202012Validator(v) for k, v in schemas.items()}

    def ctx(case):
        name = case["schema"]
        return schemas[name], validators[name], MODELS[name]

    outputs = [call(fn, case, *ctx(case)) for case in cases]  # also warm-up
    samples = []
    for _ in range(args.reps):
        for case in cases:
            c = ctx(case)
            start = time.perf_counter_ns()
            call(fn, case, *c)
            samples.append(time.perf_counter_ns() - start)

    args.out.mkdir(parents=True, exist_ok=True)
    with (args.out / f"{args.tool}.jsonl").open("w") as fh:
        for case, output in zip(cases, outputs):
            fh.write(json.dumps({"id": case["id"], "output": output}) + "\n")
    timing = {
        "tool": args.tool,
        "language": "python",
        "versions": {p: version(p) for p in packages},
        "reps": args.reps,
        "bytes_per_pass": sum(len(c["raw"].encode()) for c in cases),
        "samples_ns": samples,
    }
    (args.out / f"{args.tool}.timing.json").write_text(json.dumps(timing))


if __name__ == "__main__":
    main()
