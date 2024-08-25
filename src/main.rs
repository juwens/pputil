use chrono::{DateTime, Local};
use clap::ArgAction;
use der::{Decode, Tagged};
use std::fs::{self};
use std::path::PathBuf;
use std::time::SystemTime;
use std::{borrow::BorrowMut, collections::BTreeMap};
use tap::Pipe;

type YamlValue = serde_yml::value::Value;
type YamlDocument = BTreeMap<String, YamlValue>;

struct Row {
    app_id_name: String,
    is_xc_managed: bool,
    app_id_prefixes: Vec<String>,
    entitlements: YamlDocument,
    exp_date: String,
    misc: YamlDocument,
}

fn main() {
    let dir_arg = clap::Arg::new("directory")
        .short('d')
        .long("dir")
        .action(ArgAction::Set)
        .value_parser(clap::value_parser!(PathBuf))
        .value_name("DIR")
        .default_value( "~/Library/MobileDevice/Provisioning Profiles");
    
    let matches = clap::Command::new(clap::crate_name!())
        .author(clap::crate_authors!())
        .arg(&dir_arg)
        .get_matches();

    let input_dir_str = paths_as_strings::encode_path({
        let dir_arg_id = dir_arg.get_id().as_str();
        let path_buf = matches.get_one::<PathBuf>(&dir_arg_id).unwrap();
        path_buf
    });
    let input_dir_expanded = shellexpand::tilde(&input_dir_str).into_owned();

    println!("scanning directory: {:?}", input_dir_expanded);

    let paths = fs::read_dir(PathBuf::from(input_dir_expanded))
        .unwrap()
        .map(|dir_entry| dir_entry.unwrap().path())
        .filter_map(|path| {
            if path
            .extension()
            .map_or(false, |ext| ext == "mobileprovision")
            {
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
                let prefixes = pl["ApplicationIdentifierPrefix"].as_array().unwrap();
                prefixes
                    .iter()
                    .map(|x| x.as_string().unwrap().to_owned())
                    .collect()
            },

            entitlements: {
                let ent = pl["Entitlements"].as_dictionary().unwrap();
                YamlDocument::from([
                    (
                        "app_id".into(),
                        ent["application-identifier"].as_string().unwrap().into(),
                    ),
                    (
                        "team_id".into(),
                        ent["com.apple.developer.team-identifier"]
                            .as_string()
                            .unwrap()
                            .into(),
                    ),
                ])
            },

            exp_date: pl["ExpirationDate"]
                .as_date()
                .unwrap()
                .pipe(SystemTime::from)
                .pipe(DateTime::<Local>::from)
                .format("%Y-%m-%d")
                .to_string(),

            misc: YamlDocument::from([
                ("name".into(), pl["Name"].as_string().unwrap().into()),
                (
                    "team name".to_string(),
                    pl["TeamName"].as_string().unwrap().into(),
                ),
                (
                    "platforms".into(),
                    YamlValue::Sequence(
                        pl["Platform"]
                            .as_array()
                            .unwrap()
                            .into_iter()
                            .map(|x| x.as_string().unwrap())
                            .map(YamlValue::from)
                            .collect(),
                    ),
                ),
                (
                    "creation date".to_string(),
                    pl["CreationDate"].as_date().unwrap().to_xml_format().into(),
                ),
                (
                    "provisioned devices".to_string(),
                    (pl["ProvisionedDevices"].as_array().unwrap().len() as i64).into(),
                ),
                (
                    "file".into(),
                    path.file_name().unwrap().to_str().unwrap().into(),
                ),
            ]),
        };
    });

    let table = create_table(rows);

    println!("{table}");

    println!();
}

fn create_table(rows: impl Iterator<Item = Row>) -> comfy_table::Table {
    let mut table = comfy_table::Table::new();
    table.set_header(vec![
        "AppIDName",
        "expir. date",
        "XC\nmgd",
        "ApplId Prefix",
        "Entitlements",
        "Misc",
    ]);

    for row in rows {
        table.add_row(vec![
            row.app_id_name.clone(),
            row.exp_date.clone(),
            format!("{}", if row.is_xc_managed { "Y" } else { "N" }),
            row.app_id_prefixes.join(", "),
            to_yaml_str(&row.entitlements),
            to_yaml_str(&row.misc),
        ]);
    }

    return table;
}

fn parse_mobileprovision_into_plist(
    path: &std::path::PathBuf,
) -> Result<plist::Dictionary, Box<dyn std::error::Error>> {
    let file_bytes = fs::read(path)?;

    let mut reader = der::SliceReader::new(&file_bytes)?;

    let ci = cms::content_info::ContentInfo::decode(reader.borrow_mut())?;

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
    return Ok(dict);
}

fn to_yaml_str(value: &YamlDocument) -> String {
    let res = serde_yml::to_string(&value).unwrap();
    return res;
}
