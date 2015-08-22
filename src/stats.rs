//! Types for statistics operations.

use std::collections::HashMap;

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

