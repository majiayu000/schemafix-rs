//! rstructor types mirroring bench/schemas/*.json.

use rstructor::Instructor;
use serde::{Deserialize, Serialize};

#[derive(Instructor, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Celsius,
    Fahrenheit,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Weather {
    pub location: String,
    pub unit: Unit,
    pub days: u8,
    pub include_hourly: bool,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Customer {
    pub name: String,
    pub email: String,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Item {
    pub sku: String,
    pub quantity: u32,
    pub unit_price: f64,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Normal,
    High,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Order {
    pub order_id: u64,
    pub customer: Customer,
    pub items: Vec<Item>,
    pub priority: Priority,
    pub notes: Option<String>,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Organizer,
    Speaker,
    Attendee,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Person {
    pub name: String,
    pub role: Role,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Address {
    pub city: String,
    pub room: u32,
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Location {
    Name(String),
    Address(Address),
}

#[derive(Instructor, Serialize, Deserialize, Debug)]
pub struct Event {
    pub title: String,
    pub attendees: Vec<Person>,
    pub location: Location,
    pub all_day: bool,
    pub capacity: Option<u32>,
}
