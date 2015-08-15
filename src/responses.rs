use std::collections::HashMap;
use error::Error;

/// Returned by key space API calls.
pub type KeySpaceResult = Result<KeySpaceInfo, Error>;

/// Information about the result of a successful key space operation.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct KeySpaceInfo {
    /// The action that was taken, e.g. `get`, `set`.
    pub action: String,
    /// The etcd `Node` that was operated upon.
    pub node: Node,
    /// The previous state of the target node.
    pub prevNode: Option<Node>,
}

/// An etcd key-value pair or directory.
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

/// Versions of the etcd cluster and server.
#[derive(Debug, RustcDecodable)]
pub struct VersionInfo {
    /// The version of the etcd cluster.
    pub etcdcluster: Option<String>,
    /// The version of the etcd server.
    pub etcdserver: Option<String>,
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

/// Statistics on the health of a single etcd follower node.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct FollowerStats {
    /// Counts of Raft RPC request successes and failures to this follower.
    pub counts: Option<CountStats>,
    /// Latency statistics for this follower.
    pub latency: Option<LatencyStats>,
}

/// Statistics about the number of successful and failed Raft RPC requests to an etcd node.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct CountStats {
    /// The number of times an RPC request to the node failed.
    pub fail: Option<u64>,
    /// The number of times an RPC request to the node succeeded.
    pub success: Option<u64>,
}

/// Statistics about the network latency to an etcd node.
#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct LatencyStats {
    /// The average observed latency to the node, in seconds.
    pub average: Option<f64>,
    /// The current observed latency to the node, in seconds.
    pub current: Option<f64>,
    /// The maximum observed latency to the node, in seconds.
    pub maximum: Option<f64>,
    /// The minimum observed latency to the node, in seconds.
    pub minimum: Option<f64>,
    /// The standard deviation of latency to the node.
    pub standardDeviation: Option<f64>,
}
