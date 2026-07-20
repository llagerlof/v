mod cli;
mod highlight;
mod pager;
mod viewer;
mod wrap;

use std::process;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();

    if let Err(err) = viewer::run(&cli) {
        eprintln!("v: {err}");
        process::exit(1);
    }
}
