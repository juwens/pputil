use std::{path::Path, time::SystemTime};
use comfy_table::Cell;
use crate::YamlDocument;

#[derive(Debug)]
pub struct PrivisionFileData {
    pub app_id_name: Option<Box<str>>,
    pub name: Option<Box<str>>,
    pub team_name: Option<Box<str>>,
    /// is Xcode managed
    pub xc_managed: Option<bool>,
    pub app_id_prefixes: Option<Vec<Box<str>>>,
    /// expiration date
    pub exp_date: Option<SystemTime>,
    /// entitlements.application-identifier
    pub ent_app_id: Option<Box<str>>,
    pub provisioned_devices: Option<usize>,
    pub file_path: Box<Path>,
    pub local_provision: Option<bool>,
    pub uuid: Option<Box<str>>,
    pub properties: YamlDocument,
    #[allow(dead_code)]
    pub creation_date: Option<SystemTime>,
    #[allow(dead_code)]
    pub ent_team_id: Option<Box<str>>,
    #[allow(dead_code)]
    pub platforms: Option<Vec<Box<str>>>,
}

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

pub trait OptValueAsBoxStr {
    fn as_box_str(&self) -> Option<Box<str>>;
}

impl OptValueAsBoxStr for Option<&plist::Value> {
    fn as_box_str(&self) -> Option<Box<str>> {
        self.and_then(plist::Value::as_string).map(Box::from)
    }
}
