use crate::dir::*;
use crate::utils::eq;
use clap::{ArgGroup, Parser};
use std::path::PathBuf;
mod dir;
mod utils;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(group(
    ArgGroup::new("func").multiple(true).required(true).args(&["scan", "nuke"]),
))]
#[command(group(
    ArgGroup::new("input").required(true).args(&["config", "path"]),
))]
struct Args {
    /// Scans folders in config and calculate their size
    #[arg(short, long, group = "func")]
    scan: bool,
    /// Nuke all items under all folders in config
    #[arg(short, long, group = "func")]
    nuke: bool,
    /// Path to config file. Either this or --path must be present
    #[arg(short, long, group = "input", value_name = "CONFIG")]
    config: Option<PathBuf>,
    /// Manually specify folders to process. Either this or --config must be present
    #[arg(short, long, group = "input", value_name = "PATHS", value_delimiter = ' ', num_args = 1..)]
    path: Option<Vec<PathBuf>>,
    /// Suppresses all output except stderr
    #[arg(short, long)]
    quiet: bool,
}
fn main() {
    let args = Args::parse();
    let mut config: Vec<PathBuf> = Vec::new();
    if let Some(conf) = args.config {
        config = open_config(&conf).map_err(eq).unwrap();
    } else if let Some(vec) = args.path {
        config = vec;
    }
    let validated_list = validate(config, args.quiet).map_err(eq).unwrap();
    if !args.quiet {
        println!("{} valid entries detected in config", validated_list.len());
    }
    if args.scan {
        scan(&validated_list, args.quiet);
    }
    if args.nuke {
        nuke(&validated_list, args.quiet);
    }
}
