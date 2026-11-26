// JS competitors: jsonrepair + ajv. Writes <out>/<tool>.jsonl and .timing.json.
//   node harness.mjs --tool ajv_coerce --corpus ../data/corpus.jsonl \
//     --schemas ../schemas --out ../results --reps 5
import { readFileSync, readdirSync, writeFileSync, mkdirSync } from "node:fs";
import { join, basename } from "node:path";
import { parseArgs } from "node:util";
import { jsonrepair } from "jsonrepair";
import Ajv2020 from "ajv/dist/2020.js";

const { values: args } = parseArgs({
  options: {
    tool: { type: "string" },
    corpus: { type: "string" },
    schemas: { type: "string" },
    out: { type: "string" },
    reps: { type: "string" },
  },
});

const AJV_OPTIONS = {
  ajv_strict: { allErrors: true, strict: false },
  // ajv's own correction features: type coercion (incl. scalar -> array),
  // defaults for missing properties, removal of additionalProperties: false.
  ajv_coerce: {
    allErrors: true,
    strict: false,
    coerceTypes: "array",
    useDefaults: true,
    removeAdditional: true,
  },
};
const options = AJV_OPTIONS[args.tool];
if (!options) throw new Error(`unknown tool ${args.tool}`);

const ajv = new Ajv2020.default(options);
const validators = {};
for (const file of readdirSync(args.schemas).filter((f) => f.endsWith(".json"))) {
  const schema = JSON.parse(readFileSync(join(args.schemas, file), "utf8"));
  validators[basename(file, ".json")] = ajv.compile(schema);
}

const cases = readFileSync(args.corpus, "utf8")
  .split("\n")
  .filter(Boolean)
  .map((line) => JSON.parse(line));

function run(c) {
  try {
    const value = JSON.parse(jsonrepair(c.raw));
    // ajv coerces in place, except at the root; wrap so the root can change too.
    const holder = { value };
    return validators[c.schema](holder.value, { parentData: holder, parentDataProperty: "value" })
      ? holder.value
      : null;
  } catch {
    return null;
  }
}

const outputs = cases.map(run); // also warm-up
const samples = [];
const reps = Number(args.reps);
for (let r = 0; r < reps; r++) {
  for (const c of cases) {
    const start = process.hrtime.bigint();
    run(c);
    samples.push(Number(process.hrtime.bigint() - start));
  }
}

mkdirSync(args.out, { recursive: true });
writeFileSync(
  join(args.out, `${args.tool}.jsonl`),
  cases.map((c, i) => JSON.stringify({ id: c.id, output: outputs[i] })).join("\n") + "\n",
);
const pkg = (name) =>
  JSON.parse(readFileSync(join("node_modules", name, "package.json"), "utf8")).version;
writeFileSync(
  join(args.out, `${args.tool}.timing.json`),
  JSON.stringify({
    tool: args.tool,
    language: "node",
    versions: { jsonrepair: pkg("jsonrepair"), ajv: pkg("ajv"), node: process.version },
    reps,
    bytes_per_pass: cases.reduce((n, c) => n + Buffer.byteLength(c.raw), 0),
    samples_ns: samples,
  }),
);
