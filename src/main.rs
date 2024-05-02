use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use clap::Parser;
use core::fmt::Debug;
use regex::Regex;
use std::io::{self, BufRead, BufReader};
use std::{fmt, fs};

use nmea0183;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    input_file_name: Option<String>,

    #[arg(long = "start", short, help = "earliest date/time to log in Utc")]
    start_date: Option<String>,

    #[arg(long = "end", short, help = "latest date/time to log in Utc")]
    end_date: Option<String>,

    #[arg(
        long = "devices",
        short = 'd',
        use_value_delimiter = true,
        help = "devices to include in output.  All devices if omitted."
    )]
    include_devices: Option<Vec<String>>,

    #[arg(
        long = "xdevices",
        short = 'D',
        use_value_delimiter = true,
        help = "devices to exclude from output.  No devices excluded if omitted."
    )]
    exclude_devices: Option<Vec<String>>,

    #[arg(
        long = "messages",
        short = 'm',
        use_value_delimiter = true,
        help = "messages to include in output.  All messages if omitted."
    )]
    include_messages: Option<Vec<String>>,

    #[arg(
        long = "xmessages",
        short = 'M',
        use_value_delimiter = true,
        help = "messages to be excluded from output.  No messages excluded if omitted."
    )]
    exclude_messages: Option<Vec<String>>,

    #[arg(
        long = "termeof",
        default_value_t = false,
        help = "terminate on end of file."
    )]
    terminate_on_eof: bool,

    #[arg(
        long = "termerr",
        default_value_t = false,
        help = "terminate on i/o error."
    )]
    terminate_on_err: bool,
}

struct NMEAFile {
    stream: Box<dyn BufRead>,
    start_timestamp: DateTime<Utc>,
    end_timestamp: DateTime<Utc>,
    include_devices: Regex,
    exclude_devices: Regex,
    include_messages: Regex,
    exclude_messages: Regex,
    most_recent_time: DateTime<Utc>,
    terminate_eof: bool,
    terminate_err: bool,
}

impl NMEAFile {
    fn new(cli: &Cli) -> Option<NMEAFile> {
        let reader: Box<dyn BufRead> = match cli.input_file_name.clone() {
            None => Box::new(BufReader::new(io::stdin())),
            Some(filename) => Box::new(BufReader::new(fs::File::open(filename).unwrap())),
        };

        let ndt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(1900, 1, 1)
                .expect("Could not create date 1900-01-01 as default start date."),
            NaiveTime::from_hms_opt(0, 0, 0)
                .expect("Could not create time of 00:00:00 as default start time."),
        );
        let start_timestamp = DateTime::from_naive_utc_and_offset(ndt, Utc);

        let ndt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2100, 12, 31)
                .expect("Could not create date 2100-12-31 as default end date."),
            NaiveTime::from_hms_opt(23, 59, 59)
                .expect("Could not create time of 23:59:59 as default end time."),
        );
        let end_timestamp = DateTime::from_naive_utc_and_offset(ndt, Utc);

        let include_devices = NMEAFile::create_regex(&cli.include_devices);
        let exclude_devices = NMEAFile::create_regex(&cli.exclude_devices);
        let include_messages = NMEAFile::create_regex(&cli.include_messages);
        let exclude_messages = NMEAFile::create_regex(&cli.exclude_messages);

        Some(NMEAFile {
            stream: reader,
            start_timestamp,
            end_timestamp,
            include_devices,
            exclude_devices,
            include_messages,
            exclude_messages,
            most_recent_time: DateTime::from_naive_utc_and_offset(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1900, 1, 1)
                        .expect("Could not create most recent date of 1900-01-01"),
                    NaiveTime::from_hms_opt(0, 0, 0)
                        .expect("Could not create most recent time of 00:00:00"),
                ),
                Utc,
            ),
            terminate_eof: cli.terminate_on_eof,
            terminate_err: cli.terminate_on_err,
        })
    }

    fn create_regex(patterns: &Option<Vec<String>>) -> regex::Regex {
        let mut result = String::new();
        match patterns {
            None => regex::Regex::new("").expect("Could not create default include/exclude regex."),
            Some(patterns) => {
                for pattern in patterns {
                    if !result.is_empty() {
                        result.push('|');
                    };
                    result.push('(');
                    result.push_str(pattern);
                    result.push(')');
                }
                regex::Regex::new(result.as_str())
                    .expect(format!("Could not create regex for {:?}", patterns).as_str())
            }
        }
    }

    fn process(&mut self) {
        eprintln!("{self}");
    }
}

impl fmt::Display for NMEAFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NMEAFile {{ stream: <not printable>, start_timestamp: {:?}, end_timestamp: {:?}, include_devices: {:?}, exclude_devices: {:?}, include_messages: {:?}, exclude_messages: {:?} most_recent_time: {:?}, terminate_eof: {:?}, terminate_err: {:?} }}",
	       self.start_timestamp, self.end_timestamp, self.include_devices, self.exclude_devices,
	       self.include_messages, self.exclude_messages, self.most_recent_time,
	       self.terminate_err, self.terminate_eof)
    }
}

fn main() {
    let cli = Cli::parse();
    eprintln!("{cli:?}");
    if let Some(mut n) = NMEAFile::new(&cli) {
        n.process();
    }
}
