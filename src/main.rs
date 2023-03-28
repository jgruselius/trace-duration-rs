use pretty_env_logger;
use log::{info, LevelFilter};
use std::fs::OpenOptions;
use std::io::{Write, BufReader, BufRead};
use std::path::{Path, PathBuf};
//use std::time::Duration;
use chrono::{NaiveDateTime, Duration};
use chrono::format::ParseError;
use anyhow::{bail, ensure, Context, Result};
use clap::{App, Arg};
use console::{style, Style};
use regex::Regex;
use humantime::format_duration;

fn main() -> Result<()> {

    let matches = App::new("<app_name>")
        .version("0.1")
        .author("Joel Gruselius <joel.gruselius@perkinelmer.com>")
        .about(
            "<description>",
        )
        .after_help(
            "<extra>"
        )
        .arg(
            Arg::new("from")
                .help("The pattern that defines the start")
                .long("from")
                .short('f')
                .takes_value(true)
                .value_name("PATTERN")
                .validator(check_arg)
                .required(true),
        )
        .arg(
            Arg::new("to")
                .help("The pattern that defines the end")
                .long("to")
                .short('t')
                .takes_value(true)
                .value_name("PATTERN")
                .validator(check_arg)
                .required(true),
        )
        .arg(
            Arg::new("file")
                .help("The trace file to search")
                .required(true)
                .value_name("FILE")
                .validator(|s| check_file(&PathBuf::from(s))),
        )
        .arg(
            Arg::new("verbose")
                .help("Print information about what goes on")
                .long("verbose")
                .short('v'),
        )
        .get_matches();

    let log_level = if matches.is_present("verbose") {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };
    pretty_env_logger::formatted_builder()
        .filter_level(log_level)
        .init();

    let p1 = matches.get_arg("from")?;
    let p2 = matches.get_arg("to")?;
    let path = PathBuf::from(matches.get_arg("file")?);

    let d = run(path, p1, p2)?;

    println!("Duration: {}", format_duration(d.to_std()?));

    Ok(())
}

trait ArgExt {
    fn get_arg(&self, arg: &str) -> Result<String>;
}

impl ArgExt for clap::ArgMatches {
    fn get_arg(&self, arg: &str) -> Result<String> {
        match self.value_of(arg) {
            Some(v) => Ok(v.to_string()),
            None => bail!("No argument matching {}", arg),
        }
    }
}

fn run_regex(in_path: PathBuf, pattern1: String, pattern2: String) -> Result<()> {
    let re_timestamp = Regex::new(r"$\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}")?;
    let re1 = Regex::new(pattern1.as_str())
        .with_context(|| format!("'{}' is not a valid regex", pattern1))?;
    let re2 = Regex::new(&*pattern2.as_str())
        .with_context(|| format!("'{}' is not a valid regex", pattern1))?;
    let mut from_found = false;
    let mut to_found = false;
/*    let reader = BufReader::new(
        DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(OpenOptions::new().read(true).open(&in_path)?));*/
    let file = OpenOptions::new().read(true).open(&in_path)?;
    let reader = BufReader::new(&file);
    let mut l;
    for line in reader.lines() {
        l = line?;
        if !from_found {
            if re1.is_match(&l) {
                info!("{}", &l);
                let (timestamp, _) = (&l).split_once(">").unwrap();
                let from = parse_datetime(timestamp.to_string())?;
                from_found = true;
            }
        } else {
            if re2.is_match(&l) {
                info!("{}", &l);
                let (timestamp, _) = (&l).split_once(">").unwrap();
                let to = parse_datetime(timestamp.to_string())?;
                to_found = true;
                break;
            }
        }
    }
    ensure!(from_found, format!("Did not find '{}'", pattern1));
    ensure!(to_found, format!("Did not find '{}'", pattern2));

    Ok(())
}

fn run(in_path: PathBuf, pattern1: String, pattern2: String) -> Result<Duration> {
    let mut from_found = false;
    let mut to_found = false;
    let mut from: Option<NaiveDateTime> = None;
    let mut to: Option<NaiveDateTime> = None;
/*    let reader = BufReader::new(
        DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(OpenOptions::new().read(true).open(&in_path)?));*/
    let file = OpenOptions::new().read(true).open(&in_path)?;
    let reader = BufReader::new(&file);
    let mut l;
    for line in reader.lines() {
        l = line?;
        if !from_found {
            if (&l).contains(&pattern1) {
                info!("Matching line: {}", &l);
                let (timestamp, _) = (&l).split_once(">").unwrap();
                from = parse_datetime(timestamp.to_string()).ok();
                from_found = true;
            }
        } else {
            if (&l).contains(&pattern2) {
                info!("Matching line: {}", &l);
                let (timestamp, _) = (&l).split_once(">").unwrap();
                to = parse_datetime(timestamp.to_string()).ok();
                to_found = true;
                break;
            }
        }
    }
    ensure!(from_found, format!("Did not find '{}'", pattern1));
    ensure!(to_found, format!("Did not find '{}'", pattern2));

    let duration = match (from, to) {
        (Some(t1), Some(t2)) => t2 - t1,
        _ => bail!("Could not parse a timestamp"),
    };

    Ok(duration)
}

fn parse_datetime(dt: String) -> Result<NaiveDateTime> {
        let datetime = NaiveDateTime::parse_from_str(&dt, "%Y-%m-%d %H:%M:%S")
            .with_context(|| format!("could not parse {}", dt))?;
    Ok(datetime)
}

fn check_arg(text: &str) -> Result<()> {
    check_str(text, r"[^\w\d_\.-]")
}

fn check_str<S>(text: S, pattern: &str) -> Result<()>
where
    S: AsRef<str>,
{
    let re = Regex::new(pattern).unwrap();
    match re.is_match(text.as_ref()) {
        false => Ok(()),
        true => bail!("Must not contain: {}", pattern),
    }
}

fn check_file(path: &PathBuf) -> Result<()> {
    ensure!(path.exists(), "{} does not exist", path.display());
    ensure!(path.is_file(), "{} is not a file", path.display());
    Ok(())
}
