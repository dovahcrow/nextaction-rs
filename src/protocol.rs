use hyper::Client;
use hyper::client::Response;
use hyper::header::{ContentType, UserAgent, Accept, qitem};
use hyper::status::StatusCode;

use serde_json::{from_reader, from_str};

use errors::*;

use std::collections::BTreeMap;
use std::cmp::PartialOrd;
use std::cmp::Ordering;

use uuid::Uuid;

use url::form_urlencoded;

use std::time::Duration;

#[allow(dead_code)]
pub const VERSION: &'static str = "v7";
const ENDPOINT: &'static str = "https://todoist.com/API/v7/sync";

pub struct Todoist {
    token: String,
    sync_token: String,
    client: Client,
}

impl Todoist {
    pub fn new(token: &str) -> Todoist {
        let mut client = Client::new();
        client.set_read_timeout(Some(Duration::from_secs(30)));
        client.set_write_timeout(Some(Duration::from_secs(30)));
        Todoist {
            token: token.into(),
            sync_token: "*".into(),
            client: client,
        }
    }

    pub fn set_sync_token(&mut self, token: &str) {
        self.sync_token = token.into();
    }

    fn post<'a, I>(&self, data: I) -> Result<Response>
        where I: IntoIterator<Item = (&'a str, String)>
    {
        let mut initial = vec![("token", self.token.clone())];

        initial.extend(data);
        let dt = form_urlencoded::Serializer::new(String::new()).extend_pairs(initial).finish();

        let resp: Response = self.client
            .post(ENDPOINT)
            .header(ContentType(mime!(Application/WwwFormUrlEncoded; Charset=Utf8)))
            .header(UserAgent("curl/7.43.0".into()))
            .header(Accept(vec![qitem(mime!(_/_))]))
            .body(&dt)
            .send()?;
        Ok(resp)
    }

    pub fn sync(&mut self) -> Result<TodoistResponse> {
        let resp = self.post(vec![
            ("sync_token", self.sync_token.clone()),
            ("resource_types", "[\"all\"]".into())
            ])?;

        if resp.status != StatusCode::Ok {
            Err(format!("status code is: {}", resp.status).into())
        } else {
            let result: TodoistResponse = from_reader(resp)?;
            self.sync_token = result.sync_token.clone();
            Ok(result)
        }
    }

    pub fn manager(&mut self) -> CommandManager {
        CommandManager::new(self)
    }

    pub fn add_label(&mut self, name: &str) -> Result<Label> {
        let mut m = self.manager();
        let (temp_id, uuid) = m.add_label(name);
        let result = m.flush()?;
        if result.sync_status[&uuid] != "ok" {
            Err(format!("Add label '{}' fail", name).into())
        } else {
            let id = result.temp_id_mapping[&temp_id];

            Ok(Label {
                id: id,
                name: name.into(),
            })
        }
    }
}

pub struct CommandManager<'a> {
    todoist: &'a mut Todoist,
    commands: Vec<::json::JsonValue>,
}

impl<'a> CommandManager<'a> {
    pub fn new(td: &'a mut Todoist) -> CommandManager {
        CommandManager {
            todoist: td,
            commands: vec![],
        }
    }

    pub fn add_label(&mut self, name: &str) -> (Uuid, Uuid) {
        let temp_id = Uuid::new_v4();
        let uuid = Uuid::new_v4();
        self.commands.push(object! {
            "type" => "label_add",
            "args" => object! {
                "name" => name
            },
            "temp_id" => format!("{}", temp_id),
            "uuid" => format!("{}", uuid)
        });
        (temp_id, uuid)
    }

    pub fn set_item_label(&mut self, id: usize, label_ids: Vec<usize>) -> Uuid {
        let uuid = Uuid::new_v4();
        self.commands.push(object! {
            "type" => "item_update",
            "uuid" => format!("{}", uuid),
            "args" => object! {
                "id" => id,
                "labels" => label_ids
            }
        });
        uuid
    }

    pub fn complete_item(&mut self, id: usize) -> Uuid {
        let uuid = Uuid::new_v4();
        self.commands.push(object! {
            "type" => "close_item",
            "uuid" => format!("{}", uuid),
            "args" => object! {
                "id" => id
            }
        });
        uuid
    }

