use clap::{App, Arg};
use rand::prelude::SliceRandom;
use rand::{rngs::StdRng, SeedableRng};
use regex::{Regex, RegexBuilder};
use std::{
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    sources: Vec<String>,   // input files or directories
    pattern: Option<Regex>, // pattern to match the fortune
    seed: Option<u64>,      // seed for selecting random fortune
}

#[derive(Debug)]
struct Fortune {
    source: String,
    text: String,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("grepr")
        .version("0.1.0")
        .author("Anup Jadhav <anup.jadhav@gmail.com")
        .about("Rust fortune")
        .arg(
            Arg::with_name("pattern")
                .value_name("PATTERN")
                .short("m")
                .long("pattern")
                .help("Pattern"),
        )
        .arg(
            Arg::with_name("sources")
                .value_name("FILE")
                .help("Input files or directories")
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("insensitive")
                .short("i")
                .long("insensitive")
                .help("Case-insensitive pattern matching")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("seed")
                .value_name("SEED")
                .short("s")
                .long("seed")
                .help("Random seed"),
        )
        .get_matches();

    let pattern = matches
        .value_of("pattern")
        .map(|val| {
            RegexBuilder::new(val)
                .case_insensitive(matches.is_present("insensitive"))
                .build()
                .map_err(|_| format!("Invalid --pattern \"{}\"", val))
        })
        .transpose()?;
    Ok(Config {
        sources: matches.values_of_lossy("sources").unwrap(),
        seed: matches.value_of("seed").map(parse_u64).transpose()?,
        pattern,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let files = find_files(&config.sources)?;
    let fortunes = read_fortunes(&files)?;
    // Check if user has provided a pattern
    if let Some(pattern) = config.pattern {
        let mut prev_source = None;

        // Print all fortunes
        // where the pattern is a match
        for fortune in
            fortunes.iter().filter(|fortune| pattern.is_match(&fortune.text))
        {
            // print the source header if the
            // current source is not the same as the
            // previous one
            if prev_source.as_ref().map_or(true, |s| s != &fortune.source) {
                eprintln!("({})\n%", fortune.source);
                prev_source = Some(fortune.source.clone());
            }
            println!("{}\n%", fortune.text);
        }
    } else {
        // Select and print one fortune
        println!(
            "{}",
            pick_fortune(&fortunes, config.seed)
                .or_else(|| Some("No fortunes found".to_string()))
                .unwrap()
        )
    }
    Ok(())
}

fn parse_u64(val: &str) -> MyResult<u64> {
    val.parse().map_err(|_| format!("\"{}\" not a valid integer", val).into())
}

// ---------------------------------------------------------
// Finds the files listed in the `paths` variable,
// filters out files that have the `.dat` extension.
fn find_files(paths: &[String]) -> MyResult<Vec<PathBuf>> {
    let dat = OsStr::new("dat");
    let mut files = vec![];

    for path in paths {
        match fs::metadata(path) {
            // if `fs::metadata` fails, provide an useful error
            Err(e) => return Err(format!("{}: {}", path, e).into()),
            // Use `vec::extend` to add the results from `WalkDir`
            // iterator to the results
            Ok(_) => files.extend(
                WalkDir::new(path)
                    .into_iter()
                    // this will ignore errors for
                    // unreadable files or directories,
                    // which is the behaviour of the original
                    // program
                    .filter_map(Result::ok)
                    .filter(|e| {
                        e.file_type().is_file()
                            && e.path().extension() != Some(dat)
                    })
                    .map(|e| e.path().into()),
            ),
        }
    }

    // sort in place
    files.sort();

    // remove consecutive repeated values
    files.dedup();

    Ok(files)
}

// the files found by `find_files()` are
// inputs to this function
fn read_fortunes(paths: &[PathBuf]) -> MyResult<Vec<Fortune>> {
    let mut fortunes = vec![];
    let mut buffer = vec![];

    for path in paths {
        // Convert `Path::file_name` from `OsStr` to
        // `String`
        let basename =
            path.file_name().unwrap().to_string_lossy().into_owned();

        // Open the file or return an error message
        let file = File::open(path).map_err(|e| {
            format!("{}: {}", path.to_string_lossy().into_owned(), e)
        })?;

        // Now iterate through the lines of the file
        for line in BufReader::new(file).lines().filter_map(Result::ok) {
            // the percent sign % indicates the
            // end of a record
            if line == "%" {
                // if the buffer is not empty
                // set the text to the buffer lines joined
                // on newlines and then clear the buffer
                if !buffer.is_empty() {
                    fortunes.push(Fortune {
                        source: basename.clone(),
                        text: buffer.join("\n"),
                    });
                    buffer.clear();
                }
            } else {
                // if not % then push the current
                // line to the buffer
                buffer.push(line.to_string());
            }
        }
    }

    Ok(fortunes)
}

fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    if let Some(val) = seed {
        // create a PRNG using the provided seed
        let mut rng = StdRng::seed_from_u64(val);

        // `choose` returns a reference to one
        // random element from the slice
        fortunes.choose(&mut rng).map(|f| f.text.to_string())
    } else {
        // othwerise use a PRNG seeded by the system
        let mut rng = rand::thread_rng();
        fortunes.choose(&mut rng).map(|f| f.text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{find_files, parse_u64, read_fortunes};
    use std::path::PathBuf;

    #[test]
    fn test_parse_u64() {
        let res = parse_u64("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "\"a\" not a valid integer");

        let res = parse_u64("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 0);

        let res = parse_u64("4");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 4);
    }

    #[test]
    fn test_find_files() {
        // Verify that the function finds a file known to exist
        let res = find_files(&["./tests/inputs/jokes".to_string()]);
        assert!(res.is_ok());

        let files = res.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files.get(0).unwrap().to_string_lossy(),
            "./tests/inputs/jokes"
        );

        // Fails to find a bad file
        let res = find_files(&["/path/does/not/exist".to_string()]);
        assert!(res.is_err());

        // Finds all the input files, excludes ".dat"
        let res = find_files(&["./tests/inputs".to_string()]);
        assert!(res.is_ok());

        // Check number and order of files
        let files = res.unwrap();
        assert_eq!(files.len(), 5);
        let first = files.get(0).unwrap().display().to_string();
        assert!(first.contains("ascii-art"));
        let last = files.last().unwrap().display().to_string();
        assert!(last.contains("quotes"));

        // Test for multiple sources, path must be unique and sorted
        let res = find_files(&[
            "./tests/inputs/jokes".to_string(),
            "./tests/inputs/ascii-art".to_string(),
            "./tests/inputs/jokes".to_string(),
        ]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 2);
        if let Some(filename) = files.first().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "ascii-art".to_string())
        }
        if let Some(filename) = files.last().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "jokes".to_string())
        }
    }

    #[test]
    fn test_read_fortunes() {
        // One input file
        let res = read_fortunes(&[PathBuf::from("./tests/inputs/jokes")]);
        assert!(res.is_ok());

        if let Ok(fortunes) = res {
            // Correct number and sorting
            assert_eq!(fortunes.len(), 6);
            assert_eq!(
                fortunes.first().unwrap().text,
                "Q. What do you call a head of lettuce in a shirt and tie?\n\
                A. Collared greens."
            );
            assert_eq!(
                fortunes.last().unwrap().text,
                "Q: What do you call a deer wearing an eye patch?\n\
                A: A bad idea (bad-eye deer)."
            );
        }

        // Multiple input files
        let res = read_fortunes(&[
            PathBuf::from("./tests/inputs/jokes"),
            PathBuf::from("./tests/inputs/quotes"),
        ]);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().len(), 11);
    }
}
