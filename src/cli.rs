//! Command-line execution for the sharpie binary.

use clap::{Parser, Subcommand};
use crate::calc::{hull_draw, Ship};
use crate::gui;

use std::error::Error;

// Command line parsing {{{1
//
#[derive(Parser)]
#[command(version = concat!(
    env!("CARGO_PKG_VERSION"), "\n",
    "Copyright (C) 2024 Jeremy Brubaker\n",
    "License GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>.\n",
    "This is free software: you are free to change and redistribute it.\n",
    "There is NO WARRANTY, to the extent permitted by law.\n",
    "\n",
    "Written by Jeremy Brubaker.\n",
))]
#[command(about = "SpringSharp 3b3 clone", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[cfg(debug_assertions)]
    #[arg(short, long)]
    #[arg(help = "Show internal values")]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    Load {
        file: String,

        #[arg(short, long, num_args = 0..=1)]
        #[arg(help = "Write hull profile image (default name: <file stem>-hull.svg)")]
        image: Option<Option<String>>,

        #[arg(short, long)]
        #[arg(help = "Show ship report (the default)")]
        report: bool,
    },

    #[command(group(
        clap::ArgGroup::new("to_or_report")
            .args(["to", "report"])
            .multiple(true)
            .required(true)
    ))]
    Convert {
        #[arg(help = "SpringSharp 3 file to convert")]
        from: String,

        #[arg(short, long)]
        #[arg(help = "Filename to save conversion to")]
        to: Option<String>,

        #[arg(short, long)]
        #[arg(help = "Show ship report after conversion")]
        report: bool,

        #[arg(short, long, num_args = 0..=1)]
        #[arg(help = "Write hull profile image (default name: <file stem>-hull.svg)")]
        image: Option<Option<String>>,
    },
}

// Run the CLI {{{1
//
/// Derive the hull image filename from an input filename.
///
/// An explicit output name wins; otherwise use the input file's stem.
///
fn image_path(file: &str, out: Option<String>) -> String {
    out.unwrap_or_else(|| {
        let path = std::path::Path::new(file);
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("hull");
        format!("{stem}.svg")
    })
}

/// Write the hull side-profile SVG of a ship.
///
fn write_image(ship: &Ship, path: &str) -> Result<(), Box<dyn Error>> {
    std::fs::write(path, hull_draw::hull_svg(&ship.hull, &ship.name))?;
    println!("wrote {path}");

    Ok(())
}

/// Dispatch a parsed command line to the CLI subcommands or the GUI.
///
pub fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Some(Commands::Load { file, image, report: _ }) => {
            // Compute the image name before moving the input filename.
            //
            let image = image.map(|out| image_path(&file, out));

            match Ship::load(file) {
                Ok(ship) => {
                    println!("{}", ship.report());
                    #[cfg(debug_assertions)]
                    if cli.debug { eprintln!("{}", ship.internals()); }

                    match image {
                        Some(path) => write_image(&ship, &path),
                        None       => Ok(()),
                    }
                }

                Err(error) => Err(error),
            }
        }

        Some(Commands::Convert { from, to, report, image }) => {
            // Compute the image name before moving the input filename.
            //
            let image = image.map(|out| image_path(&from, out));

            match Ship::convert(from) {
                Ok(ship) => {
                    if report { println!("{}", ship.report()); }
                    #[cfg(debug_assertions)]
                    if cli.debug { eprintln!("{}", ship.internals()); }

                    if let Some(to) = to { ship.save(to)?; }

                    if let Some(path) = image { write_image(&ship, &path)?; }

                    Ok(())
                }

                Err(error) => Err(error),
            }
        }

        // No subcommand means launch the GUI
        None => gui::run_gui(),
    }
}

// Testing {{{1
//
// cli {{{2
#[cfg(test)]
mod cli {
    use super::*;
    use clap::Parser;

    // assert_matches! {{{3
    //
    // assert_matches! would require "nightly"
    //
    // Local replacement for std::assert_matches (unstable as of Rust 1.93).
    macro_rules! assert_matches {
        ($expression:expr, $(|)? $( $pattern:pat_param )|+ $( if $guard:expr )? $(,)?) => {
            match $expression {
                $( $pattern )|+ $( if $guard )? => {}
                _ => panic!(
                    "assertion failed: value did not match pattern `{}`",
                    stringify!($( $pattern )|+ $( if $guard )?)
                ),
            }
        };
    }

    // Test cli_parse_ok {{{3
    macro_rules! test_cli_parse_ok {
        ($($name:ident: ($args:expr, $($pattern:tt)+),)*) => {
            $(
                #[test]
                fn $name() {
                    let cli = Cli::try_parse_from($args).unwrap();
                    assert_matches!(cli.command, $($pattern)+);
                }
            )*
        }
    }

