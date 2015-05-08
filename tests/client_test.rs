extern crate etcd;

use etcd::{Client, Error};

/// Wrapper around Client that automatically cleans up etcd after each test.
struct TestClient {
    c: Client,
}

impl TestClient {
    /// Creates a new client for a test.
    fn new() -> TestClient {
        TestClient {
            c: Client::new("http://etcd:2379").unwrap(),
        }
    }
}

impl Drop for TestClient {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        self.c.delete("/test", true);
    }
}

#[test]
fn create() {
    let client = TestClient::new();

    let response = match client.c.create("/test/foo", "bar", Some(60)) {
        Ok(response) => response,
        Err(error) => panic!("{:?}", error),
    };

    assert_eq!(response.action, "create".to_string());
    assert_eq!(response.node.value.unwrap(), "bar".to_string());
    assert_eq!(response.node.ttl.unwrap(), 60);
}

#[test]
fn create_fail() {
    let client = TestClient::new();

    client.c.create("/test/foo", "bar", Some(60)).ok().unwrap();

    match client.c.create("/test/foo", "bar", None).err().unwrap() {
        Error::Etcd(error) => assert_eq!(error.message, "Key already exists".to_string()),
        _ => panic!("expected EtcdError due to pre-existing key"),
    };
}

#[test]
fn get() {
    let client = TestClient::new();
    client.c.create("/test/foo", "bar", Some(60)).ok().unwrap();

    let response = client.c.get("/test/foo", false, false).ok().unwrap();

    assert_eq!(response.action, "get".to_string());
    assert_eq!(response.node.value.unwrap(), "bar".to_string());
    assert_eq!(response.node.ttl.unwrap(), 60);

}

#[test]
fn get_recursive() {
    let client = TestClient::new();

    client.c.set("/test/dir/baz", "blah", None).ok();
    client.c.set("/test/foo", "bar", None).ok();

    let mut response = client.c.get("/test", true, false).ok().unwrap();

    assert_eq!(response.node.dir.unwrap(), true);

    let mut nodes = response.node.nodes.unwrap();

    assert_eq!(nodes[0].clone().key.unwrap(), "/test/dir".to_string());
    assert_eq!(nodes[0].clone().dir.unwrap(), true);
    assert_eq!(nodes[1].clone().key.unwrap(), "/test/foo".to_string());
    assert_eq!(nodes[1].clone().value.unwrap(), "bar".to_string());

    response = client.c.get("/test", true, true).ok().unwrap();
    nodes = response.node.nodes.unwrap();

    assert_eq!(
        nodes[0].clone().nodes.unwrap()[0].clone().value.unwrap(),
        "blah".to_string()
    );
}

#[test]
fn set() {
    let client = TestClient::new();

    let response = client.c.set("/test/foo", "baz", None).ok().unwrap();

    assert_eq!(response.action, "set".to_string());
    assert_eq!(response.node.value.unwrap(), "baz".to_string());
    assert!(response.node.ttl.is_none());
}

#[test]
fn set_dir() {
    let client = TestClient::new();

    assert!(client.c.set_dir("/test", None).is_ok());
    assert!(client.c.set_dir("/test", None).is_err());

    client.c.set("/test/foo", "bar", None).ok().unwrap();

    assert!(client.c.set_dir("/test/foo", None).is_ok());
}

#[test]
fn update() {
    let client = TestClient::new();
    client.c.create("/test/foo", "bar", None).ok().unwrap();

    let response = client.c.update("/test/foo", "blah", Some(30)).ok().unwrap();

    assert_eq!(response.action, "update".to_string());
    assert_eq!(response.node.value.unwrap(), "blah".to_string());
    assert_eq!(response.node.ttl.unwrap(), 30);
}

#[test]
fn update_fail() {
    let client = TestClient::new();

    match client.c.update("/test/foo", "bar", None).err().unwrap() {
        Error::Etcd(error) => assert_eq!(error.message, "Key not found".to_string()),
        _ => panic!("expected EtcdError due to missing key"),
    };
}

#[test]
fn delete() {
    let client = TestClient::new();
    client.c.create("/test/foo", "bar", None).ok().unwrap();

    let response = client.c.delete("/test/foo", false).ok().unwrap();

    assert_eq!(response.action, "delete");
}

#[test]
fn create_dir() {
    let client = TestClient::new();

    let response = client.c.create_dir("/test/dir", None).ok().unwrap();

    assert_eq!(response.action, "create".to_string());
    assert!(response.node.dir.unwrap());
    assert!(response.node.value.is_none());
}

#[test]
fn delete_dir() {
    let client = TestClient::new();
    client.c.create_dir("/test/dir", None).ok().unwrap();

    let response = client.c.delete_dir("/test/dir").ok().unwrap();

    assert_eq!(response.action, "delete");
}

#[test]
fn leader_stats() {
    let client = TestClient::new();

    client.c.leader_stats().unwrap();
}
