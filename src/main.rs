// #![warn(
//     clippy::pedantic,
//     clippy::nursery,
// )]

use args::{CompactSortBy, Args};
use chrono::{DateTime, Local};
use der::{Decode, Tagged};
use std::collections::BTreeMap;
use std::fs::{self};
use std::path::Path;
use std::time::SystemTime;
use std::vec;

mod args;

type YamlValue = serde_yml::value::Value;
type YamlDocument = BTreeMap<String, Option<YamlValue>>;

#[derive(Debug)]
struct Row {
    app_id_name: Option<Box<str>>,
    name: Option<Box<str>>,
    team_name: Option<Box<str>>,
    is_xc_managed: Option<bool>,
    app_id_prefixes: Option<Vec<Box<str>>>,
    exp_date: Option<SystemTime>,
    ent_app_id: Option<Box<str>>,
    provisioned_devices: Option<usize>,
    file_path: Box<Path>,
    local_provision: Option<bool>,
    properties: YamlDocument,
    #[allow(dead_code)]
    creation_date: Option<SystemTime>,
    #[allow(dead_code)]
    ent_team_id: Option<Box<str>>,
    #[allow(dead_code)]
    platforms: Option<Vec<Box<str>>>,
}

fn main() {
    let args = args::get_processed_args();
    let files = get_files(&args).collect::<Vec<_>>();

    println!();
    println!("scanning directory: {:?}", args.dir);
    println!();

    let rows = files.iter().map(|path| {
        let pl = match parse_mobileprovision_into_plist(path) {
            Ok(x) => x,
            Err(error) => panic!("Problem opening the file: {error:?}"),
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

        return Row {
            app_id_name: pl.get("AppIDName").to_str(),
            is_xc_managed: pl.get("IsXcodeManaged").and_then(plist::Value::as_boolean),
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

            ent_app_id: ent
                .get("application-identifier")
                .to_string()
                .map(String::into_boxed_str),
            ent_team_id: ent
                .get("com.apple.developer.team-identifier")
                .to_string()
                .map(String::into_boxed_str),

            exp_date: pl
                .get("ExpirationDate")
                .and_then(plist::Value::as_date)
                .map(SystemTime::from),

            creation_date: pl
                .get("CreationDate")
                .and_then(plist::Value::as_date)
                .map(SystemTime::from),

            team_name: pl.get("TeamName").to_string().map(String::into_boxed_str),
            provisioned_devices,
            file_path: path.clone(),
            platforms: pl.get("Platform").and_then(|x| x.as_array()).map(|x| {
                x.iter()
                    .map(|x| x.as_string().unwrap_or(NOT_AVAILABLE))
                    .map(String::from)
                    .map(String::into_boxed_str)
                    .collect::<Vec<_>>()
            }),
            properties: to_yaml_document(&pl),
        };
    });

    let table = match args.mode {
        args::TableMode::Compact => create_compact_table(rows, &args),
        args::TableMode::Detailed => create_detailed_table(rows),
    };

    println!("{table}");

    println!();
}

fn get_files(args: &args::Args) -> impl Iterator<Item = Box<Path>> {
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

fn create_detailed_table(rows: impl Iterator<Item = Row>) -> comfy_table::Table {
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
            row.is_xc_managed
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" }),
            row.app_id_prefixes
                .map(|x| x.join(", "))
                .unwrap_or_na()
                .as_str(),
            encode_to_yaml_str(&row.properties).as_str(),
        ]);
    }

    table
}

fn create_compact_table(iter: impl Iterator<Item = Row>, args: &Args) -> comfy_table::Table {
    let mut table = comfy_table::Table::new();
    table.set_header(vec![
        "Profile Name",
        "AppIDName",
        "Entitlements:\napplication-identifier",
        "expir.\ndate",
        "XC\nmgd",
        "lcl\nprv",
        "team name",
        "prv\ndvc",
        "file",
    ]);
    
    let mut rows = iter.collect::<Vec<_>>();

    match args.sort_by {
        CompactSortBy::Name => rows.sort_by_key(|x| x.name.clone().unwrap_or_na().to_lowercase()),
        CompactSortBy::AppIdName => rows.sort_by_key(|x| x.app_id_name.clone().unwrap_or_na().to_lowercase()),
        CompactSortBy::ExpirationDate => rows.sort_by_key(|x| x.exp_date.to_string().as_deref().map(str::to_lowercase)),
    }

    match args.sort_order {
        args::SortOrder::Asc => {},
        args::SortOrder::Desc => rows.reverse(),
    }

    for row in rows {
        table.add_row(vec![
            row.name.unwrap_or_na(),
            row.app_id_name.unwrap_or_na(),
            row.ent_app_id.unwrap_or_na(),
            row.exp_date
                .map(DateTime::<Local>::from)
                .map(|x| {
                    let mut s = x.format("%Y-%m-%d").to_string();
                    if x.le(&chrono::Utc::now()) {
                        s.push_str(" !!!");
                    }
                    s
                })
                .unwrap_or_na(),
            format!(
                "{}",
                row.is_xc_managed
                    .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
            ),
            format!(
                "{}",
                row.local_provision
                    .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
            ),
            row.team_name.unwrap_or_na(),
            row.provisioned_devices
                .map_or(String::from(NOT_AVAILABLE), |x| x.to_string()),
            format!(
                "{}...",
                &row.file_path.file_name().unwrap().to_str().unwrap()[..12]
            ),
        ]);
    }

    table
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

trait UnwrapOrNa {
    fn unwrap_or_na(self) -> String;
}

const NOT_AVAILABLE: &str = "_";
impl UnwrapOrNa for Option<String> {
    fn unwrap_or_na(self) -> String {
        self.unwrap_or(NOT_AVAILABLE.to_owned())
    }
}

impl UnwrapOrNa for Option<&str> {
    fn unwrap_or_na(self) -> String {
        self.map_or(NOT_AVAILABLE.to_owned(), ToString::to_string)
    }
}

impl UnwrapOrNa for Option<Box<str>> {
    fn unwrap_or_na(self) -> String {
        self.map_or(NOT_AVAILABLE.to_owned(), |x| x.to_string())
    }
}

impl UnwrapOrNa for Option<usize> {
    fn unwrap_or_na(self) -> String {
        self.map_or(NOT_AVAILABLE.to_owned(), |x| x.to_string())
    }
}

trait MyToString {
    fn to_string(self) -> Option<String>;
    fn to_str(self) -> Option<Box<str>>;
}

impl MyToString for Option<&plist::Value> {
    fn to_string(self) -> Option<String> {
        self.and_then(|x| x.as_string()).map(ToString::to_string)
    }

    fn to_str(self) -> Option<Box<str>> {
        self.and_then(|x| x.as_string())
            .map(|x| x.to_string().into_boxed_str())
    }
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

    YamlDocument::from_iter(items)
}

impl MyToString for Option<SystemTime> {
    fn to_string(self) -> Option<String> {
        self
            .map(DateTime::<Local>::from)
            .map(|x| x.to_string())
    }

    fn to_str(self) -> Option<Box<str>> {
        todo!()
    }
}