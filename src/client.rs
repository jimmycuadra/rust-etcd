use std::collections::HashMap;
use std::io::Read;

use hyper::status::StatusCode;
use rustc_serialize::json;
use url::{ParseError, Url};
use url::form_urlencoded::serialize_owned;

use error::Error;
use http;
use response::EtcdResult;

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

    pub fn create(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, Some(false))
    }

    pub fn create_dir(&self, key: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, None, ttl, Some(true), Some(false))
    }

    pub fn delete(&self, key: &str, recursive: bool) -> EtcdResult {
        self.raw_delete(key, Some(recursive), None)
    }

    pub fn delete_dir(&self, key: &str) -> EtcdResult {
        self.raw_delete(key, None, Some(true))
    }

    pub fn get(&self, key: &str, sort: bool, recursive: bool) -> EtcdResult {
        let base_url = self.build_url(key);
        let sort_string = format!("{}", sort);
        let recursive_string = format!("{}", recursive);

        let mut query_pairs = HashMap::new();

        query_pairs.insert("sorted", &sort_string[..]);
        query_pairs.insert("recursive", &recursive_string[..]);

        let mut url = Url::parse(&base_url[..]).unwrap();
        url.set_query_from_pairs(query_pairs.iter().map(|(k, v)| (*k, *v)));

        let mut response = try!(http::get(format!("{}", url)));
        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
            _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
        }
    }

    pub fn set(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, None)
    }

    pub fn update(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, Some(true))
    }

    // private

    fn build_url(&self, path: &str) -> String {
        format!("{}v2/keys{}", self.root_url, path)
    }

    fn raw_delete(
        &self,
        key: &str,
        recursive: Option<bool>,
        dir: Option<bool>,
    ) -> EtcdResult {
        let base_url = self.build_url(key);
        let recursive_string = format!("{}", recursive.unwrap_or(false));
        let dir_string = format!("{}", dir.unwrap_or(false));

        let mut query_pairs = HashMap::new();

        if recursive.is_some() {
            query_pairs.insert("recursive", &recursive_string[..]);
        }

        if dir.is_some() {
            query_pairs.insert("dir", &dir_string[..]);
        }

        let mut url = Url::parse(&base_url[..]).unwrap();
        url.set_query_from_pairs(query_pairs.iter().map(|(k, v)| (*k, *v)));

        let mut response = try!(http::delete(format!("{}", url)));
        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
            _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
        }
    }

    fn raw_set(
        &self,
        key: &str,
        value: Option<&str>,
        ttl: Option<u64>,
        dir: Option<bool>,
        prev_exist: Option<bool>,
    ) -> EtcdResult {
        let url = self.build_url(key);
        let mut options = vec![];

        if value.is_some() {
            options.push(("value".to_string(), value.unwrap().to_string()));
        }

        if ttl.is_some() {
            options.push(("ttl".to_string(), format!("{}", ttl.unwrap())));
        }

        if dir.is_some() {
            options.push(("dir".to_string(), format!("{}", dir.unwrap())));
        }

        if prev_exist.is_some() {
            options.push(("prevExist".to_string(), format!("{}", prev_exist.unwrap())));
        }

        let body = serialize_owned(&options);

        let mut response = try!(http::put(url, body));
        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Created | StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
            _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
        }
    }
}
