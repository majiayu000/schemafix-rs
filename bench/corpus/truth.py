"""Random ground-truth instances that satisfy each schema in bench/schemas/."""

import random

CITIES = ["Paris", "Oslo", "Lima", "Kyoto", "Austin", "Nairobi", "Lisbon", "Hanoi"]
NAMES = ["Ada Park", "Ben Okafor", "Chen Wei", "Dana Ruiz", "Eli Novak", "Fay Moreau"]
WORDS = ["leave at door", "gift wrap", "fragile", "call on arrival", "rush"]
TITLES = ["Team sync", "Design review", "Quarterly planning", "Launch party"]


def weather(rng: random.Random) -> dict:
    return {
        "location": rng.choice(CITIES),
        "unit": rng.choice(["celsius", "fahrenheit"]),
        "days": rng.randint(1, 14),
        "include_hourly": rng.choice([True, False]),
    }


def order(rng: random.Random) -> dict:
    name = rng.choice(NAMES)
    items = [
        {
            "sku": f"SKU-{rng.randint(1000, 9999)}",
            "quantity": rng.randint(1, 9),
            "unit_price": rng.randint(100, 99999) / 100,
        }
        for _ in range(rng.randint(1, 3))
    ]
    return {
        "order_id": rng.randint(1, 10**6),
        "customer": {"name": name, "email": name.split()[0].lower() + "@example.com"},
        "items": items,
        "priority": rng.choice(["low", "normal", "high"]),
        "notes": rng.choice(WORDS + [None]),
    }


def event(rng: random.Random) -> dict:
    attendees = [
        {"name": rng.choice(NAMES), "role": rng.choice(["organizer", "speaker", "attendee"])}
        for _ in range(rng.randint(1, 3))
    ]
    location = (
        f"Room {rng.choice('ABCD')}"
        if rng.random() < 0.5
        else {"city": rng.choice(CITIES), "room": rng.randint(100, 999)}
    )
    return {
        "title": rng.choice(TITLES),
        "attendees": attendees,
        "location": location,
        "all_day": rng.choice([True, False]),
        "capacity": rng.choice([rng.randint(5, 500), None]),
    }


GENERATORS = {"weather": weather, "order": order, "event": event}
