#![feature(loop_break_value)]

extern crate nextaction;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate config;
#[macro_use]
extern crate error_chain;

use nextaction::{NextAction, Result};
use std::thread;
use std::time::Duration;

use config::{Config, File, FileFormat, Environment};

quick_main!(run);

fn run() -> Result<()> {
    let mut c = Config::new();

    c.merge(File::new("Config", FileFormat::Toml).required(false)).unwrap();

    c.merge(Environment::new("NXTT")).unwrap();

    ::env_logger::init().unwrap();

    {
        let token = c.get_str("TOKEN").expect("You need to set the NXTT_TOKEN");

        let interval = c.get_int("INTERVAL").unwrap_or(10) as u64;

        let mut na = NextAction::new(&token);

        c.get("NEXTACTION_NAME").map(|n| na.nextaction_name = n.into_str().unwrap());
        c.get("SOMEDAY_NAME").map(|n| na.someday_name = n.into_str().unwrap());

        let mut result = na.loopit(interval);
        loop {
            if let Err(err) = result {
                warn!("Network issue '{:?}', continuing the loop", err);
                result = na.loopit(interval);
            } else {
                error!("Unexpected error: '{:?}', exiting...", result);
                thread::sleep(Duration::new(1, 0));
                break result;
            };
        }?;
        unreachable!("unreachable!")
    }
}
