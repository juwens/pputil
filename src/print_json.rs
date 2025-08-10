use std::{rc::Rc, time::SystemTime};

use crate::{helpers::{ProvisioningProfileFileData, UnwrapOrNa}, types::ProfilesCollection};
use chrono::{DateTime, Local};
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;

pub fn print_json(profiles_unsorted: ProfilesCollection) {
    print_json_internal(profiles_unsorted).unwrap();
}

pub fn print_json_internal(profiles_unsorted: ProfilesCollection) -> serde_json::Result<()> {
    let mut outer_obj = serde_json::Map::new();

    for profile_ptr in profiles_unsorted {
        if profile_ptr.is_ok() {
            let profile = profile_ptr.unwrap().clone();
            let provile_value = serde_json::to_value(profile.as_ref())?;

            outer_obj.insert(profile.file_path.to_string_lossy().into(), provile_value);

        }
    }
    
    let json = serde_json::to_string_pretty(&Value::Object(outer_obj))?;
    println!("{json}");

    Ok(())
}

impl Serialize for ProvisioningProfileFileData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("ProvisioningProfile", 20)?;

        s.serialize_field("AppIDName", &self.app_id_name.unwrap_or_na())?;
        s.serialize_field("ApplicationIdentifierPrefix", &un_rc_str(&self.app_id_prefixes.clone().unwrap_or_default()))?;
        s.serialize_field("CreationDate", &self.creation_date.map(systemtime_to_iso))?;
        s.serialize_field("Platform", &un_rc_str(&self.platforms.clone().unwrap_or_default()))?;
        s.serialize_field("IsXcodeManaged", &self.xc_managed)?;
        // s.serialize_field("developer_certificates", &self.developer_certificates)?;
        // s.serialize_field("DER-Encoded-Profile", &self.platforms)?;
        s.serialize_field("Entitlements", &self.entitlements_raw)?;
        s.serialize_field("ExpirationDate", &self.exp_date.map(systemtime_to_iso))?;
        s.serialize_field("Name", &self.name.unwrap_or_na())?;
        s.serialize_field("ProvisionedDevices", &un_rc_str(&self.provisioned_devices))?;
        s.serialize_field("TeamIdentifier", &self.team_identifier)?;
        s.serialize_field("TeamName", &self.team_name.unwrap_or_na())?;
        s.serialize_field("TimeToLive", &self.time_to_live)?;
        s.serialize_field("UUID", &self.uuid.unwrap_or_na())?;
        s.serialize_field("Version", &self.version)?;

        s.end()
    }
}

fn systemtime_to_iso(t: SystemTime) -> String {
    let dt: DateTime<Local> = t.into();
    dt.to_rfc3339() // e.g. "2025-08-10T14:23:55+00:00"
}

fn un_rc_str(v: &[Rc<str>]) -> Vec<String> {
    v.iter().map(|rc| rc.to_string()).collect()
}