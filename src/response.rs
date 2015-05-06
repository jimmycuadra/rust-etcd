use std::collections::HashMap;
use error::Error;

/// Returned by `Client` API calls. A result containing an etcd `Response` or an `Error`.
pub type EtcdResult = Result<Response, Error>;

/// A successful response from etcd.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct Response {
    /// The action that was taken, e.g. `get`, `set`.
    pub action: String,
    /// The etcd `Node` that was operated upon.
    pub node: Node,
    /// The previous state of the target node.
    pub prevNode: Option<Node>,
}

/// An etcd key or directory.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct Node {
    /// The new value of the etcd creation index.
    pub createdIndex: Option<u64>,
    /// Whether or not the node is a directory.
    pub dir: Option<bool>,
    /// An ISO 8601 timestamp for when the key will expire.
    pub expiration: Option<String>,
    /// The name of the key.
    pub key: Option<String>,
    /// The new value of the etcd modification index.
    pub modifiedIndex: Option<u64>,
    /// Child nodes of a directory.
    pub nodes: Option<Vec<Node>>,
    /// The key's time to live in seconds.
    pub ttl: Option<i64>,
    /// The value of the key.
    pub value: Option<String>,
}

/// Statistics about an etcd cluster leader.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct LeaderStats {
    /// A unique identifier of a leader member.
    pub leader: String,
    /// Statistics for each peer in the cluster keyed by each peer's unique identifier.
    pub followers: HashMap<String, FollowerStats>,
}

/// Statistics on the health of a single follower.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct FollowerStats {
    /// Counts of Raft RPC request successes and failures to this follower.
    pub counts: Option<CountStats>,
    /// Latency statistics for this follower.
    pub latency: Option<LatencyStats>,
}

#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct CountStats {
    pub fail: Option<u64>,
    pub success: Option<u64>,
}

#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct LatencyStats {
    pub average: Option<f64>,
    pub current: Option<f64>,
    pub maximum: Option<f64>,
    pub minimum: Option<f64>,
    pub standardDeviation: Option<f64>,
}
