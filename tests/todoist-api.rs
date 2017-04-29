extern crate nextaction;
use std::env;
use nextaction::Todoist;

fn init() -> Todoist {
    let token = env::var("TODOIST_TOKEN").unwrap();
    let client = Todoist::new(&token);
    client
}

#[test]
fn sync() {
    let result = init().sync();
    assert!(result.is_ok());
}

#[test]
fn sync_user() {
    let result = init().sync_fields(&["user"]);
    assert!(result.is_ok());
}

#[test]
fn sync_projects() {
    let result = init().sync_fields(&["projects"]);
    assert!(result.is_ok());
}

#[test]
fn add_label() {
    let mut client = init();
    let mut m = client.manager();
    m.add_label("helloword");
    m.add_label("kkk");
    m.flush().unwrap();
}