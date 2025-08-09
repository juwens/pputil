use std::collections::BTreeMap;

pub type YamlValue = serde_yml::value::Value;
pub type YamlDocument = BTreeMap<String, Option<YamlValue>>;