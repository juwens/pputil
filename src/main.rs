use chrono::{DateTime, Local};
use der::{Decode, Tagged};
use std::collections::BTreeMap;
use std::fs::{self};
use std::path::Path;
use std::time::SystemTime;
use tap::Pipe;

mod args;

type YamlValue = serde_yml::value::Value;
type YamlDocument = BTreeMap<String, YamlValue>;

#[derive(Debug)]
struct Row {
    app_id_name: String,
    name: String,
    team_name: String,
    is_xc_managed: bool,
    app_id_prefixes: Vec<String>,
    entitlements: YamlDocument,
    exp_date: String,
    misc: YamlDocument,
    ent_app_id: String,
    ent_team_id: String,
    provisioned_devices: i64,
    file_path: Box<Path>,
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

        let ent = pl["Entitlements"].as_dictionary().unwrap();
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

            ent_app_id: ent["application-identifier"].as_string().unwrap().into(),
            ent_team_id: ent["com.apple.developer.team-identifier"]
                .as_string()
                .unwrap()
                .into(),
            entitlements: {
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

            name: pl["Name"].as_string().unwrap().into(),
            team_name: pl["TeamName"].as_string().unwrap().into(),
            provisioned_devices: (pl["ProvisionedDevices"].as_array().unwrap().len() as i64),
            file_path: path.clone(),

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
                            .iter()
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
            encode_to_yaml_str(&row.entitlements),
            encode_to_yaml_str(&row.misc),
        ]);
    }

    return table;

    fn encode_to_yaml_str(value: &YamlDocument) -> String {
        serde_yml::to_string(&value).unwrap()
    }
}

fn create_compact_table(rows: impl Iterator<Item = Row>) -> comfy_table::Table {
    let mut table = comfy_table::Table::new();
    table.set_header(vec![
        "AppIDName",
        "Name",
        "expir. date",
        "XC\nmgd",
        "app id",
        "team name",
        "prvsnd\ndevices",
        "file",
    ]);

    for row in rows {
        table.add_row(vec![
            row.app_id_name.clone(),
            row.name,
            row.exp_date.clone(),
            format!("{}", if row.is_xc_managed { "Y" } else { "N" }),
            row.ent_app_id,
            row.team_name,
            row.provisioned_devices.to_string(),
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
