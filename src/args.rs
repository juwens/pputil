use clap::ArgAction;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ProcessedArgs {
    pub input_dir: String,
    pub table_mode: TableMode,
}

#[derive(Debug)]
pub enum TableMode {
    Copmpact,
    Detailed,
}

pub fn get_processed_args() -> ProcessedArgs {
    let dir_arg = clap::Arg::new("directory")
        .short('d')
        .long("dir")
        .action(ArgAction::Set)
        .value_parser(clap::value_parser!(PathBuf))
        .value_name("DIR")
        .default_value("~/Library/MobileDevice/Provisioning Profiles");

    let compact_arg = clap::Arg::new("compact")
        .short('c')
        .long("compact")
        .help("compact table output")
        .num_args(0)
        .action(ArgAction::SetTrue);

    let matches = clap::Command::new(clap::crate_name!())
        .author(clap::crate_authors!())
        .arg(&dir_arg)
        .arg(&compact_arg)
        .get_matches();

    let input_dir_str = paths_as_strings::encode_path({
        let path_buf = matches
            .get_one::<PathBuf>(dir_arg.get_id().as_str())
            .unwrap();
        path_buf
    });
    let input_dir_expanded = shellexpand::tilde(&input_dir_str).into_owned();

    {
        let input_dir_path = Path::new(&input_dir_expanded);
        assert!(input_dir_path.is_dir());
        assert!(input_dir_path.is_absolute());
    }

    ProcessedArgs {
        input_dir: input_dir_expanded,
        table_mode: if matches.get_flag(compact_arg.get_id().as_str()) {
            TableMode::Copmpact
        } else {
            TableMode::Detailed
        },
    }
}
