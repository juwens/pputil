#![warn(
    clippy::pedantic,
)]
#![allow(
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls,
)]

use args::{Cli, CompactSortBy};
use chrono::{DateTime, Local};
use comfy_table::Cell;
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
    /// is Xcode managed
    xc_managed: Option<bool>,
    app_id_prefixes: Option<Vec<Box<str>>>,
    /// expiration date
    exp_date: Option<SystemTime>,
    /// entitlements.application-identifier
    ent_app_id: Option<Box<str>>,
    provisioned_devices: Option<usize>,
    file_path: Box<Path>,
    local_provision: Option<bool>,
    uuid: Option<Box<str>>,
    properties: YamlDocument,
    #[allow(dead_code)]
    creation_date: Option<SystemTime>,
    #[allow(dead_code)]
    ent_team_id: Option<Box<str>>,
    #[allow(dead_code)]
    platforms: Option<Vec<Box<str>>>,
}

#[derive(Debug)]
enum WidthMode {
    Small,
    Unlimited,
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
            file_path: path.clone(),
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
    });

    let table = match args.mode {
        args::TableMode::Compact => create_compact_table(rows, &args),
        args::TableMode::Detailed => create_detailed_table(rows),
    };

    println!("{table}");

    println!();
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
            row.xc_managed
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" }),
            &row.app_id_prefixes
                .map(|x| x.join(", "))
                .unwrap()
                .into_boxed_str(),
            encode_to_yaml_str(&row.properties).as_str(),
        ]);
    }

    table
}

fn create_compact_table(iter: impl Iterator<Item = Row>, args: &Cli) -> comfy_table::Table {
    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

    table.set_header(vec![
        "Profile Name",
        "App ID Name",
        "Entitlements:\napplication-identifier",
        "expir.\ndate",
        "XC\nmgd",
        "lcl\nprv",
        "team name",
        "prv\ndvc",
        "UUID",
    ]);

    let mut rows = iter.collect::<Vec<_>>();

    match &args.command.as_ref().unwrap() {
        args::Commands::PrintCompact(compact_args) => {
            match compact_args.sort_by {
                CompactSortBy::Name => rows.sort_by_key(|x| x.name.unwrap_or_na().to_lowercase()),
                CompactSortBy::AppIdName => {
                    rows.sort_by_key(|x| x.app_id_name.unwrap_or_na().to_lowercase());
                }
                CompactSortBy::ExpirationDate => {
                    rows.sort_by_key(|x| x.exp_date.to_string().as_deref().map(str::to_lowercase));
                }
            };
            match compact_args.sort_order {
                args::SortOrder::Asc => {}
                args::SortOrder::Desc => rows.reverse(),
            }
        }
    };

    let width = match dbg!(termsize::get().unwrap().cols) {
        ..=250 => WidthMode::Small,
        _ => WidthMode::Unlimited,
    };
    dbg!(&width);

    for row in rows {
        table.add_row(vec![
            row.name.unwrap_or_na().into_cell(),
            row.app_id_name.unwrap_or_na().into_cell(),
            row.ent_app_id
                .unwrap_or_na()
                .truncate_ex(&width, 40)
                .into_cell(),
            row.exp_date.map(DateTime::<Local>::from).map_or_else(
                || Cell::new(NOT_AVAILABLE),
                |x| {
                    let s = x.format("%Y-%m-%d").to_string();
                    let c = Cell::new(s);
                    if x.le(&chrono::Utc::now()) {
                        return c.fg(comfy_table::Color::Red);
                    }
                    c
                },
            ),
            row.xc_managed
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
                .to_string()
                .into_cell(),
            row.local_provision
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" })
                .to_string()
                .into_cell(),
            row.team_name
                .unwrap_or_na()
                .truncate_ex(&width, 12)
                .into_cell(),
            row.provisioned_devices
                .map_or(String::from(NOT_AVAILABLE), |x| x.to_string())
                .into_cell(),
            row.uuid.unwrap_or_na().truncate_ex(&width, 6).into_cell(),
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

trait UnwrapOrNa {
    fn unwrap_or_na(&self) -> String;
}

const NOT_AVAILABLE: &str = "_";

impl UnwrapOrNa for Option<Box<str>> {
    fn unwrap_or_na(&self) -> String {
        self.clone().as_deref().unwrap_or(NOT_AVAILABLE).to_string()
    }
}

trait ToStringExt {
    fn to_string(self) -> Option<String>;
}
impl ToStringExt for Option<SystemTime> {
    fn to_string(self) -> Option<String> {
        self.map(DateTime::<Local>::from).map(|x| x.to_string())
    }
}

trait IntoCell {
    fn into_cell(self) -> Cell;
}

impl IntoCell for String {
    fn into_cell(self) -> Cell {
        Cell::new(self)
    }
}

trait Truncate {
    fn truncate(&self, count: usize) -> Self;
    fn truncate_ex(self, mode: &WidthMode, s_len: usize) -> Self;
}

impl Truncate for String {
    fn truncate(&self, count: usize) -> String {
        if self.len() <= count {
            self.to_owned()
        } else {
            format!("{}...", &self[..count])
        }
    }

    fn truncate_ex(self, mode: &WidthMode, s_len: usize) -> Self {
        match mode {
            WidthMode::Small => self.truncate(s_len),
            WidthMode::Unlimited => self,
        }
    }
}

trait OptValueAsBoxStr {
    fn as_box_str(&self) -> Option<Box<str>>;
}

impl OptValueAsBoxStr for Option<&plist::Value> {
    fn as_box_str(&self) -> Option<Box<str>> {
        self.and_then(plist::Value::as_string).map(Box::from)
    }
}
