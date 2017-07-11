//! Contains the etcd client. All API calls are made via the client.

use std::collections::HashMap;
use std::default::Default;
use std::str::FromStr;

use futures::{Future, IntoFuture, Stream};
use futures::future::{err, ok};
use futures::stream::futures_unordered;
use hyper::{StatusCode, Uri};
use native_tls::TlsConnector;
use serde_json;
use tokio_core::reactor::Handle;
use url::Url;
use url::form_urlencoded::Serializer;

use async::first_ok;
use error::{ApiError, Error};
use http::HttpClient;
use keys::{FutureKeySpaceInfo, KeySpaceInfo};
use member::Member;
use options::{ComparisonConditions, DeleteOptions, GetOptions, SetOptions};
use stats::{LeaderStats, SelfStats, StoreStats};
use version::VersionInfo;

/// API client for etcd. All API calls are made via the client.
pub struct Client {
    http_client: HttpClient,
    members: Vec<Member>,
}

/// Options for configuring the behavior of a `Client`.
#[derive(Default)]
pub struct ClientOptions {
    /// A `native_tls::TlsConnector` configured as desired for HTTPS connections.
    pub tls_connector: Option<TlsConnector>,
    /// The username and password to use for authentication.
    pub username_and_password: Option<(String, String)>,
}

impl Client {
    /// Constructs a new client. `endpoints` are URLs for the etcd cluster members to the client
    /// will make API calls to.
    ///
    /// Fails if no endpoints are provided or if any of the endpoints is an invalid URL.
    pub fn new(endpoints: &[&str], handle: &Handle) -> Result<Client, Error> {
        Client::with_options(endpoints, handle, ClientOptions::default())
    }

    /// Constructs a new client with the given options. `endpoints` are URLs for the etcd cluster
    /// members to the client will make API calls to.
    ///
    /// Fails if no endpoints are provided or if any of the endpoints is an invalid URL.
    pub fn with_options(endpoints: &[&str], handle: &Handle, options: ClientOptions) ->
    Result<Client, Error> {
        if endpoints.len() < 1 {
            return Err(Error::NoEndpoints);
        }

        let mut members = Vec::with_capacity(endpoints.len());

        for endpoint in endpoints {
            members.push(Member::new(endpoint)?);
        }

        Ok(Client {
            http_client: HttpClient::new(handle, options)?,
            members: members,
        })
    }

