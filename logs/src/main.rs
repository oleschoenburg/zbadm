use chrono_humanize::Humanize;
use regex::Regex;
use serde::Serialize;

use std::{
    collections::BTreeSet,
    io::{stdin, BufRead, IsTerminal},
    sync::OnceLock,
};

static LOG_HEADER: OnceLock<Regex> = OnceLock::new();
static LOG_CONTENT: OnceLock<Regex> = OnceLock::new();

fn main() {
    struct Input {
        name: String,
        lines: Box<dyn Iterator<Item = std::io::Result<String>>>,
    }
    let inputs = if stdin().is_terminal() {
        let dir = std::env::args()
            .nth(1)
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap();
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                let name = path.file_name().unwrap().to_str().unwrap().to_owned();
                let file = std::fs::File::open(path).unwrap();
                let lines = std::io::BufReader::new(file).lines();
                Input {
                    name: name,
                    lines: Box::new(lines),
                }
            })
            .collect::<Vec<_>>()
    } else {
        vec![Input {
            name: "stdin".to_owned(),
            lines: Box::new(std::io::stdin().lock().lines()),
        }]
    };

    let mut log = BTreeSet::new();
    for mut input in inputs {
        let mut line_number: u32 = 0;
        while let Some(Ok(line)) = input.lines.next() {
            let line = Line {
                number: line_number,
                content: &line,
                file: &input.name,
            };
            line_number += 1;
            if let Some(header) = log_header(line) {
                if let Some(Ok(line)) = input.lines.next() {
                    let line = Line {
                        number: line_number,
                        content: &line,
                        file: &input.name,
                    };
                    line_number += 1;

                    if let Some(content) = log_message(header, line) {
                        log.insert(content);
                    }
                }
            }
        }
    }

    log.iter()
        .min_by_key(|LogMessage { timestamp, .. }| timestamp);
    let from = log.first().map(|m| m.timestamp).unwrap();
    let to = log.last().map(|m| m.timestamp).unwrap();

    let span = to - from;
    let count = log.len();
    let rate = count as f64 / span.num_seconds() as f64;
    eprintln!(
        "{} entries {} from {} to {}, averaging {rate:.2} entries per second",
        count,
        span.humanize(),
        from,
        to
    );
    let brokers = log
        .iter()
        .map(|m| m.actor.broker)
        .flatten()
        .collect::<BTreeSet<u32>>();
    eprintln!("Brokers: {:?}", brokers);

    let partitions = log
        .iter()
        .map(|m| m.actor.partition)
        .flatten()
        .collect::<BTreeSet<u32>>();
    eprintln!("Partitions: {:?}", partitions);

    for entry in log {
        println!("{}", serde_json::to_string(&entry).unwrap())
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
struct LogHeader {
    start: u32,
    timestamp: String,
    actor: Actor,
    thread: String,
    level: String,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
struct Actor {
    broker: Option<u32>,
    name: String,
    partition: Option<u32>,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Serialize)]
struct LogMessage {
    timestamp: chrono::NaiveDateTime,
    lines: (u32, u32),
    file: String,
    actor: Actor,
    thread: String,
    level: String,
    logger: String,
    message: String,
}

struct Line<'a> {
    number: u32,
    content: &'a str,
    file: &'a str,
}

fn log_header(line: Line) -> Option<LogHeader> {
    // Matches "2023-06-01 17:28:06.904 [Broker-5-Startup] [Broker-5-zb-actors-3] INFO"
    let regex = LOG_HEADER.get_or_init(|| {
        Regex::new(
            r"(?x)
            (?P<timestamp>\d{4}-\d{2}-\d{2}\s\d{2}:\d{2}:\d{2}.\d{3})
            \s+
            \[(?P<actor>.+?)\]
            \s+
            \[(?P<thread>.+?)\]
            \s+
            (?P<level>[^\s]+)",
        )
        .unwrap()
    });

    if let Some(captures) = regex.captures(line.content) {
        let actor = captures["actor"].to_owned();
        let actor = match actor.split('-').collect::<Vec<_>>()[..] {
            ["Broker", broker, name] => Actor {
                broker: broker.parse().ok(),
                name: name.to_owned(),
                partition: None,
            },
            ["Broker", broker, name, partition] => Actor {
                broker: broker.parse().ok(),
                name: name.to_owned(),
                partition: partition.parse().ok(),
            },
            _ => Actor {
                broker: None,
                name: actor,
                partition: None,
            },
        };

        let header = LogHeader {
            start: line.number,
            timestamp: captures["timestamp"].to_owned(),
            actor: actor,
            thread: captures["thread"].to_owned(),
            level: captures["level"].to_owned(),
        };
        return Some(header);
    } else {
        return None;
    }
}

fn log_message(header: LogHeader, line: Line) -> Option<LogMessage> {
    let regex = LOG_CONTENT.get_or_init(|| {
        Regex::new(
            r"(?x)
            \s+
            (?P<logger>[^\s]+)
            \s+
            -
            \s+
            (?P<message>.+)
        ",
        )
        .unwrap()
    });

    if let Some(captures) = regex.captures(line.content) {
        let message = LogMessage {
            lines: (header.start, line.number),
            file: line.file.to_owned(),
            timestamp: chrono::NaiveDateTime::parse_from_str(
                &header.timestamp,
                "%Y-%m-%d %H:%M:%S%.3f",
            )
            .unwrap(),
            actor: header.actor,
            thread: header.thread,
            level: header.level,
            logger: captures["logger"].to_owned(),
            message: captures["message"].to_owned(),
        };
        return Some(message);
    } else {
        return None;
    }
}
