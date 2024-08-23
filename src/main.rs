use std::{borrow::BorrowMut, collections::BTreeMap};
use std::fs::{self};
use chrono::{DateTime, Local};
use der::Decode;
use std::time::SystemTime;
use tap::{Conv, Pipe};

type YamlValue = serde_yml::value::Value;
type YamlDocument = BTreeMap<String, YamlValue>;

struct Row {
    app_id_name: String,
    is_xc_managed: bool,
    app_id_prefixes: Vec<String>,
    entitlements: YamlDocument,
    exp_date: String,
    file_name: String,
    misc: YamlDocument,
}

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
        });

    println!();

    let rows = paths.map(|path| {
        let pl = match parse_mobileprovision_into_plist(&path) {
            Ok(x) => x,
            Err(error) => panic!("Problem opening the file: {error:?}"),
        };

        return Row {
            app_id_name: pl["AppIDName"].as_string().unwrap().into(),
            is_xc_managed: pl["IsXcodeManaged"].as_boolean().unwrap(),
            app_id_prefixes: {
                let prefixes  = pl["ApplicationIdentifierPrefix"].as_array().unwrap();
                prefixes.iter().map(|x| x.as_string().unwrap().to_owned()).collect()
            },
            entitlements: {
                let entitlements = pl["Entitlements"].as_dictionary().unwrap();
                let mut entitlements_out = new_yaml_dict();
                entitlements_out.insert("app_id".into(), entitlements["application-identifier"].as_string().unwrap().into());
                entitlements_out.insert("team_id".into(), entitlements["com.apple.developer.team-identifier"].as_string().unwrap().into());
                entitlements_out
            },
            exp_date: pl["ExpirationDate"].as_date().unwrap().conv::<SystemTime>().conv::<DateTime<Local>>().format("%Y-%m-%d").to_string(),
            file_name: path.file_name().unwrap().to_str().unwrap().into(),
            misc: {
                let mut misc = new_yaml_dict();
                misc.insert("name".to_string(), YamlValue::String(pl["Name"].as_string().unwrap().to_owned()));
                misc.insert("team name".to_string(), pl["TeamName"].as_string().unwrap().into());
                misc.insert("platforms".to_string(), YamlValue::Sequence(
                    pl["Platform"].as_array().unwrap().into_iter()
                        .map(|x| x.as_string().unwrap())
                        .map(String::from)
                        .map(YamlValue::String)
                        .collect()
                ));
                misc.insert("creation date".to_string(), YamlValue::String(pl["CreationDate"].as_date().unwrap().to_xml_format()));
                misc.insert("provisioned devices".to_string(), YamlValue::Number((pl["ProvisionedDevices"].as_array().unwrap().len() as i64).into()));
                misc
            },
        };
    });

    let table = create_table(rows);

    println!("{table}");

    println!();
}

fn new_yaml_dict() -> BTreeMap<String, serde_yml::Value> {
    return std::collections::BTreeMap::new();
}

fn create_table(rows: impl Iterator<Item = Row>) -> comfy_table::Table {
    let mut table =  comfy_table::Table::new();
    table.set_header(vec!["AppIDName", "XC mgd", "ApplId Prefix", "Entitlements", "expir. date", "Misc", "file"]);

    for row in rows {
        table.add_row(vec![
            row.app_id_name.clone(),
            format!("{}", row.is_xc_managed),
            row.app_id_prefixes.join(", "),
            to_yaml_str(&row.entitlements),
            row.exp_date.clone(),
            to_yaml_str(&row.misc),
            format!("{}...", &row.file_name[..12])
        ]);
    }

    return table;
}

fn to_yaml_str(value: &YamlDocument) -> String {
    let res = serde_yml::to_string(&value).unwrap();
    return res;
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
