"""Trap corruptions: a correct tool must NOT produce a schema-valid value,
except precision_int, whose exact value is representable as a Python int."""

import copy
import json

from corrupt import ENUM, INT, NUM, REQUIRED, UNKNOWN_ENUM, at, drop, put


def _precision_int(schema, truth, rng):
    if schema != "order":
        return None
    big = 98765432109876543210987 + rng.randint(0, 999)
    truth["order_id"] = big
    raw = copy.deepcopy(truth)
    raw["order_id"] = str(big)
    return truth, json.dumps(raw)


TRAPS = {
    "precision_int": _precision_int,
    "precision_float": at(NUM, lambda s, t, r, p: put(r, p, "19.990000000000000000001")),
    "fractional_int": at(INT, lambda s, t, r, p: put(r, p, "2.5")),
    "missing_required": at(REQUIRED, lambda s, t, r, p: drop(r, p)),
    "unfixable_type": at(INT, lambda s, t, r, p: put(r, p, "next week")),
    "enum_unknown": at(ENUM, lambda s, t, r, p: put(r, p, UNKNOWN_ENUM[s])),
}
EXPECT_TRUTH_TRAPS = {"precision_int"}
