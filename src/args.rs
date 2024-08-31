use clap::{Parser, ValueEnum};
use std::path::Path;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value="~/Library/MobileDevice/Provisioning Profiles")]
    pub dir: Box<str>,

    #[arg(short, long, value_enum, default_value_t=CompactSortBy::Name)]
    pub sort_by: CompactSortBy,

    #[arg(short='o', long="order", value_enum, default_value_t=SortOrder::Asc)]
    pub sort_order: SortOrder,

    #[arg(short, long, value_enum, default_value_t=TableMode::Compact)]
    pub mode: TableMode,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum CompactSortBy {
    Name,
    AppIdName,
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
    Desc
}

pub fn get_processed_args() -> Args {
    let args = Args::parse();
    let input_dir_expanded = shellexpand::tilde(&args.dir);

    {
        let input_dir_path = Path::new(input_dir_expanded.as_ref());
        assert!(input_dir_path.is_dir());
        assert!(input_dir_path.is_absolute());
    }

    Args {
        dir: input_dir_expanded.into(),
        mode: args.mode,
        sort_by: args.sort_by,
        sort_order: args.sort_order,
    }
}
