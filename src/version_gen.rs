/// Versions of the etcd cluster and server.
#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    /// The version of the etcd cluster.
    #[serde(rename="etcdcluster")]
    pub cluster_version: Option<String>,
    /// The version of the etcd server.
    #[serde(rename="etcdserver")]
    pub server_version: Option<String>,
}
