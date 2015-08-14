use std::collections::HashMap;
use std::io::Read;

use hyper::status::StatusCode;
use rustc_serialize::json;
use url::{ParseError, Url};
use url::form_urlencoded;

use conditions::ComparisonConditions;
use error::Error;
use http;
use query_pairs::UrlWithQueryPairs;
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

    /// Deletes a key only if the given current value and/or current modified index match.
    ///
    /// # Failures
    ///
    /// Fails if the conditions didn't match or if no conditions were given.
    pub fn compare_and_delete(
        &self,
        key: &str,
        current_value: Option<&str>,
        current_modified_index: Option<u64>
    ) -> EtcdResult {
        self.raw_delete(
            key,
            None,
            None,
            Some(ComparisonConditions {
                value: current_value,
                modified_index: current_modified_index,
            }),
        )
    }

    /// Updates the value of a key only if the given current value and/or current modified index
    /// match.
    ///
    /// # Failures
    ///
    /// Fails if the conditions didn't match or if no conditions were given.
    pub fn compare_and_swap(
        &self,
        key: &str,
        value: &str,
        ttl: Option<u64>,
        current_value: Option<&str>,
        current_modified_index: Option<u64>,
    ) -> EtcdResult {
        self.raw_set(
            key,
            Some(value),
            ttl,
            None,
            Some(true),
            Some(ComparisonConditions {
                value: current_value,
                modified_index: current_modified_index,
            }),
            false,
        )
    }

    /// Creates a new file at the given key with the given value and time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists.
    pub fn create(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, Some(false), None, false)
    }

    /// Creates a new empty directory at the given key with the given time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists.
    pub fn create_dir(&self, key: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, None, ttl, Some(true), Some(false), None, false)
    }

    /// Creates a new file in the given directory with the given value and time to live in seconds
    /// with a key name guaranteed to be greater than all existing keys in the directory.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists and is not a directory.
    pub fn create_in_order(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, None, None, true)
    }

    /// Deletes a file or directory at the given key. If `recursive` is `true` and the key is a
    /// directory, the directory and all child files and directories will be deleted.
    ///
    /// # Failures
    ///
    /// Fails if the key is a directory and `recursive` is `false`.
    pub fn delete(&self, key: &str, recursive: bool) -> EtcdResult {
        self.raw_delete(key, Some(recursive), None, None)
    }

    /// Deletes an empty directory or a file at the given key.
    ///
    /// # Failures
    ///
    /// Fails if the directory is not empty.
    pub fn delete_dir(&self, key: &str) -> EtcdResult {
        self.raw_delete(key, None, Some(true), None)
    }

    /// Gets the value of a key. If the key is a directory, `sort` will determine whether the
    /// contents of the directory are returned in a sorted order. If the key is a directory and
    /// `recursive` is `true`, the contents of child directories will be returned as well.
    pub fn get(&self, key: &str, sort: bool, recursive: bool) -> EtcdResult {
        let mut query_pairs = HashMap::new();

        query_pairs.insert("sorted", format!("{}", sort));
        query_pairs.insert("recursive", format!("{}", recursive));

        let url = UrlWithQueryPairs {
            pairs: &query_pairs,
            url: self.build_url(key),
        }.parse();

        let mut response = try!(http::get(format!("{}", url)));
        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
            _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
        }
    }

    /// Sets the key to the given value with the given time to live in seconds. Any previous value
    /// and TTL will be replaced.
    ///
    /// # Failures
    ///
    /// Fails if the key is a directory.
    pub fn set(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, None, None, false)
    }

    /// Sets the key to an empty directory with the given time to live in seconds. An existing file
    /// will be replaced, but an existing directory will not.
    ///
    /// # Failures
    ///
    /// Fails if the key is an existing directory.
    pub fn set_dir(&self, key: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, None, ttl, Some(true), None, None, false)
    }

    /// Updates the given key to the given value and time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key does not exist.
    pub fn update(&self, key: &str, value: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, Some(value), ttl, None, Some(true), None, false)
    }

    /// Updates the given key to a directory with the given time to live in seconds. If the
    /// directory already existed, only the TTL is updated. If the key was a file, its value is
    /// removed and its TTL is updated.
    ///
    /// # Failures
    ///
    /// Fails if the key does not exist.
    pub fn update_dir(&self, key: &str, ttl: Option<u64>) -> EtcdResult {
        self.raw_set(key, None, ttl, Some(true), Some(true), None, false)
    }

    /// Returns statistics on the leader member of a cluster.
    ///
    /// # Failures
    ///
    /// Fails if JSON decoding fails, which suggests a bug in our schema.
    pub fn leader_stats(&self) -> Result<LeaderStats, Error> {
        let url = format!("{}v2/stats/leader", self.root_url);
        let mut response = try!(http::get(url));
        let mut response_body = String::new();
        try!(response.read_to_string(&mut response_body));

        match response.status {
            StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
            _ => Err(Error::Etcd(json::decode(&response_body).unwrap()))
        }
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
        compare_and_delete: Option<ComparisonConditions>,
    ) -> EtcdResult {
        let mut query_pairs = HashMap::new();

        if recursive.is_some() {
            query_pairs.insert("recursive", format!("{}", recursive.unwrap()));
        }

        if dir.is_some() {
            query_pairs.insert("dir", format!("{}", dir.unwrap()));
        }

        if compare_and_delete.is_some() {
            let conditions = compare_and_delete.unwrap();

            if conditions.is_empty() {
                return Err(
                    Error::InvalidConditions("Current value or modified index is required.")
                );
            }

            if conditions.modified_index.is_some() {
              query_pairs.insert("prevIndex", format!("{}", conditions.modified_index.unwrap()));
            }

            if conditions.value.is_some() {
                query_pairs.insert("prevValue", conditions.value.unwrap().to_string());
            }
        }

        let url = UrlWithQueryPairs {
            pairs: &query_pairs,
            url: self.build_url(key),
        }.parse();

        let mut response = try!(http::delete(format!("{}", url)));
        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
            _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
        }
    }

    /// Handles all set operations.
    fn raw_set(
        &self,
        key: &str,
        value: Option<&str>,
        ttl: Option<u64>,
        dir: Option<bool>,
        prev_exist: Option<bool>,
        compare_and_swap: Option<ComparisonConditions>,
        create_in_order: bool,
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

        if compare_and_swap.is_some() {
            let conditions = compare_and_swap.unwrap();

            if conditions.is_empty() {
                return Err(
                    Error::InvalidConditions("Current value or modified index is required.")
                );
            }

            if conditions.modified_index.is_some() {
                options.push(
                    ("prevIndex".to_string(), format!("{}", conditions.modified_index.unwrap()))
                );
            }

            if conditions.value.is_some() {
                options.push(("prevValue".to_string(), conditions.value.unwrap().to_string()));
            }
        }

        let body = form_urlencoded::serialize(&options);

        let mut response = if create_in_order {
            try!(http::post(url, body))
        } else {
            try!(http::put(url, body))
        };

        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Created | StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
            _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
        }
    }
}
