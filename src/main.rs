use std::{any::Any, borrow::BorrowMut, fs::{self}};
use der::{Decode, Tagged};

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

    for path in paths {
        println!("Name: {}", path.display());

        let file_bytes = fs::read(path.clone()).unwrap();
        let mut reader = der::SliceReader::new(&file_bytes).unwrap();
        let ci = cms::content_info::ContentInfo::decode(reader.borrow_mut()).unwrap();
        let sd = ci.content.decode_as::<cms::signed_data::SignedData>().unwrap();
        let content = &sd.encap_content_info.econtent.unwrap();
        let bytes = content.value();
        let pl = plist::from_bytes::<plist::Dictionary>(bytes).unwrap();
        
        println!("\tAppIDName: {:?}", pl["AppIDName"]);
        println!("\tApplicationIdentifierPrefix: {:?}", pl["ApplicationIdentifierPrefix"]);
    }
}