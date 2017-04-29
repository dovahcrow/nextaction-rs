extern crate nextaction;
use std::iter::FromIterator;
use std::collections::BTreeSet;

use nextaction::{Project, RebuildInsertion};

#[test]
fn order1() {
    let a = Project {
        id: 1,
        item_order: 1,
        ..Default::default()
    };
    let b = Project {
        id: 2,
        item_order: 2,
        ..Default::default()
    };
    let c = Project {
        id: 3,
        item_order: 3,
        ..Default::default()
    };

    let mut bt = BTreeSet::new();
    bt.rebuild_insert(a.clone());
    bt.rebuild_insert(b.clone());
    bt.rebuild_insert(c.clone());

    let v: Vec<Project> = bt.into_iter().collect();
    assert_eq!(v, vec![a, b, c]);
}

#[test]
fn order2() {
    let a = Project {
        id: 1,
        item_order: 3,
        ..Default::default()
    };
    let b = Project {
        id: 2,
        item_order: 2,
        ..Default::default()
    };
    let c = Project {
        id: 3,
        item_order: 1,
        ..Default::default()
    };

    let mut bt = BTreeSet::new();
    bt.rebuild_insert(a.clone());
    bt.rebuild_insert(b.clone());
    bt.rebuild_insert(c.clone());

    let v: Vec<Project> = bt.into_iter().collect();
    assert_eq!(v, vec![c, b, a]);
}

#[test]
fn order3() {
    let a = Project {
        id: 1,
        item_order: 1,
        name: "a".to_string(),
        ..Default::default()
    };
    let b = Project {
        id: 2,
        item_order: 1,
        name: "b".to_string(),
        ..Default::default()
    };
    let c = Project {
        id: 3,
        item_order: 3,
        name: "c".to_string(),
        ..Default::default()
    };
    let d = Project {
        id: 1,
        item_order: 2,
        name: "d".to_string(),
        ..Default::default()
    };

    let mut bt = BTreeSet::new();
    bt.rebuild_insert(a.clone());
    bt.rebuild_insert(b.clone());
    bt.rebuild_insert(c.clone());
    bt.rebuild_insert(d.clone());

    let v: Vec<Project> = BTreeSet::from_iter(bt.into_iter()).into_iter().collect();
    assert_eq!(v, vec![b, d, c]);
}