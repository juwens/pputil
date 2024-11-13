#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls
)]

use chrono::{DateTime, Local};
use compact::print_compact_table;
use der::{Decode, Tagged};
use helpers::{OptValueAsBoxStr, PrivisionFileData, NOT_AVAILABLE};
use std::collections::BTreeMap;
use std::fs::{self};
use std::path::Path;
use std::time::SystemTime;
use std::vec;

mod args;
mod compact;
mod helpers;

type YamlValue = serde_yml::value::Value;
type YamlDocument = BTreeMap<String, Option<YamlValue>>;

fn main() {
    let args = args::get_processed_args();

    println!();
    println!("scanning directory: {:?}", args.dir);
    println!();

    let files = get_files(&args).collect::<Vec<_>>();
    let file_data_rows = files.iter().map(|x| parse_file(x));

    match args.mode {
        args::TableMode::Compact => print_compact_table(file_data_rows, &args),
        args::TableMode::Detailed => print_detailed_table(file_data_rows),
    };

    println!();
}

fn parse_file(path: &Path) -> Result<PrivisionFileData, PrivisionFileData> {
    let file = path.to_string_lossy();
    let Ok(pl) = parse_mobileprovision_into_plist(path) else {
        return Err(PrivisionFileData {
            name: Some(
                format!("failed to parse file {file}")
                    .to_string()
                    .into_boxed_str(),
            ),
            app_id_name: None,
            team_name: None,
            xc_managed: None,
            app_id_prefixes: None,
            exp_date: None,
            ent_app_id: None,
            provisioned_devices: None,
            file_path: path.into(),
            local_provision: None,
            uuid: None,
            properties: YamlDocument::new(),
            creation_date: None,
            ent_team_id: None,
            platforms: None,
        });
    };

    let fallback_entitlements = plist::Dictionary::default();
    let ent = pl
        .get("Entitlements")
        .and_then(|x| x.as_dictionary())
        .unwrap_or(&fallback_entitlements);

    let provisioned_devices = pl
        .get("ProvisionedDevices")
        .and_then(|x| x.as_array())
        .map(Vec::len);

    let row = PrivisionFileData {
        app_id_name: pl.get("AppIDName").as_box_str(),
        xc_managed: pl.get("IsXcodeManaged").and_then(plist::Value::as_boolean),
        name: pl
            .get("Name")
            .and_then(|x| x.as_string())
            .map(|x| x.to_string().into_boxed_str()),
        local_provision: pl.get("LocalProvision").and_then(|x| x.as_boolean()),
        app_id_prefixes: {
            let prefixes = pl
                .get("ApplicationIdentifierPrefix")
                .and_then(|x| x.as_array());
            prefixes.map(|x| {
                x.iter()
                    .map(|x| {
                        x.as_string()
                            .map_or(NOT_AVAILABLE.to_string(), |x| x.to_owned())
                            .into_boxed_str()
                    })
                    .collect()
            })
        },
        ent_app_id: ent.get("application-identifier").as_box_str(),
        ent_team_id: ent.get("com.apple.developer.team-identifier").as_box_str(),

        exp_date: pl
            .get("ExpirationDate")
            .and_then(plist::Value::as_date)
            .map(SystemTime::from),

        creation_date: pl
            .get("CreationDate")
            .and_then(plist::Value::as_date)
            .map(SystemTime::from),

        team_name: pl.get("TeamName").as_box_str(),
        provisioned_devices,
        file_path: path.into(),
        uuid: pl.get("UUID").as_box_str(),
        platforms: pl.get("Platform").and_then(|x| x.as_array()).map(|x| {
            x.iter()
                .map(|x| x.as_string().unwrap_or(NOT_AVAILABLE))
                .map(String::from)
                .map(String::into_boxed_str)
                .collect::<Vec<_>>()
        }),
        properties: to_yaml_document(&pl),
    };

    Ok(row)
}

fn get_files(args: &args::Cli) -> impl Iterator<Item = Box<Path>> {
    let files = fs::read_dir(Path::new(&args.dir.as_ref()))
        .unwrap()
        .map(|dir_entry| dir_entry.unwrap().path())
        .filter_map(|path| {
            if path
                .extension()
                .map_or(false, |ext| ext == "mobileprovision")
            {
                Some(path.into_boxed_path())
            } else {
                None
            }
        });
    files
}

