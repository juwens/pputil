use std::{borrow::BorrowMut, fs::{self}};
use chrono::{DateTime, Local};
use der::Decode;
use comfy_table::Table;
use std::time::SystemTime;
use tap::{Conv, Pipe};

fn main() {
    let profiles_dir = dirs::home_dir().unwrap().join("Library/MobileDevice/Provisioning Profiles");

    let paths = fs::read_dir(profiles_dir).unwrap()
        .map(|dir_entry| dir_entry.unwrap().path())
        .filter_map(|path| {
            if path.extension().map_or(false, |ext| ext == "mobileprovision") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    println!();

    let rows = paths.into_iter().map(|path| {
        let pl = match parse_mobileprovision_into_plist(&path) {
            Ok(x) => x,
            Err(error) => panic!("Problem opening the file: {error:?}"),
        };

        let app_id_prefix = pl["ApplicationIdentifierPrefix"].as_array().unwrap();
        let exp_date = pl["ExpirationDate"].as_date().unwrap().conv::<SystemTime>().conv::<DateTime<Local>>();
        let entitlements = pl["Entitlements"].as_dictionary().unwrap();

        return vec![
            (*pl["AppIDName"].as_string().unwrap()).conv::<String>(),
            (*pl["IsXcodeManaged"].as_boolean().unwrap().to_string()).conv::<String>(),
            app_id_prefix.first().unwrap().as_string().unwrap().conv::<String>(),
            entitlements["application-identifier"].as_string().unwrap().conv::<String>(),
            exp_date.format("%Y-%m-%d").to_string().conv::<String>(),
            entitlements.get("com.apple.developer.team-identifier").unwrap().as_string().unwrap().conv::<String>(),
            path.file_name().unwrap().to_str().unwrap().conv::<String>(),
        ];
    }).collect::<Vec<_>>();

    let mut table = Table::new();
    table
        .set_header(vec!["AppIDName", "XC mgd", "ApplId Prefix", "ent: app identifier", "expir. date", "ent: team-identifier", "file"])
        .add_rows(rows);

    println!("{table}");

    println!();
}

fn parse_mobileprovision_into_plist(path: &std::path::PathBuf) -> Result<plist::Dictionary, Box<dyn std::error::Error>> {
    let file_bytes = fs::read(path)?;

    let mut reader = der::SliceReader::new(&file_bytes)?;

    let ci = cms::content_info::ContentInfo::decode(reader.borrow_mut())?;

    assert_eq!(ci.content_type.to_string(), oid_registry::OID_PKCS7_ID_SIGNED_DATA.to_string());
    let sd = ci.content.decode_as::<cms::signed_data::SignedData>()?;

    assert_eq!(sd.encap_content_info.econtent_type.to_string(), oid_registry::OID_PKCS7_ID_DATA.to_string());
    
    let content = &sd.encap_content_info.econtent.unwrap();

    let pl = content.value().pipe(plist::from_bytes::<plist::Dictionary>)?;
    
    return Ok(pl);
}
