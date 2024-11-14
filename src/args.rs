use clap::{Parser, Subcommand, ValueEnum};
use std::path::Path;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(
        short,
        long,
        // default_value = "~/Library/MobileDevice/Provisioning Profiles", // XC 15
        default_values = vec![
            "~/Library/Developer/Xcode/UserData/Provisioning Profiles", // XC 16
            "~/Library/MobileDevice/Provisioning Profiles", // XC 15
        ]
    )]
    pub dirs: Vec<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, value_enum, default_value_t=TableMode::Compact)]
    pub mode: TableMode,
}

#[derive(Parser)]
// #[derive(Args)]
#[derive(Debug)]
pub struct PrintCompactCommandArgs {
    #[arg(value_enum, short, long, value_enum, default_value_t=CompactSortBy::Name)]
    pub sort_by: CompactSortBy,

    #[arg(value_enum, short='o', long="order", default_value_t=SortOrder::Asc)]
    pub sort_order: SortOrder,

    #[arg(short = 'w', long = "wrap", default_value_t = false)]
    pub allow_wrap: bool,
}

#[derive(Subcommand)]
#[command()]
pub enum Commands {
    #[command(name = "print")]
    PrintCompact(PrintCompactCommandArgs),
}

#[derive(Debug, ValueEnum, Clone)]
pub enum CompactSortBy {
    #[clap(name = "name")]
    Name,
    #[clap(name = "aidn")]
    AppIdName,
    #[clap(name = "expd")]
    ExpirationDate,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum TableMode {
    Compact,
    Detailed,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum SortOrder {
    Asc,
    Desc,
}

pub fn get_processed_args() -> Cli {
    let mut args = Cli::parse();
    let input_dirs_expanded: &Vec<String> = &args
        .dirs
        .iter()
        .map(|x| shellexpand::tilde(&x).into())
        .collect();

    // plausibility check for dirs
    for dir in input_dirs_expanded.iter() {
        let input_dir_path = Path::new(&dir);
        assert!(input_dir_path.is_dir());
        assert!(input_dir_path.is_absolute());
    }

    args.dirs = input_dirs_expanded.clone();
    args.command = {
        let a = args.command.unwrap_or_else(|| {
            let cargs = PrintCompactCommandArgs::parse();
            Commands::PrintCompact(cargs)
        });
        Some(a)
    };

    args
}
