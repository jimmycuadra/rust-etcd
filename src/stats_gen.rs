use std::collections::HashMap;

/// Statistics about an etcd cluster leader.
#[derive(Clone, Debug, Deserialize)]
pub struct LeaderStats {
    /// A unique identifier of a leader member.
    pub leader: String,
    /// Statistics for each peer in the cluster keyed by each peer's unique identifier.
    pub followers: HashMap<String, FollowerStats>,
}

/// Statistics about the health of a single etcd follower node.
#[derive(Clone, Debug, Deserialize)]
pub struct FollowerStats {
    /// Counts of Raft RPC request successes and failures to this follower.
    pub counts: CountStats,
    /// Latency statistics for this follower.
    pub latency: LatencyStats,
}

/// Statistics about the number of successful and failed Raft RPC requests to an etcd node.
#[derive(Clone, Debug, Deserialize)]
pub struct CountStats {
    /// The number of times an RPC request to the node failed.
    pub fail: u64,
    /// The number of times an RPC request to the node succeeded.
    pub success: u64,
}

/// Statistics about the network latency to an etcd node.
#[derive(Clone, Debug, Deserialize)]
pub struct LatencyStats {
    /// The average observed latency to the node, in seconds.
    pub average: f64,
    /// The current observed latency to the node, in seconds.
    pub current: f64,
    /// The maximum observed latency to the node, in seconds.
    pub maximum: f64,
    /// The minimum observed latency to the node, in seconds.
    pub minimum: f64,
    /// The standard deviation of latency to the node.
    #[serde(rename="standardDeviation")]
    pub standard_deviation: f64,
}

/// Statistics about an etcd cluster member.
#[derive(Clone, Debug, Deserialize)]
pub struct SelfStats {
    /// The unique Raft ID of the member.
    pub id: String,
    /// The member's name.
    pub name: String,
    /// A small amount of information about the leader of the cluster.
    #[serde(rename="leaderInfo")]
    pub leader_info: LeaderInfo,
    /// The number of received requests.
    #[serde(rename="recvAppendRequestCnt")]
    pub received_append_request_count: u64,
    /// The bandwidth rate of received requests.
    #[serde(rename="recvBandwidthRate")]
    pub received_bandwidth_rate: Option<f64>,
    #[serde(rename="recvPkgRate")]
    /// The package rate of received requests.
    pub received_package_rate: Option<f64>,
    /// The number of sent requests.
    #[serde(rename="sendAppendRequestCnt")]
    pub sent_append_request_count: u64,
    /// The bandwidth rate of sent requests.
    #[serde(rename="sendBandwidthRate")]
    pub sent_bandwidth_rate: Option<f64>,
    /// The package rate of sent requests.
    #[serde(rename="sendPkgRate")]
    pub sent_package_rate: Option<f64>,
    /// The time the member started.
    #[serde(rename="startTime")]
    pub start_time: String,
    /// The Raft state of the member.
    pub state: String,
}

/// A small amount of information about the leader of the cluster.
#[derive(Clone, Debug, Deserialize)]
pub struct LeaderInfo {
    /// The unique Raft ID of the leader.
    #[serde(rename="leader")]
    pub id: String,
    /// The time the leader started.
    #[serde(rename="startTime")]
    pub start_time: String,
    /// The amount of time the leader has been up.
    pub uptime: String,
}

/// Statistics about the operations handled by an etcd member.
#[derive(Clone, Debug, Deserialize)]
pub struct StoreStats {
    /// The number of failed compare and delete operations.
    #[serde(rename="compareAndDeleteFail")]
    pub compare_and_delete_fail: u64,
    /// The number of successful compare and delete operations.
    #[serde(rename="compareAndDeleteSuccess")]
    pub compare_and_delete_success: u64,
    /// The number of failed compare and swap operations.
    #[serde(rename="compareAndSwapFail")]
    pub compare_and_swap_fail: u64,
    /// The number of successful compare and swap operations.
    #[serde(rename="compareAndSwapSuccess")]
    pub compare_and_swap_success: u64,
    /// The number of failed create operations.
    #[serde(rename="createFail")]
    pub create_fail: u64,
    /// The number of successful create operations.
    #[serde(rename="createSuccess")]
    pub create_success: u64,
    /// The number of failed delete operations.
    #[serde(rename="deleteFail")]
    pub delete_fail: u64,
    /// The number of successful delete operations.
    #[serde(rename="deleteSuccess")]
    pub delete_success: u64,
    /// The number of expire operations.
    #[serde(rename="expireCount")]
    pub expire_count: u64,
    /// The number of failed get operations.
    #[serde(rename="getsFail")]
    pub get_fail: u64,
    /// The number of successful get operations.
    #[serde(rename="getsSuccess")]
    pub get_success: u64,
    /// The number of failed set operations.
    #[serde(rename="setsFail")]
    pub set_fail: u64,
    /// The number of successful set operations.
    #[serde(rename="setsSuccess")]
    pub set_success: u64,
    /// The number of failed update operations.
    #[serde(rename="updateFail")]
    pub update_fail: u64,
    /// The number of successful update operations.
    #[serde(rename="updateSuccess")]
    pub update_success: u64,
    /// The number of watchers.
    pub watchers: u64,
}
