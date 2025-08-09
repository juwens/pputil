// #![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls,
    clippy::map_unwrap_or,
    clippy::too_many_lines
)]

use args::{ListExtendedArgs, XcProvisioningProfileDir, XcProvisioningProfileDirKind};
use chrono::{DateTime, Local};
use compact::print_compact_table;
use der::{Decode, Tagged};
use helpers::{ProvisioningProfileFileData, NOT_AVAILABLE};
use openssl::x509::X509;
use std::fs::{self};
use std::path::Path;
use std::rc::Rc;
use std::time::SystemTime;
use std::vec;

use self::tui::tui_main::run_tui_mode;
use crate::helpers::encode_to_yaml_str;
use crate::types::ProfilesCollection;
use crate::yml_types::{YamlDocument, YamlValue};

mod args;
mod compact;
mod helpers;
mod tui;
mod yml_types;
mod types;

struct XcProvisioningProfileFile {
    pub path: Rc<Path>,
    pub xc_kind: XcProvisioningProfileDirKind,
}

#[derive(Debug)]
struct CertDetails {
    subject: Rc<str>,
    issuer: Rc<str>,
    serial: Rc<str>,
    not_before: Rc<str>,
    not_after: Rc<str>,
}

impl CertDetails {
    fn error() -> CertDetails {
        let na = Rc::<str>::from("n/a");
        CertDetails {
            subject: na.clone(),
            issuer: na.clone(),
            serial: na.clone(),
            not_before: na.clone(),
            not_after: na.clone(),
        }
    }

    fn from_cert(cert: &X509) -> CertDetails {
        CertDetails {
            subject: Rc::<str>::from(name_to_string(cert.subject_name())),
            issuer: Rc::<str>::from(name_to_string(cert.issuer_name())),
            serial: Rc::<str>::from(format!(
                "{}",
                cert.serial_number().to_bn().unwrap().to_dec_str().unwrap()
            )),
            not_before: Rc::from(format!("{}", cert.not_before())),
            not_after: Rc::from(format!("{}", cert.not_after())),
        }
    }
}

#[allow(clippy::unnecessary_wraps)]
fn main() -> Result<(), std::io::Error> {
    color_eyre::install().unwrap();

    let args = args::get_processed_args();

    println!();
    println!("scanning directories:");
    for dir in &args.actual_dirs() {
        println!(
            " * {} ({:?})",
            dir.relative_path.to_string_lossy(),
            dir.kind
        );
    }
    println!();

    let files = get_files_from_dirs(&args.actual_dirs());
    let file_data_rows = files.iter().map(parse_file).collect::<Vec<_>>();

    match args.command {
        args::Commands::List(x) => print_compact_table(file_data_rows, &x),
        args::Commands::ListExtended(x) => print_extended_table(file_data_rows, &x),
        args::Commands::Tui(x) => {
            if let Err(e) = run_tui_mode(file_data_rows, &x) {
                eprintln!("TUI error: {e}");
            }
        }
    }

    println!();

    Ok(())
}

