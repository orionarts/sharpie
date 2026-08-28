mod cli;
mod gui;
mod calc;
mod editor;

use clap::Parser;

use std::error::Error;

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    cli::run(cli::Cli::parse())
}
