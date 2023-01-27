use clap::{App, Arg};
use regex::Regex;
use std::{error::Error, path};
use walkdir::{DirEntry, WalkDir};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Eq, PartialEq)]
enum EntryType {
    Dir,
    File,
    Link,
}

use crate::EntryType::*;

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Vec<Regex>,
    entry_types: Vec<EntryType>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("findr")
        .version("0.1.0")
        .author("Anup Jadhav <anup.jadhav@gmail.com>")
        .about("Rust find")
        .arg(
            Arg::with_name("paths")
                .value_name("PATH")
                .help("Search paths")
                .default_value(".")
                .multiple(true),
        )
        .arg(
            Arg::with_name("names")
                .value_name("NAME")
                .short("n")
                .long("name")
                .help("Name")
                .takes_value(true)
                .multiple(true),
        )
        .arg(
            Arg::with_name("types")
                .value_name("TYPE")
                .short("t")
                .long("type")
                .help("Entry type")
                .possible_values(&["f", "d", "l"])
                .multiple(true)
                .takes_value(true),
        )
        .get_matches();

    // transform `Vec<String>` to `Vec<Regex>`
    let names = matches
        .values_of_lossy("names")
        .map(|vals| {
            vals.into_iter()
                .map(|name| {
                    Regex::new(&name)
                        .map_err(|_| format!("Invalid --name \"{}\"", name))
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    // clap should disallow anything but
    // "d", "f", or "l"
    let entry_types = matches
        .values_of_lossy("types")
        .map(|vals| {
            vals.iter()
                .map(|val| match val.as_str() {
                    "d" => Dir,
                    "f" => File,
                    "l" => Link,
                    _ => unreachable!("Invalid type"),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Config {
        paths: matches.values_of_lossy("paths").unwrap(),
        names,
        entry_types,
    })
}

// pub fn run(config: Config) -> MyResult<()> {
//     for path in config.paths {
//         for entry in WalkDir::new(path) {
//             match entry {
//                 Err(e) => eprintln!("{}", e),
//                 Ok(entry) => {
//                     if (config.entry_types.is_empty()
//                         || config.entry_types.iter().any(|entry_type| {
//                             match entry_type {
//                                 Link => entry.file_type().is_symlink(),
//                                 Dir => entry.file_type().is_dir(),
//                                 File => entry.file_type().is_file(),
//                             }
//                         }))
//                         && (config.names.is_empty()
//                             || config.names.iter().any(|re| {
//                                 re.is_match(
//                                     &entry.file_name().to_string_lossy(),
//                                 )
//                             }))
//                     {
//                         println!("{}", entry.path().display());
//                     }
//                 }
//             }
//         }
//     }
//     Ok(())
// }

pub fn run(config: Config) -> MyResult<()> {
    // Check if any of the `config.entry_types`
    // match the `DirEntry::file_type`
    let type_filter = |entry: &DirEntry| {
        config.entry_types.is_empty()
            || config.entry_types.iter().any(|entry_type| match entry_type {
                Link => entry.path_is_symlink(),
                Dir => entry.file_type().is_dir(),
                File => entry.file_type().is_file(),
            })
    };

    // remove filenames that don't match
    // one of the given regex
    let name_filter = |entry: &DirEntry| {
        config.names.is_empty()
            || config
                .names
                .iter()
                .any(|re| re.is_match(&entry.file_name().to_string_lossy()))
    };

    for path in &config.paths {
        let entries = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| match e {
                Err(err) => {
                    eprintln!("{}", err);
                    None
                }
                Ok(entry) => Some(entry),
            })
            .filter(type_filter)
            .filter(name_filter)
            // transform `DirEntry` to `String` values
            .map(|entry| entry.path().display().to_string())
            .collect::<Vec<_>>();
        println!("{}", entries.join("\n"));
    }
    Ok(())
}
