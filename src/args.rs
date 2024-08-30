use clap::{Parser, ValueEnum};
use std::path::Path;

#[derive(Debug)]
pub struct ProcessedArgs {
    pub input_dir: Box<str>,
    pub table_mode: TableMode,
    pub compact_sort_by: CompactSortBy,
    pub sort_order: SortOrder,
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

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value="~/Library/MobileDevice/Provisioning Profiles")]
    dir: Box<str>,

    #[arg(short, long, value_enum, default_value_t=CompactSortBy::Name)]
    sort_by: CompactSortBy,

    #[arg(short='o', long="order", value_enum, default_value_t=SortOrder::Asc)]
    sort_order: SortOrder,

    #[arg(short, long, value_enum, default_value_t=TableMode::Compact)]
    mode: TableMode,
}

pub fn get_processed_args() -> ProcessedArgs {
    let args = Args::parse();
    let input_dir_expanded = shellexpand::tilde(&args.dir);

    {
        let input_dir_path = Path::new(input_dir_expanded.as_ref());
        assert!(input_dir_path.is_dir());
        assert!(input_dir_path.is_absolute());
    }

    ProcessedArgs {
        input_dir: input_dir_expanded.into(),
        table_mode: args.mode,
        compact_sort_by: args.sort_by,
        sort_order: args.sort_order,
    }
}