    pub fn archive_project(&mut self, id: usize) -> Uuid {
        let uuid = Uuid::new_v4();
        self.commands.push(object! {
            "type" => "project_archive",
            "uuid" => format!("{}", uuid),
            "args" => object! {
                "ids" => array! [id]
            }
        });
        uuid
    }

    pub fn flush(self) -> Result<CommandResponse> {
        use std::io::Read;

        let count = self.commands.len();
        info!("{} items flushed", count);
        if count == 0 {
            return Ok(CommandResponse::default());
        }
        let mut resp = self.todoist
            .post(vec![       
            ("commands", ::json::stringify(
                self.commands
        ))])?;
        let mut s = String::new();
        let _ = resp.read_to_string(&mut s);
        debug!("flush response is '{}'", s);
        let result = from_str(&s)?;
        Ok(result)
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct CommandResponse {
    sync_status: BTreeMap<Uuid, String>,
    temp_id_mapping: BTreeMap<Uuid, usize>,
}

#[cfg(test)]
mod test {
    use std::env;
    use ::protocol::Todoist;
    use std::io::Read;
    use hyper::status::StatusCode;
    use hyper::client::Response;

    fn init() -> Todoist {
        ::env_logger::init().unwrap();
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
    fn add_label() {
        let mut client = init();
        let mut m = client.manager();
        println!("{:?}", m.add_label("helloword"));
        println!("{:?}", m.add_label("kkk"));
        println!("{:?}", m.flush().unwrap());
    }

    #[test]
    fn order() {
        use ::protocol::Project;
        use std::collections::BTreeSet;

        let a = Project {
            id: 1,
            item_order: 1,
            name: "a".into(),
            ..Default::default()
        };
        let b = Project {
            id: 1,
            item_order: 2,
            ..Default::default()
        };
        let c = Project {
            id: 2,
            item_order: 1,
            ..Default::default()
        };
        assert_eq!(a, b);
        assert!(a < b);
        assert!(a <= b);
        assert!(a < c);
        let mut bt = BTreeSet::new();
        bt.insert(a.clone());
        bt.insert(b.clone());
        bt.insert(c.clone());

        let v: Vec<Project> = bt.clone().into_iter().collect();
        assert_eq!(v, vec![a, c, b]);

        bt.replace(Project {
            id: 1,
            item_order: 1,
            name: "b".into(),
            ..Default::default()
        });

        println!("{:?}", bt);

    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TodoistResponse {
    pub projects: Vec<Project>,
    pub notes: Vec<Note>,
    pub items: Vec<Item>,
    pub labels: Vec<Label>,
    full_sync: bool,
    pub sync_token: String,
}

impl TodoistResponse {
    pub fn is_full_sync(&self) -> bool {
        self.full_sync
    }

    pub fn get_label_by_name(&self, name: &str) -> Option<Label> {
        self.labels.iter().find(|l| l.name == name).map(|l| l.clone())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Label {
    pub name: String,
    pub id: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Item {
    pub indent: usize,
    pub item_order: usize,
    pub id: usize,
    pub date_added: String,
    pub priority: usize,
    pub project_id: usize,
    pub content: String,
    pub all_day: bool,
    pub labels: Vec<usize>,
    pub is_deleted: usize,
    pub is_archived: usize,
    pub checked: usize,
    pub in_history: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Note {
    pub is_deleted: usize,
    pub is_archived: usize,
    pub content: String,
    pub item_id: usize,
    pub project_id: usize,
    pub id: usize,
    pub posted: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Project {
    pub name: String,
    pub id: usize,
    pub item_order: usize,
    pub indent: usize,
    pub is_archived: usize,
}

macro_rules! comparable {
    ($t:ty) => {
        impl PartialEq for $t {
            fn eq(&self, other: &Self) -> bool {
                self.id == other.id
            }
        }

        impl PartialOrd for $t {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(if self.id == other.id {
                    Ordering::Equal
                } else if self.item_order < other.item_order ||
                        (self.item_order == other.item_order && self.id < other.id) {
                    Ordering::Less
                } else {
                    Ordering::Greater
                })
            }
        }

        impl Eq for $t {}

        impl Ord for $t {
            fn cmp(&self, other: &Self) -> Ordering {
                self.partial_cmp(other).unwrap()
            }
        }
    };
    ($($t:ty),*) => {
        $(comparable!($t);)*
    }
}

comparable!(Project, Item);
