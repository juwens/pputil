use crate::{yml_types::YamlDocument, CertDetails};
use comfy_table::Cell;
use serde_json::{map, Value};
use std::{env, path::{Path, PathBuf}, rc::Rc, time::SystemTime};

#[derive(Debug)]
pub struct ProvisioningProfileFileData {
    pub app_id_name: Option<Rc<str>>,
    pub name: Option<Rc<str>>,
    // pub team_identifier: Option<Rc<str>>,
    /// is Xcode managed
    pub xc_managed: Option<bool>,
    pub xc_kind: Option<Rc<str>>,
    pub app_id_prefixes: Option<Vec<Rc<str>>>,
    /// expiration date
    pub exp_date: Option<SystemTime>,
    /// entitlements.application-identifier
    pub ent_app_id: Option<Rc<str>>,
    pub provisioned_devices: Vec<Rc<str>>,
    pub provisioned_devices_count: Option<usize>,
    pub file_path: Rc<Path>,
    pub local_provision: Option<bool>,
    pub properties: YamlDocument,
    #[allow(dead_code)]
    pub creation_date: Option<SystemTime>,
    #[allow(dead_code)]
    pub ent_team_id: Option<Rc<str>>,
    pub entitlements_raw: Option<map::Map<String, Value>>,
    #[allow(dead_code)]
    pub platforms: Option<Vec<Rc<str>>>,
    pub developer_certificates_raw: Vec<Vec<u8>>,
    pub developer_certificates: Vec<CertDetails>,
    pub team_identifier: Option<Vec<String>>,
    pub team_name: Option<Rc<str>>,
    pub time_to_live: Option<i64>,
    pub uuid: Option<Rc<str>>,
    pub version: Option<i64>,
}

pub const NOT_AVAILABLE: &str = "_";

pub trait UnwrapOrNa {
    fn unwrap_or_na(&self) -> String;
}

impl UnwrapOrNa for Option<Rc<str>> {
    fn unwrap_or_na(&self) -> String {
        self.clone().as_deref().unwrap_or(NOT_AVAILABLE).to_string()
    }
}

impl UnwrapOrNa for Option<String> {
    fn unwrap_or_na(&self) -> String {
        self.clone().as_deref().unwrap_or(NOT_AVAILABLE).to_string()
    }
}

impl UnwrapOrNa for Option<&str> {
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
    fn as_arc_str(&self) -> Option<Rc<str>>;
}

impl OptValueAsBoxStr for Option<&plist::Value> {
    fn as_arc_str(&self) -> Option<Rc<str>> {
        self.and_then(plist::Value::as_string).map(Rc::<str>::from)
    }
}

pub fn encode_to_yaml_str(value: &YamlDocument) -> String {
    serde_yml::to_string(&value).unwrap()
}

pub fn abbreviate_home(path: &Path) -> PathBuf {
    if let Some(home) = env::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return PathBuf::from("~").join(stripped);
        }
    }
    path.to_path_buf()
}

pub fn abbreviate_home_arc(path: Rc<Path>) -> PathBuf {
    if let Some(home) = env::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return PathBuf::from("~").join(stripped);
        }
    }
    path.to_path_buf()
}