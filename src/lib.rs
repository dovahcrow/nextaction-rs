#![recursion_limit = "1024"]
#![allow(dead_code)]

extern crate hyper;
extern crate hyper_rustls;
extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate url;
#[macro_use]
extern crate mime;
extern crate uuid;
#[macro_use]
extern crate error_chain;

pub mod errors;
mod protocol;
mod rebuild_insert;

use std::collections::BTreeSet;
use std::thread::sleep;
use std::time::Duration;

pub use protocol::{Project, Item, CommandManager};
pub use errors::*;
pub use protocol::{Todoist, TodoistResponse};
pub use rebuild_insert::RebuildInsertion;

pub const NEXTACTION: &'static str = "nextaction";
pub const SOMEDAY: &'static str = "someday";
pub const PARALLEL: char = '-';
pub const SEQUENTIAL: char = ':';

pub struct NextAction {
    todoist: Todoist,
    bag: BagOfThings,
    tree: TaskTree,
    nextaction_id: Option<usize>,
    someday_id: Option<usize>,
    pub nextaction_name: String,
    pub someday_name: String,
}

impl NextAction {
    pub fn new(token: &str) -> Self {
        NextAction {
            todoist: Todoist::new(token),
            tree: TaskTree::new(),
            bag: BagOfThings::default(),
            nextaction_id: None,
            nextaction_name: NEXTACTION.into(),
            someday_id: None,
            someday_name: SOMEDAY.into(),
        }
    }

    pub fn sync(&mut self) -> Result<()> {
        let result = self.todoist.sync()?;
        debug!("Sync result: '{:?}'", result);
        self.todoist.set_sync_token("*");
        self.bag = BagOfThings::default();
        self.bag.merge(&result);
        debug!("Current Bag is '{:?}'", &self.bag);

        // Find the nextaction lable id
        if let Some(lb) = result.labels
            .as_ref()
            .unwrap()
            .iter()
            .find(|l| l.name == self.nextaction_name) {
            self.nextaction_id = Some(lb.id);
        }
        // if not found, create a new lable with the name
        if self.nextaction_id.is_none() {
            let lb = self.todoist.add_label(&self.nextaction_name)?;
            self.nextaction_id = Some(lb.id);
        }
        // find the someday label id
        if let Some(lb) = result.labels
            .as_ref()
            .unwrap()
            .iter()
            .find(|l| l.name == self.someday_name) {
            self.someday_id = Some(lb.id);
        }
        // if not found, create a new label with the name
        if self.someday_id.is_none() {
            let lb = self.todoist.add_label(&self.someday_name)?;
            self.someday_id = Some(lb.id);
        }

        Ok(())
    }

    pub fn build_tree(&mut self) -> Result<()> {
        self.tree = TaskTree::new();

        for project in &self.bag.projects {
            push_level(&mut self.tree.nodes,
                       NodeType::ProjectNodeType(project.clone()),
                       project.indent);
        }

        for item in &self.bag.items {
            let project = self.tree
                .search_project(item.project_id)
                .ok_or("project_id not found in project".to_string())?;

            push_level(&mut project.nodes,
                       NodeType::ItemNodeType(item.clone()),
                       item.indent);
        }
        debug!("Tree is {:?}", self.tree);
        Ok(())
    }

    pub fn step(&mut self) -> Result<()> {
        info!("Step a round");
        self.sync()?;
        self.build_tree()?;
        let mut m = self.todoist.manager();
        for node in &self.tree.nodes {
            traversal(node,
                      &mut m,
                      TraversalState::Unconstraint,
                      self.nextaction_id.ok_or("nextaction_id is None".to_string())?,
                      self.someday_id.ok_or("nextaction_id is None".to_string())?)
        }
        m.flush()?;
        info!("Step finished");
        Ok(())
    }

