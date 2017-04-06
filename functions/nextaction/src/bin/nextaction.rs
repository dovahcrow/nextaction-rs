extern crate nextaction;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate config;
extern crate rust_apex;
extern crate serde_json;

use nextaction::{NextAction, Error};
use config::{Config, File, FileFormat, Environment};
use serde_json::Value;

fn main() {
    ::env_logger::init().unwrap();
    rust_apex::run::<_, _, Error, _>(|_: Value, _: rust_apex::Context| {
        let mut c = Config::new();
        c.merge(File::new("Config", FileFormat::Toml).required(false)).unwrap();
        c.merge(Environment::new("NXTT")).unwrap();

        let token = c.get_str("TOKEN").expect("You need to set the NXTT_TOKEN");

        let interval = c.get_int("INTERVAL").unwrap_or(10) as u64;

        let mut na = NextAction::new(&token);

        c.get("NEXTACTION_NAME").map(|n| na.nextaction_name = n.into_str().unwrap());
        c.get("SOMEDAY_NAME").map(|n| na.someday_name = n.into_str().unwrap());

        if let Err(err) = na.loopit(interval) {
            error!("Unexpected error: '{:?}', exiting...", err);
        };
        Ok(())
    });

}
