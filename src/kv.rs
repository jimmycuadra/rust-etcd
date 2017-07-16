//! etcd's key-value API.

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use futures::future::{Future, IntoFuture, err, ok};
use futures::stream::Stream;
use hyper::{StatusCode, Uri};
use hyper::client::Connect;
use serde_json;
use tokio_timer::Timer;
use url::Url;

pub use error::WatchError;

use async::first_ok;
use client::Client;
use error::{ApiError, Error};
use member::Member;
use options::{ComparisonConditions, DeleteOptions, GetOptions, SetOptions};
use url::form_urlencoded::Serializer;

/// The future returned by most key space API calls.
///
/// On success, information about the result of the operation. On failure, an error for each cluster
/// member that failed.
pub type FutureKeyValueInfo = Box<Future<Item = KeyValueInfo, Error = Vec<Error>>>;

/// A FutureKeyValueInfo for a single etcd cluster member.
pub(crate) type FutureSingleMemberKeyValueInfo = Box<Future<Item = KeyValueInfo, Error = Error>>;

/// Information about the result of a successful key space operation.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct KeyValueInfo {
    /// The action that was taken, e.g. `get`, `set`.
    pub action: Action,
    /// The etcd `Node` that was operated upon.
    pub node: Option<Node>,
    /// The previous state of the target node.
    #[serde(rename = "prevNode")]
    pub prev_node: Option<Node>,
}

/// The type of action that was taken in response to a key value API request.
///
/// "Node" refers to the key or directory being acted upon.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub enum Action {
    /// Atomic deletion of a node based on previous state.
    #[serde(rename = "compareAndDelete")]
    CompareAndDelete,
    /// Atomtic update of a node based on previous state.
    #[serde(rename = "compareAndSwap")]
    CompareAndSwap,
    /// Creation of a node that didn't previously exist.
    #[serde(rename = "create")]
    Create,
    /// Deletion of a node.
    #[serde(rename = "delete")]
    Delete,
    /// Expiration of a node.
    #[serde(rename = "expire")]
    Expire,
    /// Retrieval of a node.
    #[serde(rename = "get")]
    Get,
    /// Assignment of a node, which may have previously existed.
    #[serde(rename = "set")]
    Set,
    /// Update of an existing node.
    #[serde(rename = "update")]
    Update,
}

/// An etcd key or directory.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Node {
    /// The new value of the etcd creation index.
    #[serde(rename = "createdIndex")]
    pub created_index: Option<u64>,
    /// Whether or not the node is a directory.
    pub dir: Option<bool>,
    /// An ISO 8601 timestamp for when the key will expire.
    pub expiration: Option<String>,
    /// The name of the key.
    pub key: Option<String>,
    /// The new value of the etcd modification index.
    #[serde(rename = "modifiedIndex")]
    pub modified_index: Option<u64>,
    /// Child nodes of a directory.
    pub nodes: Option<Vec<Node>>,
    /// The key's time to live in seconds.
    pub ttl: Option<i64>,
    /// The value of the key.
    pub value: Option<String>,
}

/// Deletes a key only if the given current value and/or current modified index match.
///
/// Fails if the conditions didn't match or if no conditions were given.
pub fn compare_and_delete<C>(
    client: &Client<C>,
    key: &str,
    current_value: Option<&str>,
    current_modified_index: Option<u64>,
) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_delete(
        client,
        key,
        DeleteOptions {
            conditions: Some(ComparisonConditions {
                value: current_value,
                modified_index: current_modified_index,
            }),
            ..Default::default()
        },
    )
}

/// Updates the value of a key only if the given current value and/or current modified index
/// match.
///
/// Fails if the conditions didn't match or if no conditions were given.
pub fn compare_and_swap<C>(
    client: &Client<C>,
    key: &str,
    value: &str,
    ttl: Option<u64>,
    current_value: Option<&str>,
    current_modified_index: Option<u64>,
) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
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
pub fn create<C>(client: &Client<C>, key: &str, value: &str, ttl: Option<u64>) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
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
pub fn create_dir<C>(client: &Client<C>, key: &str, ttl: Option<u64>) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
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
pub fn create_in_order<C>(
    client: &Client<C>,
    key: &str,
    value: &str,
    ttl: Option<u64>,
) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
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
pub fn delete<C>(client: &Client<C>, key: &str, recursive: bool) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_delete(
        client,
        key,
        DeleteOptions {
            recursive: Some(recursive),
            ..Default::default()
        },
    )
}

/// Deletes an empty directory or a key-value pair at the given key.
///
/// Fails if the directory is not empty.
pub fn delete_dir<C>(client: &Client<C>, key: &str) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_delete(
        client,
        key,
        DeleteOptions {
            dir: Some(true),
            ..Default::default()
        },
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
pub fn get<C>(
    client: &Client<C>,
    key: &str,
    sort: bool,
    recursive: bool,
    strong_consistency: bool,
) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_get(
        client,
        key,
        GetOptions {
            recursive: recursive,
            sort: Some(sort),
            strong_consistency: strong_consistency,
            ..Default::default()
        },
    )
}

