//! etcd's key-value API.
//!
//! The term "node" in the documentation for this module refers to a key-value pair or a directory
//! of key-value pairs. For example, "/foo" is a key if it has a value, but it is a directory if
//! there other other key-value pairs "underneath" it, such as "/foo/bar".

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use futures::future::{Future, IntoFuture};
use futures::stream::Stream;
use hyper::client::connect::Connect;
use hyper::{StatusCode, Uri};
use serde_derive::{Deserialize, Serialize};
use serde_json;
use tokio::timer::Timeout;
use url::Url;

pub use crate::error::WatchError;

use crate::client::{Client, ClusterInfo, Response};
use crate::error::{ApiError, Error};
use crate::first_ok::first_ok;
use crate::options::{
    ComparisonConditions,
    DeleteOptions,
    GetOptions as InternalGetOptions,
    SetOptions,
};
use url::form_urlencoded::Serializer;

/// Information about the result of a successful key-value API operation.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct KeyValueInfo {
    /// The action that was taken, e.g. `get`, `set`.
    pub action: Action,
    /// The etcd `Node` that was operated upon.
    pub node: Node,
    /// The previous state of the target node.
    #[serde(rename = "prevNode")]
    pub prev_node: Option<Node>,
}

/// The type of action that was taken in response to a key value API request.
///
/// "Node" refers to the key or directory being acted upon.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

/// Options for customizing the behavior of `kv::get`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GetOptions {
    /// If true and the node is a directory, child nodes will be returned as well.
    pub recursive: bool,
    /// If true and the node is a directory, any child nodes returned will be sorted
    /// alphabetically.
    pub sort: bool,
    /// If true, the etcd node serving the response will synchronize with the quorum before
    /// returning the value.
    ///
    /// This is slower but avoids possibly stale data from being returned.
    pub strong_consistency: bool,
}

/// Options for customizing the behavior of `kv::watch`.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct WatchOptions {
    /// If given, the watch operation will return the first change at the index or greater,
    /// allowing you to watch for changes that happened in the past.
    pub index: Option<u64>,
    /// Whether or not to watch all child keys as well.
    pub recursive: bool,
    /// If given, the watch operation will time out if it's still waiting after the duration.
    pub timeout: Option<Duration>,
}