fn print_detailed_table(rows: impl Iterator<Item = Result<PrivisionFileData, PrivisionFileData>>) {
    fn encode_to_yaml_str(value: &YamlDocument) -> String {
        serde_yml::to_string(&value).unwrap()
    }

    let mut table = comfy_table::Table::new();
    table.set_header(vec![
        "Profile",
        "expir. date",
        "XC\nmgd",
        "Application\nIdentifier\nPrefix",
        "Properties",
    ]);

    for row in rows {
        if row.is_err() {
            table.add_row(vec![format!("failed to parse: {row:?}")]);
            continue;
        }

        let row = row.ok().unwrap();

        table.add_row(vec![
            format!(
                "Name: {}\n\nFile: {}",
                row.name.unwrap_or(NOT_AVAILABLE.into()),
                row.file_path.file_name().unwrap().to_string_lossy(),
            )
            .as_str(),
            row.exp_date
                .map(DateTime::<Local>::from)
                .map_or(NOT_AVAILABLE.into(), |x| x.format("%Y-%m-%d").to_string())
                .as_str(),
            row.xc_managed
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" }),
            &row.app_id_prefixes
                .map(|x| x.join(", "))
                .unwrap()
                .into_boxed_str(),
            encode_to_yaml_str(&row.properties).as_str(),
        ]);
    }

    print!("{table}");
}

fn parse_mobileprovision_into_plist(
    file: &std::path::Path,
) -> Result<plist::Dictionary, Box<dyn std::error::Error>> {
    assert!(&file.is_file());
    assert!(&file.is_absolute());

    let file_bytes = fs::read(file)?;

    let mut reader = der::SliceReader::new(&file_bytes)?;

    let ci = cms::content_info::ContentInfo::decode(&mut reader)?;

    let sd = {
        assert_eq!(
            ci.content_type.to_string(),
            oid_registry::OID_PKCS7_ID_SIGNED_DATA.to_string()
        );
        ci.content.decode_as::<cms::signed_data::SignedData>()?
    };

    let dict = {
        assert_eq!(
            sd.encap_content_info.econtent_type.to_string(),
            oid_registry::OID_PKCS7_ID_DATA.to_string()
        );
        let econtent = &sd.encap_content_info.econtent.unwrap();
        assert_eq!(econtent.tag(), der::Tag::OctetString);
        let os = econtent.decode_as::<der::asn1::OctetStringRef>()?;
        plist::from_bytes::<plist::Dictionary>(os.as_bytes())?
    };
    Ok(dict)
}

fn to_yaml_value(val: &plist::Value) -> serde_yml::Value {
    match val {
        plist::Value::String(x) => YamlValue::String(x.to_string()),
        plist::Value::Integer(x) => YamlValue::Number(x.as_signed().unwrap().into()),
        plist::Value::Boolean(x) => YamlValue::Bool(*x),
        plist::Value::Date(x) => YamlValue::String(x.to_xml_format()),
        plist::Value::Data(_) => YamlValue::String("<base64 blob>".to_string()),
        plist::Value::Array(x) => {
            if x.len() <= 10 {
                YamlValue::Sequence(x.iter().map(to_yaml_value).collect())
            } else {
                YamlValue::Sequence(vec![
                    YamlValue::String(format!("count: {}", x.len())),
                    YamlValue::String("(abbreviated)".to_string()),
                ])
            }
        }
        plist::Value::Dictionary(x) => YamlValue::Mapping({
            x.iter()
                .map(|x| {
                    (
                        to_yaml_value(&plist::Value::String(x.0.to_string())),
                        to_yaml_value(x.1),
                    )
                })
                .collect()
        }),
        _ => YamlValue::String(core::any::type_name_of_val(val).to_string()),
    }
}

fn to_yaml_document(pl: &plist::Dictionary) -> YamlDocument {
    let items = pl.iter().map(|x| -> (String, Option<serde_yml::Value>) {
        (x.0.to_owned(), Some(to_yaml_value(x.1)))
    });

    #[allow(clippy::from_iter_instead_of_collect)]
    YamlDocument::from_iter(items)
}
