use hyper::Client;
use hyper::client::Response;
use hyper::header::{ContentType, UserAgent, Accept, qitem};
use hyper::status::StatusCode;

use serde_json::from_reader;
use serde_json::value::Value;

use error::*;

use std::io::Read;
use std::collections::BTreeMap;
use std::cmp::PartialOrd;
use std::cmp::Ordering;

use url::form_urlencoded;

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
        Todoist {
            token: token.into(),
            sync_token: "*".into(),
            client: Client::new(),
        }
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
        let mut resp = self.post(vec![
            ("sync_token", self.sync_token.clone()),
            ("resource_types", "[\"all\"]".into())
            ])?;

        if resp.status != StatusCode::Ok {
            Err(Error::InternalError(format!("status code is: {}", resp.status)))
        } else {
            let mut body = String::new();
            let _ = resp.read_to_string(&mut body)?;
            let result: TodoistResponse = ::serde_json::from_str(&body)?;
            self.sync_token = result.sync_token.clone();
            Ok(result)
        }
    }

    pub fn add_label(&mut self, name: &str) -> Result<Label> {
        let temp_id = format!("{}", ::uuid::Uuid::new_v4());
        let resp = self.post(vec![
            ("commands", ::json::stringify(
                array! {   
                    object! {
                        "type" => "label_add",
                        "args" => object! {
                            "name" => name
                        },
                        "temp_id" => temp_id.clone(),
                        "uuid" => format!("{}", ::uuid::Uuid::new_v4())
                    }
                }
            )),
        ])?;

        let result: BTreeMap<String, Value> = from_reader(resp)?;

        let id =
            result["temp_id_mapping"].as_object().unwrap()[&temp_id].as_u64().unwrap() as usize;

        Ok(Label {
            id: id,
            name: name.into(),
        })
    }
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
        let mut resp = init().add_label("helloword").unwrap();
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

    pub fn merge(self, rhs: Self) -> Self {
        println!("{:?}", rhs);
        self
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Project {
    pub name: String,
    pub id: usize,
    pub item_order: usize,
    pub indent: usize,
    pub is_archived: usize,
}

impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Project {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(if self.item_order < other.item_order {
            Ordering::Less
        } else if self.item_order == other.item_order {
            Ordering::Equal
        } else {
            Ordering::Greater
        })
    }
}

impl Eq for Project {}

impl Ord for Project {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(if self.item_order < other.item_order {
            Ordering::Less
        } else if self.item_order == other.item_order {
            Ordering::Equal
        } else {
            Ordering::Greater
        })
    }
}

impl Eq for Item {}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}