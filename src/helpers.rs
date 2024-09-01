use std::time::SystemTime;

use chrono::{DateTime, Local};
use comfy_table::Cell;

pub const NOT_AVAILABLE: &str = "_";

pub trait UnwrapOrNa {
    fn unwrap_or_na(&self) -> String;
}

impl UnwrapOrNa for Option<Box<str>> {
    fn unwrap_or_na(&self) -> String {
        self.clone().as_deref().unwrap_or(NOT_AVAILABLE).to_string()
    }
}

pub trait IntoCell {
    fn into_cell(self) -> Cell;
}

impl IntoCell for String {
    fn into_cell(self) -> Cell {
        Cell::new(self)
    }
}


pub trait ToStringExt {
    fn to_string(self) -> Option<String>;
}
impl ToStringExt for Option<SystemTime> {
    fn to_string(self) -> Option<String> {
        self.map(DateTime::<Local>::from).map(|x| x.to_string())
    }
}

pub trait OptValueAsBoxStr {
    fn as_box_str(&self) -> Option<Box<str>>;
}

impl OptValueAsBoxStr for Option<&plist::Value> {
    fn as_box_str(&self) -> Option<Box<str>> {
        self.and_then(plist::Value::as_string).map(Box::from)
    }
}
