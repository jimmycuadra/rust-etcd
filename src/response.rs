#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct Response {
    pub action: String,
    pub node: Node,
    pub prevNode: Option<Node>,
}

#[derive(Clone, Debug, RustcDecodable)]
#[allow(non_snake_case)]
pub struct Node {
    pub createdIndex: Option<u64>,
    pub dir: Option<bool>,
    pub expiration: Option<String>,
    pub key: Option<String>,
    pub modifiedIndex: Option<u64>,
    pub nodes: Option<Vec<Node>>,
    pub ttl: Option<i64>,
    pub value: Option<String>,
}
