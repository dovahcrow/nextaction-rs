extern crate nextaction;
use nextaction::NextAction;
use std::env;

#[test]
fn build_tree() {
    let mut na = NextAction::new(&env::var("TODOIST_TOKEN").unwrap());
    let _ = na.sync();
    na.build_tree().unwrap();
}

#[test]
fn incremental_sync() {
    use std::thread::sleep;
    use std::time::Duration;

    let mut na = NextAction::new(&env::var("TODOIST_TOKEN").unwrap());
    let _ = na.sync();
    na.build_tree().unwrap();

    sleep(Duration::new(20, 0));

    let _ = na.sync();
    na.build_tree().unwrap()
}