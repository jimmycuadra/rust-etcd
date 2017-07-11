//! Types for the primary key space operations.

use futures::Future;

use error::Error;

/// The future returned by all key space API calls.
///
/// On success, information about the result of the operation. On failure, an error for each cluster
/// member that failed.
pub type FutureKeySpaceInfo = Box<Future<Item = KeySpaceInfo, Error = Vec<Error>>>;

/// A FutureKeySpaceInfo for a single etcd cluster member.
pub type FutureSingleMemberKeySpaceInfo = Box<Future<Item = KeySpaceInfo, Error = Error>>;

/// Information about the result of a successful key space operation.
#[derive(Clone, Debug, Deserialize)]
pub struct KeySpaceInfo {
    /// The action that was taken, e.g. `get`, `set`.
    pub action: String,
    /// The etcd `Node` that was operated upon.
    pub node: Option<Node>,
    /// The previous state of the target node.
    #[serde(rename="prevNode")]
    pub prev_node: Option<Node>,
}

/// An etcd key-value pair or directory.
#[derive(Clone, Debug, Deserialize)]
pub struct Node {
    /// The new value of the etcd creation index.
    #[serde(rename="createdIndex")]
    pub created_index: Option<u64>,
    /// Whether or not the node is a directory.
    pub dir: Option<bool>,
    /// An ISO 8601 timestamp for when the key will expire.
    pub expiration: Option<String>,
    /// The name of the key.
    pub key: Option<String>,
    /// The new value of the etcd modification index.
    #[serde(rename="modifiedIndex")]
    pub modified_index: Option<u64>,
    /// Child nodes of a directory.
    pub nodes: Option<Vec<Node>>,
    /// The key's time to live in seconds.
    pub ttl: Option<i64>,
    /// The value of the key.
    pub value: Option<String>,
}
