use clap::ArgAction;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ProcessedArgs {
    pub input_dir: String,
}

pub fn get_processed_args() -> ProcessedArgs {
    let dir_arg = clap::Arg::new("directory")
        .short('d')
        .long("dir")
        .action(ArgAction::Set)
        .value_parser(clap::value_parser!(PathBuf))
        .value_name("DIR")
        .default_value("~/Library/MobileDevice/Provisioning Profiles");

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

    ProcessedArgs {
        input_dir: input_dir_expanded,
    }
}
