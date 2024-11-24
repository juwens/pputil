use clap::{Parser, Subcommand, ValueEnum};
use std::{ffi::OsString, path::PathBuf};

const XC_16_DIR: &str = "~/Library/Developer/Xcode/UserData/Provisioning Profiles";
const XC_15_DIR: &str = "~/Library/MobileDevice/Provisioning Profiles";

// https://github.com/clap-rs/clap/blob/10c29ab75dfc15ae8ae7218d699de5a2b57afabe/clap_builder/src/output/help_template.rs#L64
// https://github.com/clap-rs/clap/blob/10c29ab75dfc15ae8ae7218d699de5a2b57afabe/tests/derive/utils.rs#L11
const HELP_TEMPLATE: &str = "\
{before-help}{name} {version}
{author}
{author-with-newline}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}\
    ";

#[derive(Parser, std::fmt::Debug)]
#[command(version, about, long_about = None, help_template(HELP_TEMPLATE))]
pub struct MyCliArgs {
    #[command(subcommand)]
    pub command: Commands,

    /// Override the profile lookup directory with a custom path.
    /// Can be necessary/helpful, when Xcode changed the path, and pputil wasn't updated yet.
    /// Should be not needed usually.
    /// Usage: `cargo run -- --dir "/tmp/" list`
    #[arg(short('d'), long("dir"), global(true))]
    #[clap(hide = true)]
    custom_dir: Vec<String>,

    #[arg(
        long,
        short = 'v',
        action = clap::ArgAction::Count,
        global = true,
        help = "Increase logging verbosity",
    )]
    verbose: u8,
}

impl MyCliArgs {
    pub fn actual_dirs(&self) -> Vec<XcProvisioningProfileDir> {
        let relative_dirs = if self.custom_dir.is_empty() {
            vec![
                XcProvisioningProfileDir {
                    relative_path: XC_16_DIR.into(),
                    kind: XcProvisioningProfileDirKind::Xc16,
                },
                XcProvisioningProfileDir {
                    relative_path: XC_15_DIR.into(),
                    kind: XcProvisioningProfileDirKind::Xc15,
                },
            ]
        } else {
            self.custom_dir
                .iter()
                .map(|x| XcProvisioningProfileDir {
                    relative_path: OsString::from(x),
                    kind: XcProvisioningProfileDirKind::Custom,
                })
                .collect()
        };

        relative_dirs
    }
}

impl std::fmt::Debug for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commands::List(args) => f.debug_tuple("ListCompact").field(args).finish(),
            Commands::ListExtended(args) => f.debug_tuple("ListExtended").field(args).finish(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct XcProvisioningProfileDir {
    pub relative_path: std::ffi::OsString,
    pub kind: XcProvisioningProfileDirKind,
}
impl XcProvisioningProfileDir {
    pub(crate) fn absolute_path(&self) -> PathBuf {
        let binding = self.relative_path.to_string_lossy().into_owned();
        let absolute = shellexpand::tilde(binding.as_str());
        let s = absolute.into_owned();
        PathBuf::from(s)
    }
}
impl std::fmt::Display for XcProvisioningProfileDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("")
            .field("path", &self.relative_path)
            .field("xc", &self.kind)
            .finish()
    }
}

#[derive(Debug, Parser)]
pub struct ListCompactArgs {
    #[arg(value_enum, short, long, value_enum, default_value_t=CompactSortBy::Name)]
    pub sort_by: CompactSortBy,

    #[arg(value_enum, short='o', long="order", default_value_t=SortOrder::Asc)]
    pub sort_order: SortOrder,

    /// Prevent truncation of text. Instead wrap long strings into a new line.
    #[arg(short = 'w', long = "wrap", default_value_t = false)]
    pub allow_wrap: bool,
}

#[derive(Parser, Debug)]
pub struct ListExtendedArgs {}

#[derive(Subcommand)]
#[command()]
pub enum Commands {
    /// print a compact table with one profile per line. Ideal for a quick overview of all profiles.
    #[command(name = "list")]
    List(ListCompactArgs),
    /// print an extended table with several lines per profile. When you need more infos.
    #[command(name = "list-ext")]
    ListExtended(ListExtendedArgs),
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

#[derive(Debug, ValueEnum, Clone, PartialEq)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy)]
pub enum XcProvisioningProfileDirKind {
    /// less or equal
    Xc15 = 1,
    /// greater or equal
    Xc16 = 2,
    Custom,
}

#[allow(clippy::assigning_clones)]
pub fn get_processed_args() -> MyCliArgs {
    let args = MyCliArgs::parse();

    if args.verbose > 0 {
        dbg!(&args);
    }

    args
}
