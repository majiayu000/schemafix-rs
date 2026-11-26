"""Synthetic corruptions. Each returns (truth, raw_text) or None when it does
not apply to the schema. `truth` may be adjusted first (e.g. forced to null)
so the corruption is well defined."""

import copy
import json
import re

INT = {"weather": ["days"], "order": ["items", 0, "quantity"], "event": ["capacity"]}
NUM = {"order": ["items", 0, "unit_price"]}
BOOL = {"weather": ["include_hourly"], "event": ["all_day"]}
ENUM = {"weather": ["unit"], "order": ["priority"], "event": ["attendees", 0, "role"]}
UNKNOWN_ENUM = {"weather": "kelvin", "order": "urgent", "event": "guest"}
SINGLETON = {"order": "items", "event": "attendees"}
# order.notes is ["string","null"]: the string "null" is already valid, so
# its intended value is ambiguous and it is excluded.
NULLABLE = {"event": ["capacity"]}
DEFAULTED = {"weather": ("include_hourly", False), "order": ("priority", "normal")}
NUM_FOR_STR = {"weather": (["location"], 10001), "event": (["title"], 2026)}
REQUIRED = {"weather": ["location"], "order": ["customer", "email"], "event": ["title"]}


def get(doc, path):
    for key in path:
        doc = doc[key]
    return doc


def put(doc, path, value):
    get(doc, path[:-1])[path[-1]] = value


def drop(doc, path):
    del get(doc, path[:-1])[path[-1]]


def at(table, mutate):
    """Corruption that rewrites one field of the raw copy."""

    def corrupt(schema, truth, rng):
        if schema not in table:
            return None
        if schema == "event" and table in (INT,):
            truth["capacity"] = truth["capacity"] or rng.randint(5, 500)
        raw = copy.deepcopy(truth)
        mutate(schema, truth, raw, table[schema])
        return truth, json.dumps(raw)

    return corrupt


def text(transform):
    return lambda schema, truth, rng: (truth, transform(truth))


def _trailing_commas(v):
    return re.sub(r'(?<=[^\[{,\s])(\s*)([}\]])', r",\1\2", json.dumps(v, indent=2))


def _python_literals(v):
    s = json.dumps(v)
    return s.replace("true", "True").replace("false", "False").replace("null", "None")


def _truncated(v):
    return json.dumps(v).rstrip("}]")


def _commented(v):
    lines = json.dumps(v, indent=2).split("\n")
    return "// tool arguments\n" + lines[0] + " /* begin */\n" + "\n".join(lines[1:])


SYNTAX = {
    "clean": text(json.dumps),
    "code_fence": text(lambda v: "```json\n" + json.dumps(v, indent=2) + "\n```"),
    "prose_fence": text(
        lambda v: "Here is the result:\n```json\n" + json.dumps(v, indent=2)
        + "\n```\nLet me know if you need anything else."
    ),
    "prose_inline": text(lambda v: "Sure! The arguments are: " + json.dumps(v) + " Hope this helps."),
    "trailing_comma": text(_trailing_commas),
    "single_quotes": text(lambda v: json.dumps(v).replace('"', "'")),
    "unquoted_keys": text(lambda v: re.sub(r'"(\w+)":', r"\1:", json.dumps(v))),
    "python_literals": text(_python_literals),
    "truncated": text(_truncated),
    "comments": text(_commented),
}


def _singleton(schema, truth, rng):
    if schema not in SINGLETON:
        return None
    key = SINGLETON[schema]
    truth[key] = truth[key][:1]
    raw = copy.deepcopy(truth)
    raw[key] = raw[key][0]
    return truth, json.dumps(raw)


def _null_string(schema, truth, rng):
    if schema not in NULLABLE:
        return None
    put(truth, NULLABLE[schema], None)
    raw = copy.deepcopy(truth)
    put(raw, NULLABLE[schema], "null")
    return truth, json.dumps(raw)


def _missing_default(schema, truth, rng):
    if schema not in DEFAULTED:
        return None
    key, default = DEFAULTED[schema]
    truth[key] = default
    raw = copy.deepcopy(truth)
    del raw[key]
    return truth, json.dumps(raw)


def _extra(schema, truth, rng):
    raw = copy.deepcopy(truth)
    raw["confidence"] = 0.9
    return truth, json.dumps(raw)


def _num_for_str(schema, truth, rng):
    if schema not in NUM_FOR_STR:
        return None
    path, number = NUM_FOR_STR[schema]
    put(truth, path, str(number))
    raw = copy.deepcopy(truth)
    put(raw, path, number)
    return truth, json.dumps(raw)


def _anyof(schema, truth, rng):
    if schema != "event":
        return None
    truth["location"] = {"city": "Oslo", "room": rng.randint(100, 999)}
    raw = copy.deepcopy(truth)
    raw["location"]["room"] = str(truth["location"]["room"])
    return truth, json.dumps(raw)


STRINGIFIED = {"order": "customer", "event": "attendees"}


def _stringified(schema, truth, rng):
    """Nested object/array emitted as a JSON string (common in tool-call arguments)."""
    if schema not in STRINGIFIED:
        return None
    raw = copy.deepcopy(truth)
    raw[STRINGIFIED[schema]] = json.dumps(raw[STRINGIFIED[schema]])
    return truth, json.dumps(raw)


def _combined(schema, truth, rng):
    truth, raw_text = at(INT, lambda s, t, r, p: put(r, p, str(get(t, p))))(schema, truth, rng)
    raw = json.loads(raw_text)
    put(raw, ENUM[schema], get(truth, ENUM[schema]).upper())
    return truth, "Result:\n```json\n" + json.dumps(raw, indent=2) + "\n```"


SCHEMA_FIXES = {
    "int_as_string": at(INT, lambda s, t, r, p: put(r, p, str(get(t, p)))),
    "number_as_string": at(NUM, lambda s, t, r, p: put(r, p, repr(get(t, p)))),
    "bool_as_string": at(BOOL, lambda s, t, r, p: put(r, p, str(get(t, p)).lower())),
    "enum_case": at(ENUM, lambda s, t, r, p: put(r, p, get(t, p).capitalize())),
    "singleton_array": _singleton,
    "null_as_string": _null_string,
    "missing_default": _missing_default,
    "extra_property": _extra,
    "number_for_string": _num_for_str,
    "anyof_branch": _anyof,
    "stringified_json": _stringified,
    "combined": _combined,
}
