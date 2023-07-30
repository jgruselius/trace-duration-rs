use anyhow::{bail, ensure, Context, Result};
use chrono::{Duration, NaiveDateTime};
use clap::{Parser, Args};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use log::{debug, info, LevelFilter};
use regex::Regex;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    from: FromMode,

    #[command(flatten)]
    to: ToMode,

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

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
struct FromMode {
    /// The pattern that defines the start
    #[arg(short, long, value_name="PATTERN")]
    from: Option<String>,

    /// The pattern that defines the start (last match)
    #[arg(short='F', long="from-last", value_name="PATTERN")]
    fromlast: Option<String>,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
struct ToMode {
    /// The pattern that defines the end
    #[arg(short, long, value_name="PATTERN")]
    to: Option<String>,

    /// The pattern that defines the end (last match)
    #[arg(short='T', long="to-last", value_name="PATTERN")]
    tolast: Option<String>,
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
        .init();

    let from = matches.from;
    let (p1, p1_replace) = match from.from {
        Some(from) => (from, false),
        _ => (from.fromlast.unwrap(), true),
    };
    let to = matches.to;
    let (p2, p2_replace) = match to.to {
        Some(to) => (to, false),
        _ => (to.tolast.unwrap(), true),
    };
    let d = if matches.regex {
        run_regex(matches.file, p1.clone(), p2.clone(), p1_replace, p2_replace)?
    } else {
        run(matches.file, p1.clone(), p2.clone(), p1_replace, p2_replace)?
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

fn run_regex(in_path: PathBuf, pattern1: String, pattern2: String, p1_replace: bool, p2_replace: bool) -> Result<Duration> {
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
        if (!from_found || p1_replace) && re1.is_match(&l) {
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
        if from_found && re2.is_match(&l) {
            info!("Matching line: {}", &l);
            let timestamp = re_ts
                .captures(&l)
                .context("Could not match a timestamp")?
                .get(0)
                .context("Could not parse a timestamp")?
                .as_str();
            to = parse_datetime(timestamp.to_string()).ok();
            to_found = true;
            if !p2_replace {
                break
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

fn run(in_path: PathBuf, pattern1: String, pattern2: String, p1_replace: bool, p2_replace: bool) -> Result<Duration> {
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
        if (!from_found || p1_replace) && l.contains(&pattern1) {
            info!("Matching line: {}", &l);
            let (timestamp, _) = (l).split_once('>').unwrap();
            from = parse_datetime(timestamp.to_string()).ok();
            from_found = true;
        }
        if from_found && l.contains(&pattern2) {
            info!("Matching line: {}", &l);
            let (timestamp, _) = (l).split_once('>').unwrap();
            to = parse_datetime(timestamp.to_string()).ok();
            to_found = true;
            if !p2_replace {
                break
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

fn check_file(s: &str) -> Result<PathBuf> {
    let path = PathBuf::from(s);
    ensure!(path.exists(), "{} does not exist", path.display());
    ensure!(path.is_file(), "{} is not a file", path.display());
    Ok(path)
}
