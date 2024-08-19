use std::{borrow::{Borrow, BorrowMut}, fs::{self}};
use chrono::{DateTime, Local};
use der::Decode;
use comfy_table::Table;
use json::{object, JsonValue};
use std::time::SystemTime;
use tap::{Conv, Pipe};
use yaml_rust2::{yaml, Yaml, YamlEmitter, YamlLoader};

struct Row {
    app_id_name: String,
    is_xc_managed: bool,
    app_id_prefixes: Vec<String>,
    entitlements: JsonValue,
    exp_date: String,
    file_name: String,
    misc: yaml::Yaml,
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


        let platforms = (pl["Platform"].as_array().unwrap().into_iter())
                .map(|x| x.as_string().unwrap())
                .map(String::from)
                .map(Yaml::String)
                .collect::<Vec<_>>()
                .pipe(|x| Yaml::Array(x));

        let mut misc = yaml::Hash::new();
        misc.insert(Yaml::String(String::from("name")), Yaml::String(pl["Name"].as_string().unwrap().into()));
        misc.insert(Yaml::String(String::from("team name")), Yaml::String(pl["TeamName"].as_string().unwrap().into()));
        misc.insert(Yaml::String(String::from("platforms")), platforms);
        misc.insert(Yaml::String(String::from("creation date")), Yaml::String(pl["CreationDate"].as_date().unwrap().to_xml_format()));
        misc.insert(Yaml::String(String::from("provisioned devices")), Yaml::Integer(pl["ProvisionedDevices"].as_array().unwrap().len() as i64));


        return Row {
            app_id_name: (*pl["AppIDName"].as_string().unwrap()).into(),
            is_xc_managed: pl["IsXcodeManaged"].as_boolean().unwrap(),
            app_id_prefixes: vec![ app_id_prefix.first().unwrap().as_string().unwrap().conv::<String>()],
            entitlements: object!(
                app_id: entitlements["application-identifier"].as_string().unwrap().conv::<String>(),
                team_id: entitlements.get("com.apple.developer.team-identifier").unwrap().as_string().unwrap().conv::<String>(),
            ),
            exp_date: exp_date.format("%Y-%m-%d").to_string().conv::<String>(),
            file_name: path.file_name().unwrap().to_str().unwrap().conv::<String>(),
            misc: Yaml::Hash(misc),
        };
    }).collect::<Vec<_>>();

    let mut table = Table::new();
    table
        .set_header(vec!["AppIDName", "XC mgd", "ApplId Prefix", "Entitlements", "expir. date", "Misc", "file"])
        .add_rows(rows.into_iter().map(|x| {
            let json = YamlLoader::load_from_str(x.entitlements.to_string().borrow()).unwrap();
            let mut out_str = String::new();
            YamlEmitter::new(&mut out_str).dump(&json[0]).unwrap();

            let mut misc_out = String::new();
            YamlEmitter::new(&mut misc_out).dump(&x.misc).unwrap();

            return vec![
                    x.app_id_name,
                    x.is_xc_managed.to_string(),
                    x.app_id_prefixes.join(", "),
                    trim_yaml_start(&out_str),
                    x.exp_date,
                    trim_yaml_start(&misc_out),
                    (x.file_name[..12].to_string() + "...").into()
                ]
        }));

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

pub fn trim_yaml_start(s : &str) -> String {
    return s.trim_start_matches('-').trim_start().into();
}