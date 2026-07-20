mod cli;
mod config;
mod highlight;
mod pager;
mod viewer;
mod wrap;

use std::process;

use crate::cli::{build_command, resolve};
use crate::config::Config;

fn main() {
    let (config_path, config) = match Config::ensure() {
        Ok(result) => result,
        Err(err) => {
            eprintln!("v: {err}");
            process::exit(1);
        }
    };

    let matches = match build_command(&config, &config_path).try_get_matches() {
        Ok(matches) => matches,
        Err(err) => {
            err.exit();
        }
    };

    if matches.get_one::<std::path::PathBuf>("file").is_none() {
        let _ = build_command(&config, &config_path).print_help();
        println!();
        return;
    }

    let resolved = resolve(&matches, &config);

    if let Err(err) = viewer::run(&resolved) {
        eprintln!("v: {err}");
        process::exit(1);
    }
}
