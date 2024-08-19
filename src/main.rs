use std::{borrow::BorrowMut, fs::{self}};
use der::Decode;
use comfy_table::Table;

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

    let mut table = Table::new();
    table
        .set_header(vec!["AppIDName", "ApplId Prefix", "expir. date", "ent: app identifier", "team-identifier", "file"]);

    for path in paths {
        // println!("Name: {}", path.display());

        let file_bytes = fs::read(path.clone()).unwrap();
        let mut reader = der::SliceReader::new(&file_bytes).unwrap();
        let ci = cms::content_info::ContentInfo::decode(reader.borrow_mut()).unwrap();
        let sd = ci.content.decode_as::<cms::signed_data::SignedData>().unwrap();
        let content = &sd.encap_content_info.econtent.unwrap();
        let pl = plist::from_bytes::<plist::Dictionary>(content.value()).unwrap();

        let app_id_name = &pl["AppIDName"];
        let app_id_prefix = &pl["ApplicationIdentifierPrefix"].as_array().unwrap();
        let exp_date = &pl["ExpirationDate"].as_date().unwrap();
        let entitlements = &pl["Entitlements"].as_dictionary().unwrap();

        table.add_row(vec![
            app_id_name.as_string().unwrap_or_default(),
            app_id_prefix.first().unwrap().as_string().unwrap(),
            &exp_date.to_xml_format(),
            &entitlements["application-identifier"].as_string().unwrap(),
            &entitlements.get("com.apple.developer.team-identifier").unwrap().as_string().unwrap(),
            path.file_name().unwrap().to_str().unwrap()
        ]);
    }

    println!("{table}");

    println!();
}
