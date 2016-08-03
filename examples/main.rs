#![feature(plugin)]
#![plugin(docopt_macros)]
extern crate docopt;
extern crate nextaction;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;

use nextaction::{NextAction, ErrorKind};
use std::env;
use std::process;
use std::thread;
use std::time::Duration;


docopt!(Args,
        "
Usage: nextaction for todoist
    nextaction [--token <token>] [--interval <interval>]
");


fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    ::env_logger::init().unwrap();
    {
        let token = if args.arg_token == "" {
            env::var("TODOIST_TOKEN").unwrap()
        } else {
            args.arg_token
        };

        let interval: u64 = args.arg_interval.parse().unwrap_or(10);

        let mut na = NextAction::new(&token);
        let mut result = na.loopit(interval).unwrap_err();
        loop {
            if let ErrorKind::HyperError = result.0 {
                warn!("Network issue '{:?}', continuing the loop", result.1);
                result = na.loopit(interval).unwrap_err();
            } else {
                error!("Unexpected error: '{:?}', exiting...", result);
                thread::sleep(Duration::new(1, 0));
                break;
            }
        }
    }
    process::exit(-1);
}
