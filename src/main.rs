extern crate serde;
extern crate serde_json;
extern crate chrono;
extern crate reqwest;
extern crate slack_hook;
extern crate getopts;

#[macro_use]
extern crate serde_derive;

mod config;
mod bitbucket;
mod http;

use std::collections::HashMap;
use std::{thread, time};
use std::env;
use chrono::{Local, NaiveDateTime, TimeZone, Duration, DateTime};
use slack_hook::{Slack, PayloadBuilder};
use getopts::Options;

fn from_unix_timestamp(ts: i64) -> DateTime<Local> {
    let secs = ts / 1000;
    let nsecs = ts % 1000;
    let ndt = NaiveDateTime::from_timestamp(secs, nsecs as u32);

    Local.from_utc_datetime(&ndt)
}

#[derive(Debug)]
struct NotificationHistoryEntry {
    updated_at: DateTime<Local>,
    notified_ctn: u32,
}

impl NotificationHistoryEntry {
    fn new(now: DateTime<Local>) -> NotificationHistoryEntry {
        NotificationHistoryEntry {
            updated_at: now,
            notified_ctn: 0,
        }
    }
}

enum Notification {
    TooOld(bitbucket::PullRequest, DateTime<Local>),
    NeedReview(bitbucket::PullRequest, DateTime<Local>, u8),
    Reviewed(bitbucket::PullRequest, DateTime<Local>, u8),
}

//enum Notificator<'a> {
//    Slack(SlackNotifier<'a>),
//    Console,
//}

fn notify_console(notification: Notification) {
    match notification {
        Notification::TooOld(pull_request, created) => {
            println!("pull request {} is too old (created: {})", pull_request.title,
                     created.format("%Y-%m-%d %H:%M:%S"));
        }

        Notification::NeedReview(pull_request, created, reviewers_approved) => {
            println!("pull request {} needs to be reviewed (approved: {} created: {})",
                     pull_request.title,
                     reviewers_approved,
                     created.format("%Y-%m-%d %H:%M:%S"));
        }

        Notification::Reviewed(pull_request, created, reviewers_approved) => {
            println!("pull request {} has been reviewed (approved: {} created: {})",
                     pull_request.title,
                     reviewers_approved,
                     created.format("%Y-%m-%d %H:%M:%S"));
        }
    }
}

trait Notifiable {
    fn notify(&self, notification: Notification) {
        notify_console(notification)
    }
}

//impl<'a> Notificator<'a> {
//    fn notify(&self, notification: Notification) {
//        match self {
//            &Notificator::Slack(ref slack) => slack.notify(notification),
//            &Notificator::Console => notify_console(notification),
//        }
//    }
//}

struct SlackNotifier<'a> {
    client: Slack,
    channel: &'a str,
    username: &'a str,
}

impl<'a> SlackNotifier<'a> {
    fn new(uri: &str, channel: &'a str, username: &'a str) -> SlackNotifier<'a> {
        SlackNotifier {
            client: Slack::new(uri).expect("couldn't connect to slack"),
            channel,
            username,
        }
    }

    fn send(&self, msg: &str) {
        let payload = match PayloadBuilder::new()
            .text(msg)
            .channel(self.channel)
            .username(self.username)
            .build() {
            Ok(payload) => payload,
            Err(err) => {
                println!("ERR: {:?}", err);
                return;
            },
        };

        if let Err(err) = self.client.send(&payload) {
            println!("ERR: {:?}", err);
        }
    }
}

