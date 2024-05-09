use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use clap::Parser;
use core::fmt::Debug;
use libnmea0183::base::Nmea0183Base;
use regex::Regex;
use std::io::{self, BufRead, BufReader};
use std::{fmt, fs};

// *****************************************************************************************
// Command Line parsing
// *****************************************************************************************

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

// *****************************************************************************************
// NMEAFile
// *****************************************************************************************

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

fn create_timestamp(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> Option<DateTime<Utc>> {
    let naivedate = NaiveDate::from_ymd_opt(year, month, day)?;
    let naivetime = NaiveTime::from_hms_opt(hour, minute, second)?;
    let naivedatetime = NaiveDateTime::new(naivedate, naivetime);
    Some(DateTime::from_naive_utc_and_offset(naivedatetime, Utc))
}

impl NMEAFile {
    fn new(cli: &Cli) -> Option<NMEAFile> {
        let reader: Box<dyn BufRead> = match cli.input_file_name.clone() {
            None => Box::new(BufReader::new(io::stdin())),
            Some(filename) => Box::new(BufReader::new(fs::File::open(filename).unwrap())),
        };

        let start_timestamp =
            create_timestamp(1900, 1, 1, 0, 0, 0).expect("Could not create starting timestamp");
        let end_timestamp =
            create_timestamp(2100, 12, 31, 23, 59, 59).expect("Could not create ending timestamp");

        let binding1 = Some(vec![String::from(".*")]);
        let binding2 = &(cli.include_devices.clone());
        let include_devices = NMEAFile::create_regex(if cli.include_devices.is_some() {
            &binding2
        } else {
            &binding1
        });

        let binding3 = cli.include_messages.clone();
        let include_messages = NMEAFile::create_regex(if cli.include_messages.is_some() {
            &binding3
        } else {
            &binding1
        });

        let binding4 = cli.exclude_devices.clone();
        let binding5 = Some(vec![String::from("^$")]);
        let exclude_devices = NMEAFile::create_regex(if cli.exclude_devices.is_some() {
            &binding4
        } else {
            &binding5
        });

        let binding6 = cli.exclude_messages.clone();
        let exclude_messages = NMEAFile::create_regex(if cli.exclude_messages.is_some() {
            &binding6
        } else {
            &binding5
        });

        Some(NMEAFile {
            stream: reader,
            start_timestamp,
            end_timestamp,
            include_devices,
            exclude_devices,
            include_messages,
            exclude_messages,
            most_recent_time: create_timestamp(1900, 1, 1, 0, 0, 0)
                .expect("Could not create starting current timestamp"),
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
        let mut buffer = String::new();
        loop {
            buffer.clear();
            match self.stream.read_line(&mut buffer) {
                Err(e) => {
                    if self.terminate_err {
                        eprintln!("{e:?}");
                        return;
                    }
                }

                Ok(n) => {
                    if n == 0 && self.terminate_eof {
                        return;
                    }
                    self.process_line(&buffer);
                }
            }
        }
    }

    fn process_line(&mut self, buffer: &String) {
        let mut buffer = buffer.clone();
        match buffer.pop() {
            Some('\n') => {}
            Some(c) => buffer.push(c),
            _ => {}
        }
        if buffer.len() > 0 {
            if let Ok(nmea) = Nmea0183Base::from_string(&buffer) {
                println!("{nmea:?}");
                let message = nmea.message.as_str();
                let sender = nmea.sender.as_str();

                if self.include_messages.is_match(message)
                    && !self.exclude_messages.is_match(message)
                    && self.include_devices.is_match(sender)
                    && !self.exclude_devices.is_match(sender)
                {
                    println!("{buffer}");
                }
            }
        }
    }
}

// *****************************************************************************************
// *****************************************************************************************

impl fmt::Display for NMEAFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NMEAFile {{ stream: <not printable>, start_timestamp: {:?}, end_timestamp: {:?}, include_devices: {:?}, exclude_devices: {:?}, include_messages: {:?}, exclude_messages: {:?} most_recent_time: {:?}, terminate_eof: {:?}, terminate_err: {:?} }}",
	       self.start_timestamp, self.end_timestamp, self.include_devices, self.exclude_devices,
	       self.include_messages, self.exclude_messages, self.most_recent_time,
	       self.terminate_err, self.terminate_eof)
    }
}

// *****************************************************************************************
// Main entrypoint
// *****************************************************************************************

fn main() {
    let cli = Cli::parse();
    eprintln!("{cli:?}");
    if let Some(mut n) = NMEAFile::new(&cli) {
        n.process();
    }
}
