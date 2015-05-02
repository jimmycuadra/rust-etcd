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