    test_cli_parse_ok! {
        // name: (args, pattern)
        cli_no_subcommand:
            (["sharpie"], None),
        cli_load:
            (["sharpie", "load", "ship.ship"],
             Some(Commands::Load {
                 ref file, report: false, image: None })
             if file == "ship.ship"),
        cli_load_image_bare:
            (["sharpie", "load", "ship.ship", "--image"],
             Some(Commands::Load {
                 ref file, report: false, image: Some(None) })
             if file == "ship.ship"),
        cli_load_image_value:
            (["sharpie", "load", "ship.ship", "-i", "out.svg"],
             Some(Commands::Load {
                 ref file, report: false, image: Some(ref image) })
             if file == "ship.ship" && *image == Some("out.svg".to_owned())),
        cli_load_report_long:
            (["sharpie", "load", "ship.ship", "--report"],
             Some(Commands::Load {
                 ref file, report: true, image: None })
             if file == "ship.ship"),
        cli_load_report_short:
            (["sharpie", "load", "ship.ship", "-r"],
             Some(Commands::Load {
                 ref file, report: true, image: None })
             if file == "ship.ship"),
        cli_convert_to_long:
            (["sharpie", "convert", "in.sship", "--to", "out.ship"],
             Some(Commands::Convert {
                 ref from, to: Some(ref to), report: false, image: None })
             if from == "in.sship" && to == "out.ship"),
        cli_convert_to_short:
            (["sharpie", "convert", "in.sship", "-t", "out.ship"],
             Some(Commands::Convert {
                 ref from, to: Some(ref to), report: false, image: None })
             if from == "in.sship" && to == "out.ship"),
        cli_convert_report_long:
            (["sharpie", "convert", "in.sship", "--report"],
            Some(Commands::Convert {
                ref from, to: None, report: true, image: None })
            if from == "in.sship"),
        cli_convert_report_short:
            (["sharpie", "convert", "in.sship", "-r"],
            Some(Commands::Convert {
                ref from, to: None, report: true, image: None })
            if from == "in.sship"),
        cli_convert_image_bare:
            (["sharpie", "convert", "in.sship", "--image", "--to", "out.ship"],
             Some(Commands::Convert {
                 ref from, to: Some(ref to), report: false, image: Some(None) })
             if from == "in.sship" && to == "out.ship"),
        cli_convert_image_value:
            (["sharpie", "convert", "in.sship", "-i", "out.svg", "--to", "out.ship"],
             Some(Commands::Convert {
                 ref from, to: Some(ref to), report: false, image: Some(ref image) })
             if from == "in.sship" && to == "out.ship" && *image == Some("out.svg".to_owned())),
        cli_convert_to_and_report:
            (["sharpie", "convert", "in.sship", "--to", "out.ship", "--report"],
             Some(Commands::Convert {
                 ref from, to: Some(ref to), report: true, image: None })
             if from == "in.sship" && to == "out.ship"),
        cli_convert_image_bare_report:
            (["sharpie", "convert", "in.sship", "--image", "-r"],
             Some(Commands::Convert {
                 ref from, to: None, report: true, image: Some(_) })
             if from == "in.sship"),
        cli_convert_image_value_report:
            (["sharpie", "convert", "in.sship", "-i", "out.svg", "-r"],
             Some(Commands::Convert {
                 ref from, to: None, report: true, image: Some(ref image) })
             if from == "in.sship" && *image == Some("out.svg".to_owned())),
    }

    // Test cli_parse_err {{{3
    macro_rules! test_cli_parse_err {
        ($($name:ident: ($args:expr),)*) => {
            $(
                #[test]
                fn $name() {
                    assert!(Cli::try_parse_from($args).is_err());
                }
            )*
        }
    }

    test_cli_parse_err! {
        // name:                    (args)
        cli_err_load:              (["sharpie", "load"]),
        cli_err_load_extra:        (["sharpie", "load", "a.ship", "b.ship"]),
        cli_err_convert:           (["sharpie", "convert"]),
        cli_err_convert_no_action: (["sharpie", "convert", "in.sship"]),
        cli_err_convert_to_noval:  (["sharpie", "convert", "in.sship", "--to"]),
        cli_err_bogus:             (["sharpie", "bogus"]),
    }

    // Test cli_debug {{{3
    #[cfg(debug_assertions)]
    #[test]
    fn cli_debug() {
        let cli = Cli::try_parse_from(["sharpie", "--debug", "load", "ship.ship"]).unwrap();
        assert!(cli.debug);
    }
}
