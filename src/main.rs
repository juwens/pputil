// #![warn(
//     clippy::pedantic,
//     clippy::nursery,
// )]

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
    creation_date: Option<SystemTime>,
    ent_app_id: Option<Box<str>>,
    ent_team_id: Option<Box<str>>,
    provisioned_devices: Option<usize>,
    file_path: Box<Path>,
    platforms: Option<Vec<Box<str>>>,
    local_provision: Option<bool>,
}

fn main() {
    let args = args::get_processed_args();
    let files = get_files(&args).collect::<Vec<_>>();

    println!();
    println!("scanning directory: {:?}", &args.input_dir);
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
                    .map(|x| x.as_string().unwrap_or("n/a"))
                    .map(String::from)
                    .map(String::into_boxed_str)
                    .collect::<Vec<_>>()
            }),
        };
    });

    let table = match args.table_mode {
        args::TableMode::Copmpact => create_compact_table(rows),
        args::TableMode::Detailed => create_detailed_table(rows),
    };

    println!("{table}");

    println!();
}

fn get_files(args: &args::ProcessedArgs) -> impl Iterator<Item = Box<Path>> {
    let files = fs::read_dir(Path::new(&args.input_dir))
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
        "Entitlements",
    ]);

    for row in rows {
        let misc = YamlDocument::from([
            ("AppIDName".to_string(), row.app_id_name.to_yaml_value()),
            ("team name".to_string(), row.team_name.to_yaml_value()),
            (
                "platforms".to_string(),
                row.platforms.map(|x| {
                    YamlValue::Sequence(
                        x.iter()
                            .map(|x| YamlValue::String(x.to_owned().to_string()))
                            .collect(),
                    )
                }),
            ),
            (
                "creation_date".to_string(),
                row.creation_date
                    .map(DateTime::<Local>::from)
                    .map(|x| x.to_string())
                    .to_yaml_value(),
            ),
            (
                "provisioned_devices".to_string(),
                row.provisioned_devices.to_yaml_value(),
            ),
            (
                "file".into(),
                row.file_path
                    .file_name()
                    .and_then(|x| x.to_str())
                    .to_yaml_value(),
            ),
            ("LocalProvision".into(), row.local_provision.to_yaml_value()),
        ]);

        table.add_row(vec![
            format!(
                "Name: {}\n\nFile: {}",
                row.name.unwrap_or("n/a".into()),
                row.file_path.file_name().unwrap().to_string_lossy(),
            ).as_str(),
            row.exp_date
                .map(DateTime::<Local>::from)
                .map_or("n/a".into(), |x| x.format("%Y-%m-%d").to_string()).as_str(),
            row.is_xc_managed
                .map_or("n/a", |x| if x { "Y" } else { "N" }),
            row.app_id_prefixes
                .map(|x| x.join(", "))
                .unwrap_or_na()
                .as_str(),
            encode_to_yaml_str(&misc).as_str(),
            encode_to_yaml_str(&YamlDocument::from([
                ("app id".to_string(), row.ent_app_id.to_yaml_value()),
                ("team id".to_string(), row.ent_team_id.to_yaml_value()),
            ]))
            .as_str(),
        ]);
    }

    table
}

fn create_compact_table(rows: impl Iterator<Item = Row>) -> comfy_table::Table {
    let mut table = comfy_table::Table::new();
    table.set_header(vec![
        "Profile Name",
        "AppIDName",
        "ent: application-identifier",
        "expir.\ndate",
        "XC\nmgd",
        "lcl\nprv",
        "team name",
        "prvsnd\ndevices",
        "file",
    ]);

    for row in rows {
        table.add_row(vec![
            row.name.unwrap_or_na(),
            row.app_id_name.unwrap_or_na(),
            row.ent_app_id.unwrap_or_na(),
            row.exp_date
                .map(DateTime::<Local>::from)
                .map(|x| x.format("%Y-%m-%d").to_string())
                .unwrap_or_na(),
            format!(
                "{}",
                row.is_xc_managed
                    .map_or("n/a", |x| if x { "Y" } else { "N" })
            ),
            format!(
                "{}",
                row.local_provision
                    .map_or("n/a", |x| if x { "Y" } else { "N" })
            ),
            row.team_name.unwrap_or_na(),
            row.provisioned_devices
                .map_or(String::from("n/a"), |x| x.to_string()),
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

const NOT_AVAILABLE: &str = "n/a";
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

trait ToYamlValue {
    fn to_yaml_value(self) -> Option<YamlValue>;
}

impl ToYamlValue for Option<&str> {
    fn to_yaml_value(self) -> Option<YamlValue> {
        self.map(|x| YamlValue::String(x.to_string()))
    }
}

impl ToYamlValue for Option<Box<str>> {
    fn to_yaml_value(self) -> Option<YamlValue> {
        self.map(|x| YamlValue::String(x.into_string()))
    }
}

impl ToYamlValue for Option<String> {
    fn to_yaml_value(self) -> Option<YamlValue> {
        self.map(YamlValue::String)
    }
}

impl ToYamlValue for Option<usize> {
    fn to_yaml_value(self) -> Option<YamlValue> {
        self.map(|x| YamlValue::Number((x as i64).into()))
    }
}

impl ToYamlValue for Option<bool> {
    fn to_yaml_value(self) -> Option<YamlValue> {
        self.map(YamlValue::Bool)
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
