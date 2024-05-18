use chrono::{DateTime, Days, NaiveDateTime, NaiveTime, Utc};
use clap::Parser;
use core::fmt::Debug;
use libnmea0183::{base::Nmea0183Base, Nmea0183};
use regex::Regex;
use std::io::{self, BufRead, BufReader, Write};
use std::process::exit;
use std::{fmt, fs};
use std::fs::OpenOptions;
use std::path::Path;

// *****************************************************************************************
// Command Line parsing
// *****************************************************************************************

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    input_file_name: Option<String>,

    #[arg(
        long = "count",
        short = 'c',
        help = "Display running count of lines processed"
    )]
    display_count: bool,

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
        long = "end",
        short,
        help = "latest date/time to log in Utc formatted as yyyy-mm-ddThh:mm:ssZ"
    )]
    end_date: Option<String>,

    #[arg(
    long = "init",
    help = "Initialization data to send to NMEA"
    )]
    initialization : Option<Vec<String>>,

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
        long = "start",
        short,
        help = "earliest date/time to log in Utc formatted as yyyy-mm-ddThh:mm:ssZ"
    )]
    start_date: Option<String>,

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
    display_count: bool,
    include_devices: Regex,
    exclude_devices: Regex,
    include_messages: Regex,
    exclude_messages: Regex,
    most_recent_timestamp: DateTime<Utc>,
    terminate_eof: bool,
    terminate_err: bool,
}

impl NMEAFile {
    fn new(cli: &Cli) -> Option<NMEAFile> {
        if cli.initialization.is_some() {
            match cli.input_file_name.clone() {
                None => {
                    let mut d = io::stdout();
                    for l in cli.initialization.clone().unwrap() {
                        d.write(l.as_bytes()).expect("Could not initialize stdout");
                        d.write(&['\n' as u8]).expect("Could not terminate initialization on stdout");
                    }
                },
                Some(filename) => {
                    if let Ok(mut data_file) = OpenOptions::new().append(true).open(filename) {
                        for l in cli.initialization.clone().unwrap() {
                            data_file
                                .write(l.as_bytes()).expect("Could not initialize device");
                            data_file.write(&['\n' as u8]).expect("Could not terminate initialization on device");
                        }
                    };
                }
            }
        }

        let reader: Box<dyn BufRead>;
        match cli.input_file_name.clone() {
            None => reader = Box::new(BufReader::new(io::stdin())),
            Some(filename) => {
                let p = Path::new(&filename);
                let f = fs::File::open(&p);
                if f.is_err() {
                    eprintln!("{:?}", f);
                    return None;
                }
                reader = Box::new(BufReader::new(f.unwrap()))
            }
        };

        let binding_default = Some(vec![String::from(".*")]);
        let include_devices = {
            let binding_local = &(cli.include_devices.clone());
            NMEAFile::create_regex(if cli.include_devices.is_some() {
                &binding_local
            } else {
                &binding_default
            })
        };

        let include_messages = {
            let binding_local = cli.include_messages.clone();
            NMEAFile::create_regex(if cli.include_messages.is_some() {
                &binding_local
            } else {
                &binding_default
            })
        };

        let binding_default = Some(vec![String::from("^$")]);
        let exclude_devices = {
            let binding_local = cli.exclude_devices.clone();
            NMEAFile::create_regex(if cli.exclude_devices.is_some() {
                &binding_local
            } else {
                &binding_default
            })
        };

        let exclude_messages = {
            let binding_local = cli.exclude_messages.clone();
            NMEAFile::create_regex(if cli.exclude_messages.is_some() {
                &binding_local
            } else {
                &binding_default
            })
        };

        let most_recent_timestamp = {
            let current_timestamp = Utc::now();
            let current_date = current_timestamp.date_naive();
            let current_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            let naive_timestamp = NaiveDateTime::new(current_date, current_time);
            naive_timestamp.and_utc()
        };

        let start_timestamp = if cli.start_date.is_none() {
            DateTime::parse_from_rfc3339("2000-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
        } else {
            match DateTime::parse_from_rfc3339(cli.start_date.clone().unwrap().as_str()) {
                Ok(d) => d.with_timezone(&Utc),
                Err(e) => {
                    eprintln!("{e:?}");
                    exit(-1);
                }
            }
        };

        let end_timestamp = if cli.end_date.is_none() {
            DateTime::parse_from_rfc3339("2050-12-31T23:59:59Z")
                .unwrap()
                .with_timezone(&Utc)
        } else {
            match DateTime::parse_from_rfc3339(cli.end_date.clone().unwrap().as_str()) {
                Ok(d) => d.with_timezone(&Utc),
                Err(e) => {
                    eprintln!("{e:?}");
                    exit(-1);
                }
            }
        };

        println!("Start time {:?} End time {:?}", start_timestamp, end_timestamp);
        if start_timestamp > end_timestamp {
            eprintln!("Start time is after or the same as end time.");
            exit(-1);
        }

        let display_count = cli.display_count;

        Some(NMEAFile {
            stream: reader,
            start_timestamp,
            end_timestamp,
            display_count,
            include_devices,
            exclude_devices,
            include_messages,
            exclude_messages,
            most_recent_timestamp,
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
        let mut buffer = String::new();
        let mut linecount: u128 = 0;
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
                    if self.display_count {
                        linecount += 1;
                        eprint!("{linecount} \r");
                    }
                    self.process_line(&buffer);
                }
            }
        }
    }

