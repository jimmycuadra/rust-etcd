//! Types for the version endpoint.

/// Information about the versions of etcd running in a cluster.
///
/// This value is returned by `Client::versions`.
#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    /// The version of the entire etcd cluster.
    #[serde(rename="etcdcluster")]
    pub cluster_version: String,
    /// The version of the etcd server that returned this `VersionInfo`.
    #[serde(rename="etcdserver")]
    pub server_version: String,
}
