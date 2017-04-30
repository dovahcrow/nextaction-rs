use std::collections::BTreeMap;
use std::cmp::{PartialOrd, Ordering};

use hyper::Client;
use hyper::client::Response;
use hyper::header::{ContentType, UserAgent, Accept, qitem};
use hyper::status::StatusCode;
use hyper::net::HttpsConnector;
use hyper_rustls::TlsClient;

use serde_json::{from_reader, from_str, to_string, Value as JsonValue};

use errors::*;

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
        let mut client = Client::with_connector(HttpsConnector::new(TlsClient::new()));
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
        self.sync_fields(&["all"])
    }

    pub fn sync_fields(&mut self, fields: &[&str]) -> Result<TodoistResponse> {
        let resp = self.post(vec![("sync_token", self.sync_token.clone()),
                       ("resource_types", format!(r#"["{}"]"#, fields.join(r#"",""#)))])?;

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
    commands: Vec<JsonValue>,
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
        self.commands.push(json!({
                "type": "label_add",
                "args": json!({
                    "name": name
                }),
                "temp_id": format!("{}", temp_id),
                "uuid": format!("{}", uuid)
            }
        ));
        (temp_id, uuid)
    }

    pub fn set_item_label(&mut self, id: usize, label_ids: Vec<usize>) -> Uuid {
        let uuid = Uuid::new_v4();
        self.commands.push(json! ({
            "type": "item_update",
            "uuid": format!("{}", uuid),
            "args": json! ({
                "id": id,
                "labels": label_ids
            })
        }));
        uuid
    }

    pub fn complete_item(&mut self, id: usize) -> Uuid {
        let uuid = Uuid::new_v4();
        self.commands.push(json!({
            "type": "close_item",
            "uuid": format!("{}", uuid),
            "args": json! ({
                "id": id
            })
        }));
        uuid
    }

    pub fn archive_project(&mut self, id: usize) -> Uuid {
        let uuid = Uuid::new_v4();
        self.commands.push(json!({
            "type" : "project_archive",
            "uuid" : format!("{}", uuid),
            "args" : json!({
                "ids" : json!([id])
            })
        }));
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
            .post(vec![("commands", to_string(&self.commands)?)])?;
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TodoistResponse {
    pub projects: Option<Vec<Project>>,
    pub notes: Option<Vec<Note>>,
    pub items: Option<Vec<Item>>,
    pub labels: Option<Vec<Label>>,
    pub user: Option<User>,
    full_sync: bool,
    pub sync_token: String,
}

impl TodoistResponse {
    pub fn is_full_sync(&self) -> bool {
        self.full_sync
    }

    pub fn get_label_by_name(&self, name: &str) -> Option<Label> {
        self.labels.as_ref().and_then(|label| label.iter().find(|l| l.name == name).cloned())
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

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct User {
    pub id: i64,
    pub token: Option<String>,
    pub email: String,
    pub full_name: String,
    pub inbox_project: i64,
    pub join_date: String,
}


// In rust, > >= < <= uses PartialOrd, == uses PartialEq, BTree* uses Ord
macro_rules! comparable {
    ($t:ty) => {
        impl PartialEq for $t {
            fn eq(&self, other: &Self) -> bool {
                // Result here doesn't matter BTree*'s result, return whatever you want.
                self.id == other.id
            }
        }

        // this impl is to sort all items in the list order we see in GUI, fron top to bottom
        impl PartialOrd for $t {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(if self.id == other.id {
                    Ordering::Equal
                } else if self.item_order < other.item_order ||
                        (self.item_order == other.item_order && self.id < other.id) { // the second situation is because when merging two
                        // btrees, the new item may have same order as an old unupdated item, but after all items are merged,
                        // there shouldn't any items with same order. So, same order with different IDs is just a transient situation.
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