    fn update_time_only(&mut self, timestamp: NaiveTime) {
        let mut current_date = self.most_recent_timestamp.date_naive();
        let current_time = self.most_recent_timestamp.time();
        if current_time > timestamp {
            // increment current_date by one day
            current_date = current_date.checked_add_days(Days::new(1)).unwrap();
        }
        let naive_date_stamp = NaiveDateTime::new(current_date, timestamp);
        self.most_recent_timestamp = DateTime::from_naive_utc_and_offset(naive_date_stamp, Utc);
    }

    fn process_line(&mut self, buffer: &String) {
        let mut buffer = buffer.clone();
        match buffer.pop() {
            Some('\n') => {}
            Some(c) => buffer.push(c),
            _ => {}
        }
        if buffer.len() > 0 {
            if let Ok(nmea_base) = Nmea0183Base::from_string(&buffer) {
                let message = nmea_base.message.as_str();
                let sender = nmea_base.sender.as_str();

                if self.include_messages.is_match(message)
                    && !self.exclude_messages.is_match(message)
                    && self.include_devices.is_match(sender)
                    && !self.exclude_devices.is_match(sender)
                {
                    match libnmea0183::classify(nmea_base) {
                        Nmea0183::BWC(sentence) => self.update_time_only(
                            sentence
                                .timestamp()
                                .expect("Internal error in BWC sentence"),
                        ),
                        Nmea0183::BWR(sentence) => self.update_time_only(
                            sentence
                                .timestamp()
                                .expect("Internal error in BWR sentence"),
                        ),
                        Nmea0183::GBS(sentence) => self.update_time_only(
                            sentence
                                .timestamp()
                                .expect("Internal error in GBS sentence"),
                        ),
                        Nmea0183::GGA(sentence) => self.update_time_only(
                            sentence
                                .timestamp()
                                .expect("Internal error in GGA sentence"),
                        ),
                        Nmea0183::GLL(sentence) => self.update_time_only(
                            sentence
                                .timestamp()
                                .expect("Internal error in GLL sentence"),
                        ),
                        Nmea0183::GRS(sentence) => self.update_time_only(
                            sentence.timestamp().expect("Internal error in GRS message"),
                        ),
                        Nmea0183::GST(sentence) => self.update_time_only(
                            sentence.timestamp().expect("Internal error in GST message"),
                        ),
                        Nmea0183::GXA(sentence) => self.update_time_only(
                            sentence.timestamp().expect("Internal error in GXA message"),
                        ),
                        Nmea0183::RMC(sentence) => {
                            self.most_recent_timestamp = sentence
                                .timestamp()
                                .expect("Internal error in RMC sentence")
                        }
                        Nmea0183::TRF(sentence) => self.update_time_only(
                            sentence
                                .timestamp()
                                .expect("Internal error in TRF sentence"),
                        ),
                        Nmea0183::ZDA(sentence) => {
                            self.most_recent_timestamp = sentence
                                .timestamp()
                                .expect("Internal error in ZDA sentence")
                        }
                        _ => {}
                    }
                    if self.most_recent_timestamp >= self.start_timestamp
                        && self.most_recent_timestamp <= self.end_timestamp
                    {
                        println!("{buffer}");
                    }
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
	       self.include_messages, self.exclude_messages, self.most_recent_timestamp,
	       self.terminate_err, self.terminate_eof)
    }
}

// *****************************************************************************************
// Main entrypoint
// *****************************************************************************************

fn main() {
    let cli = Cli::parse();
    match NMEAFile::new(&cli) {
        None => eprintln!("Could not create NMEAFile tracking system."),
        Some(mut n) => n.process(),
    }
}