/// Deletes a node only if the given current value and/or current modified index match.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the node to delete.
/// * current_value: If given, the node must currently have this value for the operation to
/// succeed.
/// * current_modified_index: If given, the node must currently be at this modified index for the
/// operation to succeed.
///
/// # Errors
///
/// Fails if the conditions didn't match or if no conditions were given.
pub fn compare_and_delete<C>(
    client: &Client<C>,
    key: &str,
    current_value: Option<&str>,
    current_modified_index: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Updates a node only if the given current value and/or current modified index
/// match.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the node to update.
/// * value: The new value for the node.
/// * ttl: If given, the node will expire after this many seconds.
/// * current_value: If given, the node must currently have this value for the operation to
/// succeed.
/// * current_modified_index: If given, the node must currently be at this modified index for the
/// operation to succeed.
///
/// # Errors
///
/// Fails if the conditions didn't match or if no conditions were given.
pub fn compare_and_swap<C>(
    client: &Client<C>,
    key: &str,
    value: &str,
    ttl: Option<u64>,
    current_value: Option<&str>,
    current_modified_index: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Creates a new key-value pair.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the key-value pair to create.
/// * value: The new value for the node.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the key already exists.
pub fn create<C>(
    client: &Client<C>,
    key: &str,
    value: &str,
    ttl: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Creates a new empty directory.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the directory to create.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the key already exists.
pub fn create_dir<C>(
    client: &Client<C>,
    key: &str,
    ttl: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Creates a new key-value pair in a directory with a numeric key name larger than any of its
/// sibling key-value pairs.
///
/// For example, the first value created with this function under the directory "/foo" will have a
/// key name like "00000000000000000001" automatically generated. The second value created with
/// this function under the same directory will have a key name like "00000000000000000002".
///
/// This behavior is guaranteed by the server.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the directory to create a key-value pair in.
/// * value: The new value for the key-value pair.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the key already exists and is not a directory.
pub fn create_in_order<C>(
    client: &Client<C>,
    key: &str,
    value: &str,
    ttl: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Deletes a node.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the node to delete.
/// * recursive: If true, and the key is a directory, the directory and all child key-value
/// pairs and directories will be deleted as well.
///
/// # Errors
///
/// Fails if the key is a directory and `recursive` is `false`.
pub fn delete<C>(
    client: &Client<C>,
    key: &str,
    recursive: bool,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the node to delete.
///
/// # Errors
///
/// Fails if the directory is not empty.
pub fn delete_dir<C>(
    client: &Client<C>,
    key: &str,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Gets the value of a node.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the node to retrieve.
/// * options: Options to customize the behavior of the operation.
///
/// # Errors
///
/// Fails if the key doesn't exist.
pub fn get<C>(
    client: &Client<C>,
    key: &str,
    options: GetOptions,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
where
    C: Clone + Connect,
{
    raw_get(
        client,
        key,
        InternalGetOptions {
            recursive: options.recursive,
            sort: Some(options.sort),
            strong_consistency: options.strong_consistency,
            ..Default::default()
        },
    )
}

/// Sets the value of a key-value pair.
///
/// Any previous value and TTL will be replaced.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the key-value pair to set.
/// * value: The new value for the key-value pair.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the node is a directory.
pub fn set<C>(
    client: &Client<C>,
    key: &str,
    value: &str,
    ttl: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Refreshes the already set etcd key, bumping its TTL without triggering watcher updates.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the key-value pair to set.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the node does not exist.
pub fn refresh<C>(
    client: &Client<C>,
    key: &str,
    ttl: u64,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
where
    C: Clone + Connect,
{
    raw_set(
        client,
        key,
        SetOptions {
            ttl: Some(ttl),
            refresh: true,
            prev_exist: Some(true),
            ..Default::default()
        },
    )
}

/// Sets the key to an empty directory.
///
/// An existing key-value pair will be replaced, but an existing directory will not.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the directory to set.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the node is an existing directory.
pub fn set_dir<C>(
    client: &Client<C>,
    key: &str,
    ttl: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Updates an existing key-value pair.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the key-value pair to update.
/// * value: The new value for the key-value pair.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the key does not exist.
pub fn update<C>(
    client: &Client<C>,
    key: &str,
    value: &str,
    ttl: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Updates a directory.
///
/// If the directory already existed, only the TTL is updated. If the key was a key-value pair, its
/// value is removed and its TTL is updated.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the node to update.
/// * ttl: If given, the node will expire after this many seconds.
///
/// # Errors
///
/// Fails if the node does not exist.
pub fn update_dir<C>(
    client: &Client<C>,
    key: &str,
    ttl: Option<u64>,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

/// Watches a node for changes and returns the new value as soon as a change takes place.
///
/// # Parameters
///
/// * client: A `Client` to use to make the API call.
/// * key: The name of the node to watch.
/// * options: Options to customize the behavior of the operation.
///
/// # Errors
///
/// Fails if `options.index` is too old and has been flushed out of etcd's internal store of the
/// most recent change events. In this case, the key should be queried for its latest
/// "modified index" value and that should be used as the new `options.index` on a subsequent
/// `watch`.
///
/// Fails if a timeout is specified and the duration lapses without a response from the etcd
/// cluster.
pub fn watch<C>(
    client: &Client<C>,
    key: &str,
    options: WatchOptions,
) -> Box<dyn Future<Item = Response<KeyValueInfo>, Error = WatchError> + Send>
where
    C: Clone + Connect,
{
    let work = raw_get(
        client,
        key,
        InternalGetOptions {
            recursive: options.recursive,
            wait_index: options.index,
            wait: true,
            ..Default::default()
        },
    )
    .map_err(|errors| WatchError::Other(errors));

    if let Some(duration) = options.timeout {
        Box::new(
            Timeout::new(work, duration).map_err(|e| match e.into_inner() {
                Some(we) => we,
                None => WatchError::Timeout,
            }),
        )
    } else {
        Box::new(work)
    }
}

/// Constructs the full URL for an API call.
fn build_url(endpoint: &Uri, path: &str) -> String {
    format!("{}v2/keys{}", endpoint, path)
}

/// Handles all delete operations.
fn raw_delete<C>(
    client: &Client<C>,
    key: &str,
    options: DeleteOptions<'_>,
) -> Box<dyn Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send>
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

    let result = first_ok(client.endpoints().to_vec(), move |endpoint| {
        let url = Url::parse_with_params(&build_url(endpoint, &key), query_pairs.clone())
            .map_err(Error::from)
            .into_future();

        let uri = url.and_then(|url| {
            Uri::from_str(url.as_str())
                .map_err(Error::from)
                .into_future()
        });

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.delete(uri).map_err(Error::from));

        response.and_then(move |response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |ref body| {
                if status == StatusCode::OK {
                    match serde_json::from_slice::<KeyValueInfo>(body) {
                        Ok(data) => Ok(Response { data, cluster_info }),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                } else {
                    match serde_json::from_slice::<ApiError>(body) {
                        Ok(error) => Err(Error::Api(error)),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                }
            })
        })
    });

    Box::new(result)
}

/// Handles all get operations.
fn raw_get<C>(
    client: &Client<C>,
    key: &str,
    options: InternalGetOptions,
) -> impl Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send
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

    first_ok(client.endpoints().to_vec(), move |endpoint| {
        let url = Url::parse_with_params(&build_url(endpoint, &key), query_pairs.clone())
            .map_err(Error::from)
            .into_future();

        let uri = url.and_then(|url| {
            Uri::from_str(url.as_str())
                .map_err(Error::from)
                .into_future()
        });

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| http_client.get(uri).map_err(Error::from));

        response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |ref body| {
                if status == StatusCode::OK {
                    match serde_json::from_slice::<KeyValueInfo>(body) {
                        Ok(data) => Ok(Response { data, cluster_info }),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                } else {
                    match serde_json::from_slice::<ApiError>(body) {
                        Ok(error) => Err(Error::Api(error)),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                }
            })
        })
    })
}

/// Handles all set operations.
fn raw_set<C>(
    client: &Client<C>,
    key: &str,
    options: SetOptions<'_>,
) -> Box<dyn Future<Item = Response<KeyValueInfo>, Error = Vec<Error>> + Send>
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

    let prev_exist = match options.prev_exist {
        Some(prev_exist) => prev_exist,
        None => false,
    };

    // If we are calling refresh, we should also ensure we are setting prevExist.
    if prev_exist || options.refresh {
        let prev_exist = prev_exist || options.refresh;
        http_options.push(("prevExist".to_owned(), prev_exist.to_string()));
    }

    if options.refresh {
        http_options.push(("refresh".to_owned(), "true".to_owned()));
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

    let result = first_ok(client.endpoints().to_vec(), move |endpoint| {
        let mut serializer = Serializer::new(String::new());
        serializer.extend_pairs(http_options.clone());
        let body = serializer.finish();

        let url = build_url(endpoint, &key);
        let uri = Uri::from_str(url.as_str())
            .map_err(Error::from)
            .into_future();

        let http_client = http_client.clone();

        let response = uri.and_then(move |uri| {
            if create_in_order {
                http_client.post(uri, body).map_err(Error::from)
            } else {
                http_client.put(uri, body).map_err(Error::from)
            }
        });

        response.and_then(|response| {
            let status = response.status();
            let cluster_info = ClusterInfo::from(response.headers());
            let body = response.into_body().concat2().map_err(Error::from);

            body.and_then(move |ref body| match status {
                StatusCode::CREATED | StatusCode::OK => {
                    match serde_json::from_slice::<KeyValueInfo>(body) {
                        Ok(data) => Ok(Response { data, cluster_info }),
                        Err(error) => Err(Error::Serialization(error)),
                    }
                }
                _ => match serde_json::from_slice::<ApiError>(body) {
                    Ok(error) => Err(Error::Api(error)),
                    Err(error) => Err(Error::Serialization(error)),
                },
            })
        })
    });

    Box::new(result)
}