    /// Deletes a key only if the given current value and/or current modified index match.
    ///
    /// Fails if the conditions didn't match or if no conditions were given.
    pub fn compare_and_delete(
        &self,
        key: &str,
        current_value: Option<&str>,
        current_modified_index: Option<u64>
    ) -> FutureKeySpaceInfo {
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
    /// Fails if the conditions didn't match or if no conditions were given.
    pub fn compare_and_swap(
        &self,
        key: &str,
        value: &str,
        ttl: Option<u64>,
        current_value: Option<&str>,
        current_modified_index: Option<u64>,
    ) -> FutureKeySpaceInfo {
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
    /// Fails if the key already exists.
    pub fn create(&self, key: &str, value: &str, ttl: Option<u64>) -> FutureKeySpaceInfo {
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
    /// Fails if the key already exists.
    pub fn create_dir(&self, key: &str, ttl: Option<u64>) -> FutureKeySpaceInfo {
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
    /// Fails if the key already exists and is not a directory.
    pub fn create_in_order(&self, key: &str, value: &str, ttl: Option<u64>) -> FutureKeySpaceInfo {
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
    /// Fails if the key is a directory and `recursive` is `false`.
    pub fn delete(&self, key: &str, recursive: bool) -> FutureKeySpaceInfo {
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
    /// Fails if the directory is not empty.
    pub fn delete_dir(&self, key: &str) -> FutureKeySpaceInfo {
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
    ) -> FutureKeySpaceInfo {
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

    /// Returns statistics about the leader member of a cluster.
    ///
    /// Fails if JSON decoding fails, which suggests a bug in our schema.
    pub fn leader_stats(&self) -> Box<Future<Item = LeaderStats, Error = Error>> {
        let url = format!("{}v2/stats/leader", self.members[0].endpoint);
        let uri = url.parse().map_err(Error::from).into_future();
        let cloned_client = self.http_client.clone();
        let response = uri.and_then(move |uri| cloned_client.get(uri).map_err(Error::from));
        let result = response.and_then(|response| {
            let status = response.status();
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| {
                if status == StatusCode::Ok {
                    match serde_json::from_slice::<LeaderStats>(body) {
                        Ok(stats) => ok(stats),
                        Err(error) => err(Error::Serialization(error)),
                    }
                } else {
                    match serde_json::from_slice::<ApiError>(body) {
                        Ok(error) => err(Error::Api(error)),
                        Err(error) => err(Error::Serialization(error)),
                    }
                }
            })
        });

        Box::new(result)
    }

    /// Returns statistics about each cluster member the client was initialized with.
    ///
    /// Fails if JSON decoding fails, which suggests a bug in our schema.
    pub fn self_stats(&self) -> Box<Stream<Item = SelfStats, Error = Error>> {
        let futures = self.members.iter().map(|member| {
            let url = format!("{}v2/stats/self", member.endpoint);
            let uri = url.parse().map_err(Error::from).into_future();
            let cloned_client = self.http_client.clone();
            let response = uri.and_then(move |uri| cloned_client.get(uri).map_err(Error::from));
            response.and_then(|response| {
                let status = response.status();
                let body = response.body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    if status == StatusCode::Ok {
                        match serde_json::from_slice::<SelfStats>(body) {
                            Ok(stats) => ok(stats),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    } else {
                        match serde_json::from_slice::<ApiError>(body) {
                            Ok(error) => err(Error::Api(error)),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    }
                })
            })
        });

        Box::new(futures_unordered(futures))
    }

    /// Sets the key to the given value with the given time to live in seconds. Any previous value
    /// and TTL will be replaced.
    ///
    /// Fails if the key is a directory.
    pub fn set(&self, key: &str, value: &str, ttl: Option<u64>) -> FutureKeySpaceInfo {
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
    /// Fails if the key is an existing directory.
    pub fn set_dir(&self, key: &str, ttl: Option<u64>) -> FutureKeySpaceInfo {
        self.raw_set(
            key,
            SetOptions {
                dir: Some(true),
                ttl: ttl,
                ..Default::default()
            },
        )
    }

    /// Returns statistics about operations handled by each etcd member the client was initialized
    /// with.
    ///
    /// Fails if JSON decoding fails, which suggests a bug in our schema.
    pub fn store_stats(&self) -> Box<Stream<Item = StoreStats, Error = Error>> {
        let futures = self.members.iter().map(|member| {
            let url = format!("{}v2/stats/store", member.endpoint);
            let uri = url.parse().map_err(Error::from).into_future();
            let cloned_client = self.http_client.clone();
            let response = uri.and_then(move |uri| cloned_client.get(uri).map_err(Error::from));
            response.and_then(|response| {
                let status = response.status();
                let body = response.body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    if status == StatusCode::Ok {
                        match serde_json::from_slice::<StoreStats>(body) {
                            Ok(stats) => ok(stats),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    } else {
                        match serde_json::from_slice::<ApiError>(body) {
                            Ok(error) => err(Error::Api(error)),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    }
                })
            })
        });

        Box::new(futures_unordered(futures))
    }

    /// Updates the given key to the given value and time to live in seconds.
    ///
    /// Fails if the key does not exist.
    pub fn update(&self, key: &str, value: &str, ttl: Option<u64>) -> FutureKeySpaceInfo {
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
    /// Fails if the key does not exist.
    pub fn update_dir(&self, key: &str, ttl: Option<u64>) -> FutureKeySpaceInfo {
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

    /// Returns version information from each etcd cluster member the client was initialized with.
    pub fn versions(&self) -> Box<Stream<Item = VersionInfo, Error = Error>> {
        let futures = self.members.iter().map(|member| {
            let url = format!("{}version", member.endpoint);
            let uri = url.parse().map_err(Error::from).into_future();
            let cloned_client = self.http_client.clone();
            let response = uri.and_then(move |uri| cloned_client.get(uri).map_err(Error::from));
            response.and_then(|response| {
                let status = response.status();
                let body = response.body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    if status == StatusCode::Ok {
                        match serde_json::from_slice::<VersionInfo>(body) {
                            Ok(stats) => ok(stats),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    } else {
                        match serde_json::from_slice::<ApiError>(body) {
                            Ok(error) => err(Error::Api(error)),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    }
                })
            })
        });

        Box::new(futures_unordered(futures))
    }

    /// Watches etcd for changes to the given key (including all child keys if `recursive` is
    /// `true`,) and returns the new value as soon as a change takes place.
    ///
    /// The watch will return the first change indexed with `index` or greater, if specified,
    /// allowing you to watch for changes that happened in the past.
    ///
    /// Fails if a supplied `index` value is too old and has been flushed out of etcd's internal
    /// store of the most recent change events. In this case, the key should be queried for its
    /// latest "modified index" value and that should be used as the new `index` on a subsequent
    /// `watch`.
    pub fn watch(&self, key: &str, index: Option<u64>, recursive: bool) -> FutureKeySpaceInfo {
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

    /// Handles all delete operations.
    fn raw_delete(
        &self,
        key: &str,
        options: DeleteOptions,
    ) -> FutureKeySpaceInfo {
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
                return Box::new(Err(
                    vec![Error::InvalidConditions("Current value or modified index is required.")]
                ).into_future());
            }

            if conditions.modified_index.is_some() {
              query_pairs.insert("prevIndex", format!("{}", conditions.modified_index.unwrap()));
            }

            if conditions.value.is_some() {
                query_pairs.insert("prevValue", conditions.value.unwrap().to_owned());
            }
        }

        let http_client = self.http_client.clone();
        let key = key.to_string();

        let result = first_ok(self.members.clone(), move |member| {
            let url = Url::parse_with_params(
                &build_url(member, &key),
                query_pairs.clone(),
            ).map_err(Error::from).into_future();

            let uri = url.and_then(|url| {
                Uri::from_str(url.as_str()).map_err(Error::from).into_future()
            });

            let http_client = http_client.clone();

            let response = uri.and_then(move |uri| http_client.delete(uri).map_err(Error::from));

            let result = response.and_then(move |response| {
                let status = response.status();
                let body = response.body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    if status == StatusCode::Ok {
                        match serde_json::from_slice::<KeySpaceInfo>(body) {
                            Ok(key_space_info) => ok(key_space_info),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    } else {
                        match serde_json::from_slice::<ApiError>(body) {
                            Ok(error) => err(Error::Api(error)),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    }
                })
            });

            Box::new(result)
        });

        Box::new(result)
    }

    /// Handles all get operations.
    fn raw_get(&self, key: &str, options: GetOptions) -> FutureKeySpaceInfo {
        let mut query_pairs = HashMap::new();

        query_pairs.insert("recursive", format!("{}", options.recursive));

        if options.sort.is_some() {
            query_pairs.insert("sorted", format!("{}", options.sort.unwrap()));
        }

        if options.wait {
            query_pairs.insert("wait", "true".to_owned());
        }

        if options.wait_index.is_some() {
            query_pairs.insert("waitIndex", format!("{}", options.wait_index.unwrap()));
        }

        let http_client = self.http_client.clone();
        let key = key.to_string();

        let result = first_ok(self.members.clone(), move |member| {
            let url = Url::parse_with_params(
                &build_url(member, &key),
                query_pairs.clone(),
            ).map_err(Error::from).into_future();

            let uri = url.and_then(|url| {
                Uri::from_str(url.as_str()).map_err(Error::from).into_future()
            });

            let http_client = http_client.clone();

            let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));

            let result = response.and_then(|response| {
                let status = response.status();
                let body = response.body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    if status == StatusCode::Ok {
                        match serde_json::from_slice::<KeySpaceInfo>(body) {
                            Ok(key_space_info) => ok(key_space_info),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    } else {
                        match serde_json::from_slice::<ApiError>(body) {
                            Ok(error) => err(Error::Api(error)),
                            Err(error) => err(Error::Serialization(error)),
                        }
                    }
                })
            });

            Box::new(result)
        });

        Box::new(result)
    }

    /// Handles all set operations.
    fn raw_set(
        &self,
        key: &str,
        options: SetOptions,
    ) -> FutureKeySpaceInfo {
        let mut http_options = vec![];

        if let Some(ref value) = options.value {
            http_options.push(("value".to_owned(), value.to_string()));
        }

        if let Some(ref ttl) = options.ttl {
            http_options.push(("ttl".to_owned(), ttl.to_string()));
        }

        if let Some(ref dir) = options.dir {
            http_options.push(("dir".to_owned(), dir.to_string()));
        }

        if let Some(ref prev_exist) = options.prev_exist {
            http_options.push(
                ("prevExist".to_owned(), prev_exist.to_string())
            );
        }

        if let Some(ref conditions) = options.conditions {
            if conditions.is_empty() {
                return Box::new(Err(
                    vec![Error::InvalidConditions("Current value or modified index is required.")]
                ).into_future());
            }

            if let Some(ref modified_index) = conditions.modified_index {
                http_options.push(("prevIndex".to_owned(), modified_index.to_string()));
            }

            if let Some(ref value) = conditions.value {
                http_options.push(("prevValue".to_owned(), value.to_string()));
            }
        }

        let http_client = self.http_client.clone();
        let key = key.to_string();
        let create_in_order = options.create_in_order;

        let result = first_ok(self.members.clone(), move |member| {
            let mut serializer = Serializer::new(String::new());
            serializer.extend_pairs(http_options.clone());
            let body = serializer.finish();

            let url = build_url(member, &key);
            let uri = Uri::from_str(url.as_str()).map_err(Error::from).into_future();

            let http_client = http_client.clone();

            let response = uri.and_then(move |uri| {
                if create_in_order {
                    http_client.post(uri, body).map_err(Error::from)
                } else {
                    http_client.put(uri, body).map_err(Error::from)
                }
            });

            let result = response.and_then(|response| {
                let status = response.status();
                let body = response.body().concat2().map_err(Error::from);

                body.and_then(move |ref body| {
                    match status {
                        StatusCode::Created | StatusCode::Ok => {
                            match serde_json::from_slice::<KeySpaceInfo>(body) {
                                Ok(key_space_info) => ok(key_space_info),
                                Err(error) => err(Error::Serialization(error)),
                            }
                        }
                        _ => {
                            match serde_json::from_slice::<ApiError>(body) {
                                Ok(error) => err(Error::Api(error)),
                                Err(error) => err(Error::Serialization(error)),
                            }
                        }
                    }
                })
            });

            Box::new(result)
        });

        Box::new(result)
    }
}

/// Constructs the full URL for an API call.
fn build_url(member: &Member, path: &str) -> String {
    format!("{}v2/keys{}", member.endpoint, path)
}