    pub fn loopit(&mut self, sec: u64) -> Result<()> {
        loop {
            info!("Start a round of loop");
            self.sync()?;
            self.build_tree()?;
            let mut m = self.todoist.manager();
            for node in &self.tree.nodes {
                traversal(node,
                          &mut m,
                          TraversalState::Unconstraint,
                          self.nextaction_id.ok_or("nextaction_id is None".to_string())?,
                          self.someday_id.ok_or("nextaction_id is None".to_string())?)
            }
            m.flush()?;
            info!("Round finished, sleeping for {} sec", sec);
            sleep(Duration::new(sec, 0));
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum TraversalState {
    Suppressed,
    Unconstraint,
    Active,
}

fn traversal(node: &Node,
             manager: &mut CommandManager,
             state: TraversalState,
             naid: usize,
             sdid: usize) {
    use TraversalState::*;

    let name: String = node.name();

    let (is_parallel, is_sequential) = (name.ends_with(PARALLEL), name.ends_with(SEQUENTIAL));

    match node.ntype {
        NodeType::ItemNodeType(ref rnode) => {
            if rnode.checked == 1 {
                if rnode.labels.contains(&naid) || rnode.labels.contains(&sdid) {
                    let v: Vec<usize> = rnode.labels
                        .clone()
                        .into_iter()
                        .filter(|&u| u != naid && u != sdid)
                        .collect();
                    manager.set_item_label(rnode.id, v);
                }
            } else {
                if state == Active &&
                   (node.nodes.len() == 0 || node.nodes.iter().all(|l| l.checked()) ||
                    (!is_parallel && !is_sequential)) &&
                   !rnode.labels.contains(&sdid) {
                    if !rnode.labels.contains(&naid) {
                        let mut v = vec![naid];
                        v.extend_from_slice(&rnode.labels);
                        manager.set_item_label(rnode.id, v);
                    }
                } else {
                    if rnode.labels.contains(&naid) {
                        let v: Vec<usize> =
                            rnode.labels.clone().into_iter().filter(|&u| u != naid).collect();
                        manager.set_item_label(rnode.id, v);
                    }
                }
            }
        }
        NodeType::ProjectNodeType(_) => {}
    }


    let mut substate = match state {
        Unconstraint => Active,
        Suppressed => Suppressed,
        Active => Active,
    };

    if is_parallel {
        for node in &node.nodes {
            traversal(node, manager, substate, naid, sdid);
        }
    } else if is_sequential {
        for node in &node.nodes {
            traversal(node, manager, substate, naid, sdid);
            match node.ntype {
                NodeType::ItemNodeType(ref node) => {
                    if node.checked == 0 {
                        substate = Suppressed;
                    }
                }
                NodeType::ProjectNodeType(_) => {
                    substate = Suppressed;
                }
            }
        }
    } else {
        for node in &node.nodes {
            traversal(node, manager, Unconstraint, naid, sdid);
        }
    }
}

#[derive(Default, Debug)]
struct BagOfThings {
    projects: BTreeSet<Project>,
    items: BTreeSet<Item>,
}

impl BagOfThings {
    fn merge(&mut self, other: &TodoistResponse) {
        for project in other.projects
            .as_ref()
            .unwrap() {
            if project.is_archived == 1 {
                self.projects.remove(project);
            } else {
                self.projects.rebuild_insert(project.clone());
            }
        }

        for item in other.items
            .as_ref()
            .unwrap() {
            if item.is_deleted == 1 || item.is_archived == 1 {
                self.items.remove(item);
            } else {
                self.items.rebuild_insert(item.clone());
            }
        }
    }
}

fn push_level(to: &mut Vec<Node>, node: NodeType, level: usize) {
    if level == 1 {
        to.push(Node {
            ntype: node,
            nodes: vec![],
        })
    } else {
        let l = to.len() - 1;
        push_level(&mut to[l].nodes, node, level - 1)
    }
}

#[derive(Debug)]
pub enum NodeType {
    ProjectNodeType(Project),
    ItemNodeType(Item),
}

impl NodeType {
    fn id(&self) -> usize {
        match self {
            &NodeType::ProjectNodeType(ref project) => project.id,
            &NodeType::ItemNodeType(ref item) => item.id,
        }
    }

    fn name(&self) -> String {
        match self {
            &NodeType::ProjectNodeType(ref project) => project.name.clone(),
            &NodeType::ItemNodeType(ref item) => item.content.clone(),
        }
    }

    fn checked(&self) -> bool {
        match self {
            &NodeType::ProjectNodeType(_) => false,
            &NodeType::ItemNodeType(ref node) => node.checked == 1,
        }
    }

    fn is_project(&self) -> bool {
        match self {
            &NodeType::ProjectNodeType(_) => true,
            &NodeType::ItemNodeType(_) => false,
        }
    }

    fn is_item(&self) -> bool {
        !self.is_project()
    }
}

#[derive(Debug)]
pub struct Node {
    ntype: NodeType,
    nodes: Vec<Node>,
}

impl Node {
    fn id(&self) -> usize {
        self.ntype.id()
    }

    fn name(&self) -> String {
        self.ntype.name().clone()
    }

    fn checked(&self) -> bool {
        self.ntype.checked()
    }

    fn is_project(&self) -> bool {
        self.ntype.is_project()
    }

    fn is_item(&self) -> bool {
        self.ntype.is_item()
    }

    fn search<F>(&mut self, pred: &F) -> Option<&mut Self>
        where F: Fn(&Node) -> bool
    {
        if pred(self) {
            Some(self)
        } else {
            for node in &mut self.nodes {
                if let Some(found) = node.search(pred) {
                    return Some(found);
                }
            }
            None
        }
    }

    fn search_project(&mut self, id: usize) -> Option<&mut Self> {
        self.search(&|node: &Node| node.id() == id && node.is_project())
    }

    fn search_item(&mut self, id: usize) -> Option<&mut Self> {
        self.search(&|node: &Node| node.id() == id && node.is_item())
    }
}

#[derive(Debug)]
pub struct TaskTree {
    nodes: Vec<Node>,
}

impl TaskTree {
    fn new() -> Self {
        TaskTree { nodes: Vec::new() }
    }

    fn search_project(&mut self, id: usize) -> Option<&mut Node> {
        for node in &mut self.nodes {
            if let Some(node) = node.search_project(id) {
                return Some(node);
            }
        }
        None
    }
}