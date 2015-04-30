use std::io::Read;

use hyper::Client as HyperClient;
use hyper::HttpError;
use hyper::client::Response as HyperResponse;
use hyper::header::ContentType;
use hyper::status::StatusCode;
use rustc_serialize::json;
use url::{ParseError, Url};
use url::form_urlencoded::serialize_owned;

#[derive(Debug)]
#[derive(RustcDecodable)]
pub struct Response {
    pub action: String,
    pub node: Node,
    pub prev_node: Option<Node>,
}

#[derive(Debug)]
#[derive(RustcDecodable)]
pub struct Node {
    pub created_index: Option<u64>,
    pub dir: Option<bool>,
    pub expiration: Option<String>,
    pub key: Option<String>,
    pub modified_index: Option<u64>,
    pub nodes: Option<Vec<Node>>,
    pub ttl: Option<i64>,
    pub value: Option<String>,
}

#[derive(Debug)]
pub enum Error {
    EtcdError(EtcdError),
    HttpError(HttpError),
}

#[derive(Debug)]
#[derive(RustcDecodable)]
#[allow(non_snake_case)]
pub struct EtcdError {
    pub cause: Option<String>,
    pub errorCode: u64,
    pub index: u64,
    pub message: String,
}

#[derive(Debug)]
pub struct Client {
    root_url: String,
}

impl Client {
    pub fn new(root_url: &str) -> Result<Client, ParseError> {
        let url = try!(Url::parse(root_url));
        let client = Client {
            root_url: format!("{}", url.serialize()),
        };

        Ok(client)
    }

    pub fn default() -> Client {
        Client {
            root_url: "http://127.0.0.1:2379/".to_string(),
        }
    }

    pub fn get(&self, key: &str) -> Result<Response, EtcdError> {
        Err(EtcdError)
    }

    pub fn mk(&self, key: &str, value: &str, ttl: Option<u64>) -> Result<Response, Error> {
        let url = self.build_url(key);
        let mut options = vec![];

        options.push(("value".to_string(), value.to_string()));
        options.push(("prevExist".to_string(), "false".to_string()));

        if ttl.is_some() {
            options.push(("ttl".to_string(), format!("{}", ttl.unwrap())));
        }

        let body = serialize_owned(&options);

        match self.put(url, body) {
            Ok(mut response) => {
                let mut response_body = String::new();

                response.read_to_string(&mut response_body).unwrap();

                println!("{:?} - {:?} - {:?}", response.status, response.headers, response_body);

                match response.status {
                    StatusCode::Created => Ok(json::decode(&response_body).unwrap()),
                    _ => Err(Error::EtcdError(json::decode(&response_body).unwrap())),
                }
            },
            Err(error) => Err(Error::HttpError(error)),
        }
    }

    pub fn mkdir(&self, key: &str, ttl: Option<u64>) -> Result<Response, EtcdError> {
        Err(EtcdError)
    }

    pub fn set(
        &self,
        key: &str,
        value: &str,
        ttl: Option<u64>,
        prev_value: Option<&str>,
        prev_index: Option<u64>
    ) -> Result<Response, EtcdError> {
        Err(EtcdError)
    }

    // private

    fn build_url(&self, path: &str) -> String {
        format!("{}v2/keys{}", self.root_url, path)
    }

    fn put(&self, url: String, body: String) -> Result<HyperResponse, HttpError> {
        let mut client = HyperClient::new();
        let content_type: ContentType = ContentType(
            "application/x-www-form-urlencoded".parse().unwrap()
        );

        client.put(&url[..]).header(content_type).body(&body[..]).send()
    }
}

#[cfg(test)]
mod mk_tests {
    use super::{Client, Error};

    #[test]
    fn mk_key() {
        let client = Client::new("http://etcd:2379").unwrap();

        let response = client.mk("/foo", "bar", Some(100)).ok().unwrap();

        assert_eq!(response.action, "create".to_string());
        assert_eq!(response.node.value.unwrap(), "bar".to_string());
        assert_eq!(response.node.ttl.unwrap(), 100);
    }

    #[test]
    fn mk_key_failure() {
        let client = Client::new("http://etcd:2379").unwrap();

        assert!(client.mk("/foo", "bar", None).is_ok());

        match client.mk("/foo", "bar", None).err().unwrap() {
            Error::EtcdError(error) => assert_eq!(error.message, "Key already exists".to_string()),
            _ => panic!("expected EtcdError due to pre-existing key"),
        };
    }
}

#[cfg(test)]
mod mkdir_tests {
    use super::Client;

    #[test]
    fn mkdir() {
        let client = Client::default();

        assert!(client.mkdir("/foo", None).ok().unwrap().node.dir.unwrap());
    }

    #[test]
    fn mkdir_failure() {
        let client = Client::default();

        assert!(client.mkdir("/foo", None).is_ok());
        assert!(client.mkdir("/foo", None).is_err());
    }
}

#[cfg(test)]
mod get_tests {
    use super::Client;

    #[test]
    fn set_and_get_key() {
        let client = Client::default();

        assert!(client.set("/foo", "bar", None, None, None).is_ok());
        assert_eq!(client.get("/foo").ok().unwrap().node.value.unwrap(), "bar");
    }
}
