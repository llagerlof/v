mod cli;
mod config;
mod highlight;
mod pager;
mod viewer;
mod wrap;

use std::process;

use crate::cli::{format_help, format_version, parse_matches, resolve};
use crate::config::Config;

fn main() {
    let (config_path, config) = match Config::ensure() {
        Ok(result) => result,
        Err(err) => {
            eprintln!("v: {err}");
            process::exit(1);
        }
    };

    let matches = match parse_matches() {
        Ok(matches) => matches,
        Err(err) => {
            err.exit();
        }
    };

    if matches.get_flag("version") {
        println!("{}", format_version());
        return;
    }

    if matches.get_flag("help") || matches.get_one::<std::path::PathBuf>("file").is_none() {
        print!("{}", format_help(&config_path));
        println!();
        return;
    }

    let resolved = resolve(&matches, &config);

    if let Err(err) = viewer::run(&resolved) {
        eprintln!("v: {err}");
        process::exit(1);
    }
}
