"""Pydantic models mirroring bench/schemas/*.json (for the instructor harness)."""

from typing import Literal, Optional, Union

from pydantic import BaseModel, Field


class Weather(BaseModel):
    location: str
    unit: Literal["celsius", "fahrenheit"] = "celsius"
    days: int = Field(ge=1, le=14)
    include_hourly: bool = False


class Customer(BaseModel):
    name: str
    email: str


class Item(BaseModel):
    sku: str
    quantity: int = Field(ge=1)
    unit_price: float


class Order(BaseModel):
    order_id: int
    customer: Customer
    items: list[Item] = Field(min_length=1)
    priority: Literal["low", "normal", "high"] = "normal"
    notes: Optional[str]


class Person(BaseModel):
    name: str
    role: Literal["organizer", "speaker", "attendee"]


class Address(BaseModel):
    city: str
    room: int


class Event(BaseModel):
    title: str
    attendees: list[Person]
    location: Union[str, Address]
    all_day: bool
    capacity: Optional[int]


MODELS = {"weather": Weather, "order": Order, "event": Event}
