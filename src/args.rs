use clap::{Parser, Subcommand, ValueEnum};
use std::{ffi::OsString, path::{Path, PathBuf}, str::FromStr};

const XC_16_DIR: &str = "~/Library/Developer/Xcode/UserData/Provisioning Profiles";
const XC_15_DIR: &str = "~/Library/MobileDevice/Provisioning Profiles";

#[derive(Parser, std::fmt::Debug)]
#[command(version, about, long_about = None)]
pub struct MyCliArgs {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Override the profile lookup directory with a custom path.
    /// Can be necessary/helpful, when Xcode changed the path, and pputil wasn't updated yet.
    /// Should be not needed usually.
    /// Usage: `cargo run -- --dir "/tmp/" list`
    #[arg(short('d'), long("dir"))]
    #[clap(hide=true)]
    pub custom_dir: Vec<String>,

    // the default and/or expanded paths
    #[clap(skip)]
    pub dirs_ex: Vec<XcProvisioningProfileDir>,

    #[arg(short, long, value_enum, default_value_t=TableMode::Compact)]
    pub mode: TableMode,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}

impl std::fmt::Debug for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ListCompact(arg0) => f.debug_tuple("ListCompact").field(arg0).finish(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct XcProvisioningProfileDir {
    pub path: std::ffi::OsString,
    pub kind: XcProvisioningProfileDirKind,
}
impl XcProvisioningProfileDir {
    pub(crate) fn path_as_path(&self) -> PathBuf {
        let expanded = shellexpand::tilde(&self.path.to_str().unwrap()).to_string();
        let osstr: OsString = OsString::from_str(&expanded).unwrap();
        osstr.into()
    }
}
impl std::fmt::Display for XcProvisioningProfileDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("path", &self.path)
            .field("xc", &self.kind)
            .finish()
    }
}

#[derive(Parser)]
// #[derive(Args)]
#[derive(Debug)]
pub struct ListCompactCommandArgs {
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
    #[command(name = "list")]
    ListCompact(ListCompactCommandArgs),
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

#[allow(clippy::assigning_clones)]
pub fn get_processed_args() -> MyCliArgs {
    let mut args = MyCliArgs::parse();
    let input_dirs_expanded: &Vec<String> = &args
        .custom_dir
        .iter()
        .map(|x| shellexpand::tilde(&x).into())
        .collect();

    // plausibility check for dirs
    for dir in input_dirs_expanded {
        let input_dir_path = Path::new(&dir);
        assert!(input_dir_path.is_dir());
        assert!(input_dir_path.is_absolute());
    }

    args.custom_dir = input_dirs_expanded.clone();
    
    if args.custom_dir.is_empty() {
        args.dirs_ex = vec![
            XcProvisioningProfileDir{path: XC_16_DIR.into(), kind: XcProvisioningProfileDirKind::Xc16},
            XcProvisioningProfileDir{path: XC_15_DIR.into(), kind: XcProvisioningProfileDirKind::Xc15},
        ];
        args.custom_dir = args.dirs_ex.iter().map(|x|x.path.to_string_lossy().to_string()).collect();
    }
    args.command = {
        let a = args.command.unwrap_or_else(|| {
            let cargs = ListCompactCommandArgs::parse();
            Commands::ListCompact(cargs)
        });
        Some(a)
    };

    if args.verbose.is_present() {
        dbg!(&args);
    }

    args
}

#[derive(Debug, Clone, Copy)]
pub enum XcProvisioningProfileDirKind {
    /// less or equal
    Xc15 = 1,
    /// greater or equal
    Xc16 = 2,
}