fn parse_file(
    file: &XcProvisioningProfileFile,
) -> Result<Rc<ProvisioningProfileFileData>, Rc<ProvisioningProfileFileData>> {
    let Ok(pl) = parse_mobileprovision_into_plist(&file.path) else {
        return Err(Rc::new(ProvisioningProfileFileData {
            name: Some(Rc::<str>::from(format!(
                "failed to parse file {}",
                file.path.to_string_lossy()
            ))),
            app_id_name: None,
            team_name: None,
            xc_managed: None,
            xc_kind: None,
            app_id_prefixes: None,
            exp_date: None,
            ent_app_id: None,
            provisioned_devices: vec![],
            provisioned_devices_count: None,
            file_path: Rc::clone(&file.path),
            local_provision: None,
            uuid: None,
            properties: YamlDocument::new(),
            creation_date: None,
            ent_team_id: None,
            platforms: None,
            developer_certificates_raw: Vec::new(),
            developer_certificates: Vec::new(),
        }));
    };

    let fallback_entitlements = plist::Dictionary::default();
    let ent = pl
        .get("Entitlements")
        .and_then(|x| x.as_dictionary())
        .unwrap_or(&fallback_entitlements);

    let provisioned_devices: Vec<Rc<str>> = pl
        .get("ProvisionedDevices")
        .and_then(|devices| devices.as_array())
        .map(|array| {
            array
                .iter()
                .map(|item| Rc::<str>::from(item.as_string().unwrap_or(NOT_AVAILABLE)))
                .collect::<Vec<Rc<str>>>()
        })
        .unwrap_or_else(|| vec![Rc::<str>::from("failed to parse")]);

    let row = ProvisioningProfileFileData {
        app_id_name: pl
            .get("AppIDName")
            .and_then(plist::Value::as_string)
            .map(Rc::<str>::from)
            .or_else(|| Some(Rc::<str>::from(NOT_AVAILABLE))),
        xc_managed: pl.get("IsXcodeManaged").and_then(plist::Value::as_boolean),
        xc_kind: match file.xc_kind {
            XcProvisioningProfileDirKind::Xc15 => Some("15-".into()),
            XcProvisioningProfileDirKind::Xc16 => Some("16+".into()),
            XcProvisioningProfileDirKind::Custom => Some("custom".into()),
        },
        name: pl
            .get("Name")
            .and_then(plist::Value::as_string)
            .map(Rc::<str>::from),
        local_provision: pl.get("LocalProvision").and_then(|x| x.as_boolean()),
        app_id_prefixes: {
            let prefixes = pl
                .get("ApplicationIdentifierPrefix")
                .and_then(|x| x.as_array());
            prefixes.map(|x| {
                x.iter()
                    .map(|x| Rc::<str>::from(x.as_string().unwrap_or(NOT_AVAILABLE)))
                    .collect()
            })
        },
        ent_app_id: ent
            .get("application-identifier")
            .and_then(plist::Value::as_string)
            .map(Rc::<str>::from)
            .or_else(|| Some(Rc::<str>::from(NOT_AVAILABLE))),
        ent_team_id: ent
            .get("com.apple.developer.team-identifier")
            .and_then(plist::Value::as_string)
            .map(Rc::<str>::from)
            .or_else(|| Some(Rc::<str>::from(NOT_AVAILABLE))),

        exp_date: pl
            .get("ExpirationDate")
            .and_then(plist::Value::as_date)
            .map(SystemTime::from),

        creation_date: pl
            .get("CreationDate")
            .and_then(plist::Value::as_date)
            .map(SystemTime::from),

        team_name: pl
            .get("TeamName")
            .and_then(plist::Value::as_string)
            .map(Rc::<str>::from),
        provisioned_devices,
        provisioned_devices_count: Some(usize::MAX),
        file_path: file.path.clone(),
        uuid: pl
            .get("UUID")
            .and_then(plist::Value::as_string)
            .map(Rc::<str>::from)
            .or_else(|| Some(Rc::<str>::from(NOT_AVAILABLE))),
        platforms: pl.get("Platform").and_then(|x| x.as_array()).map(|x| {
            x.iter()
                .map(|x| Rc::<str>::from(x.as_string().unwrap_or(NOT_AVAILABLE)))
                .collect::<Vec<_>>()
        }),
        properties: to_yaml_document(&pl),
        developer_certificates_raw: {
            let property = pl
                .get("DeveloperCertificates")
                .and_then(plist::Value::as_array);
            if property.is_none() {
                Vec::new()
            } else {
                let res = property
                    .unwrap()
                    .iter()
                    .filter_map(|x| x.as_data())
                    .map(|x| x.to_vec())
                    .collect::<Vec<_>>();
                res
            }
        },
        developer_certificates: {
            let property = pl
                .get("DeveloperCertificates")
                .and_then(plist::Value::as_array);
            if property.is_none() {
                Vec::new()
            } else {
                let res = property
                    .unwrap()
                    .iter()
                    .filter_map(|x| x.as_data())
                    .map(X509::from_der)
                    .map(|x| match x {
                        Ok(cert) => CertDetails::from_cert(&cert),
                        Err(_) => CertDetails::error(),
                    })
                    .collect::<Vec<_>>();
                res
            }
        },
    };

    Ok(Rc::new(row))
}

fn get_files_from_dirs(dirs: &[XcProvisioningProfileDir]) -> Vec<XcProvisioningProfileFile> {
    dirs.iter().flat_map(get_files_from_dir).collect::<Vec<_>>()
}

fn get_files_from_dir(xc_dir: &XcProvisioningProfileDir) -> Vec<XcProvisioningProfileFile> {
    match fs::read_dir(xc_dir.absolute_path()) {
        Err(_) => vec![],
        Ok(dir) => dir
            .map(|dir_entry| dir_entry.unwrap().path())
            .filter_map(|path| {
                if path.extension().is_some_and(|ext| ext == "mobileprovision") {
                    Some(XcProvisioningProfileFile {
                        path: Rc::from(path),
                        xc_kind: xc_dir.kind,
                    })
                } else {
                    None
                }
            })
            .collect(),
    }
}

fn print_extended_table<'a>(
    rows: ProfilesCollection,
    _args: &ListExtendedArgs,
) {
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
                row.name.as_deref().unwrap_or(NOT_AVAILABLE),
                row.file_path.file_name().unwrap().to_string_lossy(),
            )
            .as_str(),
            row.exp_date
                .map(DateTime::<Local>::from)
                .map_or(NOT_AVAILABLE.into(), |x| x.format("%Y-%m-%d").to_string())
                .as_str(),
            row.xc_managed
                .map_or(NOT_AVAILABLE, |x| if x { "Y" } else { "N" }),
            &row.app_id_prefixes.clone()
                .map(|x| x.join(", "))
                .unwrap_or_default(),
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

// Helper: convert X509Name to a simple string like "CN=..., O=..., C=..."
fn name_to_string(name: &openssl::x509::X509NameRef) -> String {
    let mut parts = vec![];
    for entry in name.entries() {
        let obj = entry.object();
        // try to fetch a short name for known OIDs, else use OID string
        let asn1_object_ref = obj.to_string();
        let key = obj.nid().short_name().unwrap_or(asn1_object_ref.as_str());
        match entry.data().as_utf8() {
            Ok(v) => parts.push(format!("{key}={v}")),
            Err(_) => parts.push(format!("{key}=<binary>")),
        }
    }
    parts.join(", ")
}
