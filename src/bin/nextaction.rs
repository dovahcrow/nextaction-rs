extern crate nextaction;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate config;
#[macro_use]
extern crate error_chain;

use nextaction::{NextAction, ErrorKind, Result, Error};
use std::thread;
use std::time::Duration;

use config::{Config, File, FileFormat, Environment};

quick_main!(run);

fn run() -> Result<()> {
    let mut c = Config::new();

    c.merge(File::new("Config", FileFormat::Toml).required(false)).unwrap();

    c.merge(Environment::new("NXTT")).unwrap();

    ::env_logger::init().unwrap();

    let token = c.get_str("TOKEN").expect("You need to set the NXTT_TOKEN");

    let interval = c.get_int("INTERVAL").unwrap_or(10) as u64;

    let mut na = NextAction::new(&token);

    c.get("NEXTACTION_NAME").map(|n| na.nextaction_name = n.into_str().unwrap());
    c.get("SOMEDAY_NAME").map(|n| na.someday_name = n.into_str().unwrap());

    let mut result = na.loopit(interval);
    loop {
        match result {
            Err(Error(ErrorKind::HyperError(err), _)) => {
                warn!("Network issue '{:?}', continuing the loop", err)
            }
            Err(err) => {
                error!("Unexpected error: '{:?}', exiting...", err);
                thread::sleep(Duration::new(1, 0));
            }
            Ok(_) => {}
        }
        result = na.loopit(interval);
    }
}
