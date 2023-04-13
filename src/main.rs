use anyhow::{bail, ensure, Context, Result};
use chrono::{Duration, NaiveDateTime};
use clap::Parser;
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use log::{info, LevelFilter};
use regex::Regex;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The pattern that defines the start
    #[arg(short, long, value_name="PATTERN")]
    from: String,

    /// The pattern that defines the end
    #[arg(short, long, value_name="PATTERN")]
    to: String,

    /// The trace file to search
    #[arg(value_name="FILE", value_parser=check_file)]
    file: PathBuf,

    /// Use regex patterns
    #[arg(short, long)]
    regex: bool,

    /// Only print the duration
    #[arg(short, long)]
    short: bool,

    /// Print matching lines
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let matches = Cli::parse();

    let log_level = if matches.verbose {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };
    env_logger::builder()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    let p1 = matches.from;
    let p2 = matches.to;

    let d = if matches.regex {
        run(matches.file, p1.clone(), p2.clone())?
    } else {
        run_regex(matches.file, p1.clone(), p2.clone())?
    };

    if matches.short {
        println!("{}", format_duration(&d));
    } else {
        println!(
            "\"{}\" => \"{}\": {} (hh:mm:ss)",
            p1,
            p2,
            format_duration(&d)
        );
    }

    Ok(())
}

fn format_duration(d: &Duration) -> String {
    let sign = match d.num_seconds() {
        s if s < 0 => "–",
        _ => "+",
    };
    let total_secs = d.num_seconds().abs();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 60 / 60;
    format!("{}{:0>2}:{:0>2}:{:0>2}", sign, hours, mins, secs)
}

fn run_regex(in_path: PathBuf, pattern1: String, pattern2: String) -> Result<Duration> {
    let re_ts = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}")?;
    let re1 = Regex::new(pattern1.as_str())
        .with_context(|| format!("'{}' is not a valid regex", pattern1))?;
    let re2 = Regex::new(pattern2.as_str())
        .with_context(|| format!("'{}' is not a valid regex", pattern1))?;
    let mut from_found = false;
    let mut to_found = false;
    let mut from: Option<NaiveDateTime> = None;
    let mut to: Option<NaiveDateTime> = None;
    let file = OpenOptions::new().read(true).open(&in_path)?;
    let reader = BufReader::new(
        DecodeReaderBytesBuilder::new()
            .encoding(Some(WINDOWS_1252))
            .build(&file),
    );
    let mut l;
    for line in reader.lines() {
        l = line?;
        if !from_found {
            if re1.is_match(&l) {
                info!("Matching line: {}", &l);
                let timestamp = re_ts
                    .captures(&l)
                    .context("Could not match a timestamp")?
                    .get(0)
                    .context("Could not parse a timestamp")?
                    .as_str();
                from = parse_datetime(timestamp.to_string()).ok();
                from_found = true;
            }
        } else if re2.is_match(&l) {
                info!("Matching line: {}", &l);
                let timestamp = re_ts
                    .captures(&l)
                    .context("Could not match a timestamp")?
                    .get(0)
                    .context("Could not parse a timestamp")?
                    .as_str();
                to = parse_datetime(timestamp.to_string()).ok();
                to_found = true;
                break;
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

fn run(in_path: PathBuf, pattern1: String, pattern2: String) -> Result<Duration> {
    let mut from_found = false;
    let mut to_found = false;
    let mut from: Option<NaiveDateTime> = None;
    let mut to: Option<NaiveDateTime> = None;
    let file = OpenOptions::new().read(true).open(&in_path)?;
    let reader = BufReader::new(
        DecodeReaderBytesBuilder::new()
            .encoding(Some(WINDOWS_1252))
            .build(&file),
    );
    let mut l;
    for line in reader.lines() {
        l = line?;
        if !from_found {
            if l.contains(&pattern1) {
                info!("Matching line: {}", &l);
                let (timestamp, _) = (l).split_once('>').unwrap();
                from = parse_datetime(timestamp.to_string()).ok();
                from_found = true;
            }
        } else if l.contains(&pattern2) {
                info!("Matching line: {}", &l);
                let (timestamp, _) = (l).split_once('>').unwrap();
                to = parse_datetime(timestamp.to_string()).ok();
                to_found = true;
                break;
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

fn check_file(s: &str) -> Result<PathBuf> {
    let path = PathBuf::from(s);
    ensure!(path.exists(), "{} does not exist", path.display());
    ensure!(path.is_file(), "{} is not a file", path.display());
    Ok(path)
}