impl<'a> Notifiable for SlackNotifier<'a> {
    fn notify(&self, notification: Notification) {
        match notification {
            Notification::TooOld(pull_request, created) => {
                self.send(&format!("pull request {} is too old (created: {})", pull_request.title,
                         created.format("%Y-%m-%d %H:%M:%S")));
            }

            Notification::NeedReview(pull_request, created, reviewers_approved) => {
                self.send(&format!("pull request {} needs to be reviewed (approved: {} created: {})",
                         pull_request.title,
                         reviewers_approved,
                         created.format("%Y-%m-%d %H:%M:%S")));
            }

            Notification::Reviewed(pull_request, created, reviewers_approved) => {
                self.send(&format!("pull request {} has been reviewed (approved: {} created: {})",
                         pull_request.title,
                         reviewers_approved,
                         created.format("%Y-%m-%d %H:%M:%S")));
            }
        }
    }
}

struct ConsoleNotifier;

impl Notifiable for ConsoleNotifier {}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

struct Bot<'a> {
    cli: &'a http::Client,
    bitbucket_uri: &'a str,
    min_reviewers_approved: u8,
    max_age: Duration,
    notificator: &'a Notifiable,
    notification_history: &'a mut HashMap<u32, NotificationHistoryEntry>,
    notification_timeout: Duration
}

impl<'a> Bot<'a> {
    fn run(&mut self) -> Result<(), reqwest::Error> {
        let pull_requests: bitbucket::Response = self.cli.get(self.bitbucket_uri)?;

        let now = Local::now();

        for pull_request in pull_requests.values {
            if !pull_request.open {
                continue;
            }

            let entry = self.notification_history.entry(pull_request.id)
                .or_insert(NotificationHistoryEntry::new(now));

            if now.signed_duration_since(entry.updated_at) < self.notification_timeout && entry.notified_ctn > 0 {
                continue;
            }

            entry.notified_ctn += 1;
            entry.updated_at = now;

            let created = from_unix_timestamp(pull_request.created_date);

            if now.signed_duration_since(created) > self.max_age {
                self.notificator.notify(Notification::TooOld(pull_request, created));
                continue;
            }

            let reviewers_approved = pull_request.reviewers.iter()
                .filter(|reviewer| reviewer.approved)
                .count() as u8;

            let notification = if reviewers_approved < self.min_reviewers_approved {
                Notification::NeedReview(pull_request, created, reviewers_approved)
            } else {
                Notification::Reviewed(pull_request, created, reviewers_approved)
            };

            self.notificator.notify(notification);
        }

        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("c", "config", "config file name", "CONFIG");
    opts.optflag("d", "debug", "print messages to console instead of slack");
    opts.optflag("h", "help", "print this help menu");
    let matches = opts.parse(&args[1..]).expect("couldn't parse command line");
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let config_filename = matches.opt_str("c").unwrap_or(String::from("config.toml"));

    let cfg: config::Config = config::parse(&config_filename).expect(&format!("can't parse config '{}'", config_filename));

    println!("config: {:?}", cfg);

    let cli = http::Client::new(&cfg.bitbucket.username, &cfg.bitbucket.password);

//    let notifier = if matches.opt_present("d") {
//        Notificator::Console
//    } else {
//        Notificator::Slack(SlackNotifier::new(&cfg.slack.uri, &cfg.slack.channel, &cfg.slack.username))
//    };

    let notifier: Box<Notifiable> = if matches.opt_present("d") {
        Box::new(ConsoleNotifier)
    } else {
        Box::new(SlackNotifier::new(&cfg.slack.uri, &cfg.slack.channel, &cfg.slack.username))
    };

    let mut notification_history: HashMap<u32, NotificationHistoryEntry> = HashMap::new();

    let sleep_interval = time::Duration::from_secs(cfg.sleep_interval as u64);

    let mut bot = Bot {
        cli: &cli,
        bitbucket_uri: &cfg.bitbucket.uri,
        min_reviewers_approved: cfg.min_reviewers_approved,
        max_age: Duration::days(cfg.pr_max_age as i64),
        notificator: notifier.as_ref(),
        notification_history: &mut notification_history,
        notification_timeout: Duration::seconds(cfg.notification_timeout as i64),
    };

    loop {
        bot.run().expect("can't get PRs");

        thread::sleep(sleep_interval);
    }
}
