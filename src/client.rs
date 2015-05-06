use std::collections::HashMap;

use url::{ParseError, Url};
use url::form_urlencoded;

use error::Error;
use http;
use response::{EtcdResult, LeaderStats};

/// API client for etcd.
#[derive(Debug)]
pub struct Client {
    root_url: String,
}

impl Client {
    /// Constructs a new client.
    pub fn new(root_url: &str) -> Result<Client, ParseError> {
        let url = try!(Url::parse(root_url));
        let client = Client {
            root_url: format!("{}", url.serialize()),
        };

        Ok(client)
    }

    /// Constructs a client that will connect to http://127.0.0.1:2379/.
    pub fn default() -> Client {
        Client {
            root_url: "http://127.0.0.1:2379/".to_string(),
        }
    }

    /// Creates a new file at the given key with the given value and time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists.
    pub fn create(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, Some(false))
    }

    /// Creates a new empty directory at the given key with the given time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists.
    pub fn create_dir(&self, key: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, None, ttl, Some(true), Some(false))
    }

    /// Deletes a file or directory at the given key. If `recursive` is `true` and the key is a
    /// directory, the directory and all child files and directories will be deleted.
    ///
    /// # Failures
    ///
    /// Fails if the key is a directory and `recursive` is `false`.
    pub fn delete(&self, key: &str, recursive: bool) -> EtcdResult {
        self.raw_delete(key, Some(recursive), None)
    }

    /// Deletes an empty directory or a file at the given key.
    ///
    /// # Failures
    ///
    /// Fails if the directory is not empty.
    pub fn delete_dir(&self, key: &str) -> EtcdResult {
        self.raw_delete(key, None, Some(true))
    }

    /// Gets the value of a key. If the key is a directory, `sort` will determine whether the
    /// contents of the directory are returned in a sorted order. If the key is a directory and
    /// `recursive` is `true`, the contents of child directories will be returned as well.
    pub fn get(&self, key: &str, sort: bool, recursive: bool) -> EtcdResult {
        let base_url = self.build_url(key);
        let sort_string = format!("{}", sort);
        let recursive_string = format!("{}", recursive);

        let mut query_pairs = HashMap::new();

        query_pairs.insert("sorted", &sort_string[..]);
        query_pairs.insert("recursive", &recursive_string[..]);

        let mut url = Url::parse(&base_url[..]).unwrap();
        url.set_query_from_pairs(query_pairs.iter().map(|(k, v)| (*k, *v)));

        let response = try!(http::get(format!("{}", url)));
        http::decode(response)
    }

    /// Sets the key to the given value with the given time to live in seconds. Any previous value
    /// and TTL will be replaced.
    ///
    /// # Failures
    ///
    /// Fails if the key is a directory.
    pub fn set(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, None)
    }

    /// Updates the given key to the given value and time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key does not exist.
    pub fn update(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, Some(true))
    }

    /// Returns statistics on the leader member of a cluster.
    ///
    /// # Failures
    ///
    /// Fails if JSON decoding fails, which suggests a bug in our schema.
    pub fn leader_stats(&self) -> Result<LeaderStats, Error> {
        let url = format!("{}v2/stats/leader", self.root_url);
        let response = try!(http::get(url));
        http::decode(response)
    }

    // private

    /// Constructs the full URL for an API call.
    fn build_url(&self, path: &str) -> String {
        format!("{}v2/keys{}", self.root_url, path)
    }

    /// Handles all delete operations.
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

        let response = try!(http::delete(format!("{}", url)));
        http::decode(response)
    }

    /// Handles all set operations.
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

        let body = form_urlencoded::serialize(&options);

        let response = try!(http::put(url, body));
        http::decode(response)
    }
}
