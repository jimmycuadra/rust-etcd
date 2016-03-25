//! Contains the etcd client. All API calls are made via the client.
use std::collections::HashMap;
use std::io::Read;

use hyper::status::StatusCode;
use serde_json::from_str;
use url::{form_urlencoded, ParseError, Url};

use keys::KeySpaceResult;
use error::{EtcdResult, Error};
use http;
use options::{ComparisonConditions, DeleteOptions, GetOptions, SetOptions};
use query_pairs::UrlWithQueryPairs;
use stats::{LeaderStats, SelfStats};
use version::VersionInfo;

/// API client for etcd. All API calls are made via the client.
#[derive(Debug)]
pub struct Client {
    root_url: String,
}

/// The entry point for all etcd API calls.
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
    ) -> KeySpaceResult {
        self.raw_delete(
            key,
            DeleteOptions {
                conditions: Some(ComparisonConditions {
                    value: current_value,
                    modified_index: current_modified_index,
                }),
                ..Default::default()
            }
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
    ) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                conditions: Some(ComparisonConditions {
                    value: current_value,
                    modified_index: current_modified_index,
                }),
                ttl: ttl,
                value: Some(value),
                ..Default::default()
            },
        )
    }

    /// Creates a new key-value pair with any given time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists.
    pub fn create(&self, key: &str, value: &str, ttl: Option<u64>) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                prev_exist: Some(false),
                ttl: ttl,
                value: Some(value),
                ..Default::default()
            },
        )
    }

    /// Creates a new empty directory at the given key with the given time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists.
    pub fn create_dir(&self, key: &str, ttl: Option<u64>) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                dir: Some(true),
                prev_exist: Some(false),
                ttl: ttl,
                ..Default::default()
            },
        )
    }

    /// Creates a new key-value pair in the given directory with any given time to live in seconds
    /// and a key name guaranteed to be greater than all existing keys in the directory.
    ///
    /// # Failures
    ///
    /// Fails if the key already exists and is not a directory.
    pub fn create_in_order(&self, key: &str, value: &str, ttl: Option<u64>) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                create_in_order: true,
                ttl: ttl,
                value: Some(value),
                ..Default::default()
            },
        )
    }

    /// Deletes a key-value pair or directory.
    ///
    /// If `recursive` is `true` and the key is a directory, the directory and all child key-value
    /// pairs and directories will be deleted.
    ///
    /// # Failures
    ///
    /// Fails if the key is a directory and `recursive` is `false`.
    pub fn delete(&self, key: &str, recursive: bool) -> KeySpaceResult {
        self.raw_delete(
            key,
            DeleteOptions {
                recursive: Some(recursive),
                ..Default::default()
            }
        )
    }

    /// Deletes an empty directory or a key-value pair at the given key.
    ///
    /// # Failures
    ///
    /// Fails if the directory is not empty.
    pub fn delete_dir(&self, key: &str) -> KeySpaceResult {
        self.raw_delete(
            key,
            DeleteOptions {
                dir: Some(true),
                ..Default::default()
            }
        )
    }

    /// Gets the value of a key.
    ///
    /// If the key is a directory, `sort` will determine whether the
    /// contents of the directory are returned in a sorted order.
    ///
    /// If the key is a directory and `recursive` is `true`, the contents of child directories will
    /// be returned as well.
    ///
    /// If `strong_consistency` is `true`, the etcd node serving the response will synchronize with
    /// the quorum before returning the value. This is slower but avoids possibly stale data from
    /// being returned.
    pub fn get(
        &self,
        key: &str,
        sort: bool,
        recursive: bool,
        strong_consistency: bool,
    ) -> KeySpaceResult {
        self.raw_get(
            key,
            GetOptions {
                recursive: recursive,
                sort: Some(sort),
                strong_consistency: strong_consistency,
                ..Default::default()
            },
        )
    }

    /// Returns statistics on the leader member of a cluster.
    ///
    /// # Failures
    ///
    /// Fails if JSON decoding fails, which suggests a bug in our schema.
    pub fn leader_stats(&self) -> EtcdResult<LeaderStats> {
        let url = format!("{}v2/stats/leader", self.root_url);
        let mut response = try!(http::get(url));
        let mut response_body = String::new();
        try!(response.read_to_string(&mut response_body));

        match response.status {
            StatusCode::Ok => Ok(from_str(&response_body).unwrap()),
            _ => Err(Error::Etcd(from_str(&response_body).unwrap()))
        }
    }

    /// Returns statistics on a cluster member.
    ///
    /// # Failures
    ///
    /// Fails if JSON decoding fails, which suggests a bug in our schema.
    pub fn self_stats(&self) -> EtcdResult<SelfStats> {
        let url = format!("{}v2/stats/self", self.root_url);
        let mut response = try!(http::get(url));
        let mut response_body = String::new();
        try!(response.read_to_string(&mut response_body));

        match response.status {
            StatusCode::Ok => Ok(from_str(&response_body).unwrap()),
            _ => Err(Error::Etcd(from_str(&response_body).unwrap()))
        }
    }

    /// Sets the key to the given value with the given time to live in seconds. Any previous value
    /// and TTL will be replaced.
    ///
    /// # Failures
    ///
    /// Fails if the key is a directory.
    pub fn set(&self, key: &str, value: &str, ttl: Option<u64>) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                ttl: ttl,
                value: Some(value),
                ..Default::default()
            },
        )
    }

    /// Sets the key to an empty directory with the given time to live in seconds. An existing
    /// key-value will be replaced, but an existing directory will not.
    ///
    /// # Failures
    ///
    /// Fails if the key is an existing directory.
    pub fn set_dir(&self, key: &str, ttl: Option<u64>) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                dir: Some(true),
                ttl: ttl,
                ..Default::default()
            },
        )
    }

    /// Updates the given key to the given value and time to live in seconds.
    ///
    /// # Failures
    ///
    /// Fails if the key does not exist.
    pub fn update(&self, key: &str, value: &str, ttl: Option<u64>) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                prev_exist: Some(true),
                ttl: ttl,
                value: Some(value),
                ..Default::default()
            },
        )
    }

    /// Updates the given key to a directory with the given time to live in seconds. If the
    /// directory already existed, only the TTL is updated. If the key was a key-value pair, its
    /// value is removed and its TTL is updated.
    ///
    /// # Failures
    ///
    /// Fails if the key does not exist.
    pub fn update_dir(&self, key: &str, ttl: Option<u64>) -> KeySpaceResult {
        self.raw_set(
            key,
            SetOptions {
                dir: Some(true),
                prev_exist: Some(true),
                ttl: ttl,
                ..Default::default()
            },
        )
    }

    /// Returns the versions of the etcd cluster and server.
    pub fn version(&self) -> EtcdResult<VersionInfo> {
        let url = format!("{}version", self.root_url);
        let mut response = try!(http::get(url));
        let mut response_body = String::new();
        try!(response.read_to_string(&mut response_body));

        match response.status {
            StatusCode::Ok => Ok(from_str(&response_body).unwrap()),
            _ => Err(Error::Etcd(from_str(&response_body).unwrap()))
        }
    }

    /// Watches etcd for changes to the given key (including all child keys if `recursive` is
    /// `true`,) and returns the new value as soon as a change takes place.
    ///
    /// The watch will return the first change indexed with `index` or greater, if specified,
    /// allowing you to watch for changes that happened in the past.
    ///
    /// # Failures
    ///
    /// Fails if a supplied `index` value is too old and has been flushed out of etcd's internal
    /// store of the most recent change events. In this case, the key should be queried for its
    /// latest "modified index" value and that should be used as the new `index` on a subsequent
    /// `watch`.
    pub fn watch(&self, key: &str, index: Option<u64>, recursive: bool) -> KeySpaceResult {
        self.raw_get(
            key,
            GetOptions {
                recursive: recursive,
                wait_index: index,
                wait: true,
                ..Default::default()
            },
        )
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
        options: DeleteOptions,
    ) -> KeySpaceResult {
        let mut query_pairs = HashMap::new();

        if options.recursive.is_some() {
            query_pairs.insert("recursive", format!("{}", options.recursive.unwrap()));
        }

        if options.dir.is_some() {
            query_pairs.insert("dir", format!("{}", options.dir.unwrap()));
        }

        if options.conditions.is_some() {
            let conditions = options.conditions.unwrap();

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
            StatusCode::Ok => Ok(from_str(&response_body).unwrap()),
            _ => Err(Error::Etcd(from_str(&response_body).unwrap())),
        }
    }

    /// Handles all get operations.
    fn raw_get(&self, key: &str, options: GetOptions) -> KeySpaceResult {
        let mut query_pairs = HashMap::new();

        query_pairs.insert("recursive", format!("{}", options.recursive));

        if options.sort.is_some() {
            query_pairs.insert("sorted", format!("{}", options.sort.unwrap()));
        }

        if options.wait {
            query_pairs.insert("wait", "true".to_string());
        }

        if options.wait_index.is_some() {
            query_pairs.insert("waitIndex", format!("{}", options.wait_index.unwrap()));
        }

        let url = UrlWithQueryPairs {
            pairs: &query_pairs,
            url: self.build_url(key),
        }.parse();

        let mut response = try!(http::get(format!("{}", url)));
        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Ok => Ok(from_str(&response_body).unwrap()),
            _ => Err(Error::Etcd(from_str(&response_body).unwrap())),
        }
    }

    /// Handles all set operations.
    fn raw_set(
        &self,
        key: &str,
        options: SetOptions,
    ) -> KeySpaceResult {
        let url = self.build_url(key);
        let mut http_options = vec![];

        if options.value.is_some() {
            http_options.push(("value".to_string(), options.value.unwrap().to_string()));
        }

        if options.ttl.is_some() {
            http_options.push(("ttl".to_string(), format!("{}", options.ttl.unwrap())));
        }

        if options.dir.is_some() {
            http_options.push(("dir".to_string(), format!("{}", options.dir.unwrap())));
        }

        if options.prev_exist.is_some() {
            http_options.push(
                ("prevExist".to_string(), format!("{}", options.prev_exist.unwrap()))
            );
        }

        if options.conditions.is_some() {
            let conditions = options.conditions.unwrap();

            if conditions.is_empty() {
                return Err(
                    Error::InvalidConditions("Current value or modified index is required.")
                );
            }

            if conditions.modified_index.is_some() {
                http_options.push(
                    ("prevIndex".to_string(), format!("{}", conditions.modified_index.unwrap()))
                );
            }

            if conditions.value.is_some() {
                http_options.push(("prevValue".to_string(), conditions.value.unwrap().to_string()));
            }
        }

        let body = form_urlencoded::serialize(&http_options);

        let mut response = if options.create_in_order {
            try!(http::post(url, body))
        } else {
            try!(http::put(url, body))
        };

        let mut response_body = String::new();
        response.read_to_string(&mut response_body).unwrap();

        match response.status {
            StatusCode::Created | StatusCode::Ok => Ok(from_str(&response_body).unwrap()),
            _ => Err(Error::Etcd(from_str(&response_body).unwrap())),
        }
    }
}