/// Sets the key to the given value with the given time to live in seconds. Any previous value
/// and TTL will be replaced.
///
/// Fails if the key is a directory.
pub fn set<C>(client: &Client<C>, key: &str, value: &str, ttl: Option<u64>) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
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
pub fn set_dir<C>(client: &Client<C>, key: &str, ttl: Option<u64>) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
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
/// Fails if the key does not exist.
pub fn update<C>(client: &Client<C>, key: &str, value: &str, ttl: Option<u64>) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
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
pub fn update_dir<C>(client: &Client<C>, key: &str, ttl: Option<u64>) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
    raw_set(
        client,
        key,
        SetOptions {
            dir: Some(true),
            prev_exist: Some(true),
            ttl: ttl,
            ..Default::default()
        },
    )
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
pub fn watch<C>(
    client: &Client<C>,
    key: &str,
    index: Option<u64>,
    recursive: bool,
    timeout: Option<Duration>,
) -> Box<Future<Item = KeyValueInfo, Error = WatchError>>
where
    C: Clone + Connect,
{
    let work = raw_get(
        client,
        key,
        GetOptions {
            recursive: recursive,
            wait_index: index,
            wait: true,
            ..Default::default()
        },
    ).map_err(|errors| WatchError::Other(errors));

    if let Some(duration) = timeout {
        let timer = Timer::default();

        Box::new(timer.timeout(work, duration))
    } else {
        Box::new(work)
    }
}

/// Constructs the full URL for an API call.
fn build_url(member: &Member, path: &str) -> String {
    let maybe_slash = if member.endpoint.as_ref().ends_with("/") {
        ""
    } else {
        "/"
    };

    format!("{}{}v2/keys{}", member.endpoint, maybe_slash, path)
}

/// Handles all delete operations.
fn raw_delete<C>(client: &Client<C>, key: &str, options: DeleteOptions) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
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
            return Box::new(Err(vec![Error::InvalidConditions]).into_future());
        }

        if conditions.modified_index.is_some() {
            query_pairs.insert(
                "prevIndex",
                format!("{}", conditions.modified_index.unwrap()),
            );
        }

        if conditions.value.is_some() {
            query_pairs.insert("prevValue", conditions.value.unwrap().to_owned());
        }
    }

    let http_client = client.http_client().clone();
    let key = key.to_string();

    let result = first_ok(client.members().to_vec(), move |member| {
        let url = Url::parse_with_params(&build_url(member, &key), query_pairs.clone())
            .map_err(Error::from)
            .into_future();

        let uri = url.and_then(|url| {
            Uri::from_str(url.as_str())
                .map_err(Error::from)
                .into_future()
        });

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.delete(uri).map_err(Error::from));

        let result = response.and_then(move |response| {
            let status = response.status();
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| if status == StatusCode::Ok {
                match serde_json::from_slice::<KeyValueInfo>(body) {
                    Ok(key_space_info) => ok(key_space_info),
                    Err(error) => err(Error::Serialization(error)),
                }
            } else {
                match serde_json::from_slice::<ApiError>(body) {
                    Ok(error) => err(Error::Api(error)),
                    Err(error) => err(Error::Serialization(error)),
                }
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Handles all get operations.
fn raw_get<C>(client: &Client<C>, key: &str, options: GetOptions) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
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

    let http_client = client.http_client().clone();
    let key = key.to_string();

    let result = first_ok(client.members().to_vec(), move |member| {
        let url = Url::parse_with_params(&build_url(member, &key), query_pairs.clone())
            .map_err(Error::from)
            .into_future();

        let uri = url.and_then(|url| {
            Uri::from_str(url.as_str())
                .map_err(Error::from)
                .into_future()
        });

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));

        let result = response.and_then(|response| {
            let status = response.status();
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| if status == StatusCode::Ok {
                match serde_json::from_slice::<KeyValueInfo>(body) {
                    Ok(key_space_info) => ok(key_space_info),
                    Err(error) => err(Error::Serialization(error)),
                }
            } else {
                match serde_json::from_slice::<ApiError>(body) {
                    Ok(error) => err(Error::Api(error)),
                    Err(error) => err(Error::Serialization(error)),
                }
            })
        });

        Box::new(result)
    });

    Box::new(result)
}

/// Handles all set operations.
fn raw_set<C>(client: &Client<C>, key: &str, options: SetOptions) -> FutureKeyValueInfo
where
    C: Clone + Connect,
{
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
        http_options.push(("prevExist".to_owned(), prev_exist.to_string()));
    }

    if let Some(ref conditions) = options.conditions {
        if conditions.is_empty() {
            return Box::new(Err(vec![Error::InvalidConditions]).into_future());
        }

        if let Some(ref modified_index) = conditions.modified_index {
            http_options.push(("prevIndex".to_owned(), modified_index.to_string()));
        }

        if let Some(ref value) = conditions.value {
            http_options.push(("prevValue".to_owned(), value.to_string()));
        }
    }

    let http_client = client.http_client().clone();
    let key = key.to_string();
    let create_in_order = options.create_in_order;

    let result = first_ok(client.members().to_vec(), move |member| {
        let mut serializer = Serializer::new(String::new());
        serializer.extend_pairs(http_options.clone());
        let body = serializer.finish();

        let url = build_url(member, &key);
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| if create_in_order {
            http_client.post(uri, body).map_err(Error::from)
        } else {
            http_client.put(uri, body).map_err(Error::from)
        });

        let result = response.and_then(|response| {
            let status = response.status();
            let body = response.body().concat2().map_err(Error::from);

            body.and_then(move |ref body| match status {
                StatusCode::Created | StatusCode::Ok => {
                    match serde_json::from_slice::<KeyValueInfo>(body) {
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
            })
        });

        Box::new(result)
    });

    Box::new(result)
}
