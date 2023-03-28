use log::{info, LevelFilter};
use env_logger;
use std::fs::OpenOptions;
use std::io::{BufReader, BufRead};
use std::path::{PathBuf};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use chrono::{NaiveDateTime, Duration};
use anyhow::{bail, ensure, Context, Result};
use clap::{App, Arg};
use regex::Regex;

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
                //.validator(check_arg)
                .required(true),
        )
        .arg(
            Arg::new("to")
                .help("The pattern that defines the end")
                .long("to")
                .short('t')
                .takes_value(true)
                .value_name("PATTERN")
                //.validator(check_arg)
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
            Arg::new("regex")
                .help("Use regex patterns")
                .long("regex")
                .short('r')
                .takes_value(false)
                .required(false),
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
    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    let p1 = matches.get_arg("from")?;
    let p2 = matches.get_arg("to")?;
    let path = PathBuf::from(matches.get_arg("file")?);

    let d = match matches.occurrences_of("regex") {
        0 => run(path, p1, p2)?,
        _ => run_regex(path, p1, p2)?,
    };

    println!("Duration: {}", format_duration(&d));

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

fn format_duration(d: &Duration) -> String {
    let sign = match d.num_seconds() {
        s if s < 0 => "–",
        _ => "+"
    };
    let total_secs = d.num_seconds().abs();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 60 / 60;
    format!("({}{:0>2}:{:0>2}:{:0>2})", sign, hours, mins, secs)
}

fn run_regex(in_path: PathBuf, pattern1: String, pattern2: String) -> Result<Duration> {
    let re_ts = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}")?;
    let re1 = Regex::new(pattern1.as_str())
        .with_context(|| format!("'{}' is not a valid regex", pattern1))?;
    let re2 = Regex::new(&*pattern2.as_str())
        .with_context(|| format!("'{}' is not a valid regex", pattern1))?;
    let mut from_found = false;
    let mut to_found = false;
    let mut from: Option<NaiveDateTime> = None;
    let mut to: Option<NaiveDateTime> = None;
/*    let reader = BufReader::new(
        DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(OpenOptions::new().read(true).open(&in_path)?));*/
    let file = OpenOptions::new().read(true).open(&in_path)?;
    let reader = BufReader::new(DecodeReaderBytesBuilder::new()
            .encoding(Some(WINDOWS_1252))
            .build(&file));
    let mut l;
    for line in reader.lines() {
        l = line?;
        if !from_found {
            if re1.is_match(&l) {
                info!("Matching line: {}", &l);
                let timestamp = re_ts.captures(&l)
                    .context("Could not match a timestamp")?
                    .get(0).context("Could not parse a timestamp")?.as_str();
                from = parse_datetime(timestamp.to_string()).ok();
                from_found = true;
            }
        } else {
            if re2.is_match(&l) {
                info!("Matching line: {}", &l);
                let timestamp = re_ts.captures(&l)
                    .context("Could not match a timestamp")?
                    .get(0).context("Could not parse a timestamp")?.as_str();
                to = parse_datetime(timestamp.to_string()).ok();
                to_found = true;
                break;
            }
        }
    }
    let duration = match (from, to) {
        (Some(t1), Some(t2)) => t2 - t1,
        _ => bail!("Could not parse a timestamp"),
    };

    Ok(duration)
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
    let reader = BufReader::new(DecodeReaderBytesBuilder::new()
            .encoding(Some(WINDOWS_1252))
            .build(&file));
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
