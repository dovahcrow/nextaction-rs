#![feature(question_mark, custom_derive, plugin)]
#![plugin(serde_macros)]

#[macro_use]
extern crate hyper;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate wrapped_enum;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate url;
#[macro_use]
extern crate mime;
#[macro_use]
extern crate json;
extern crate uuid;

use protocol::*;
use error::*;

mod error;
mod protocol;


pub struct NextAction {
    todoist: Todoist,
    raw_data: Option<TodoistResponse>,
    tree: TaskTree,
    nextaction_id: Option<usize>,
    pub nextaction_name: String,
}

impl NextAction {
    pub fn new(token: &str) -> Self {
        NextAction {
            todoist: Todoist::new(token),
            tree: TaskTree::new(),
            raw_data: None,
            nextaction_id: None,
            nextaction_name: "nextaction".into(),
        }
    }

    pub fn sync(&mut self) -> Result<()> {
        self.tree = TaskTree::new();

        let result = self.todoist.sync()?;

        self.raw_data = Some(if let Some(tr) = self.raw_data.take() {
            tr.merge(result)
        } else {
            result
        });

        if let Some(lb) = self.raw_data
            .as_ref()
            .unwrap()
            .labels
            .iter()
            .find(|l| l.name == self.nextaction_name) {
            self.nextaction_id = Some(lb.id);
        } else {
            let lb = self.todoist.add_label(&self.nextaction_name)?;
            self.nextaction_id = Some(lb.id);
        }

        let tmp = self.raw_data.as_mut().unwrap();

        tmp.projects.sort();
        tmp.items.sort();

        Ok(())
    }

    pub fn build_tree(&mut self) -> Result<()> {

        self.tree = TaskTree::new();

        let dt: &TodoistResponse = self.raw_data.as_ref().unwrap();

        for project in &dt.projects {
            push_level(&mut self.tree.nodes,
                       NodeType::ProjectNodeType(project.clone()),
                       project.indent);
        }

        for item in &dt.items {
            let project = self.tree
                .search_project(item.project_id)
                .ok_or("project_id not found in project".to_string())?;

            push_level(&mut project.nodes,
                       NodeType::ItemNodeType(item.clone()),
                       item.indent);
        }
        Ok(())
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

#[cfg(test)]
mod test {
    use ::NextAction;
    use ::std::env;
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
}