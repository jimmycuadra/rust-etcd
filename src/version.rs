//! Types for the version endpoint.

/// Versions of the etcd cluster and server.
#[derive(Debug, RustcDecodable)]
pub struct VersionInfo {
    /// The version of the etcd cluster.
    pub etcdcluster: Option<String>,
    /// The version of the etcd server.
    pub etcdserver: Option<String>,
}
