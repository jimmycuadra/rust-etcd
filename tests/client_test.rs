extern crate etcd;

use std::ops::Deref;
use std::thread::{sleep, spawn};
use std::time::Duration;

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
        self.delete("/test", true);
    }
}

impl Deref for TestClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.c
    }
}

#[test]
fn create() {
    let client = TestClient::new();

    let response = match client.create("/test/foo", "bar", Some(60)) {
        Ok(response) => response,
        Err(error) => panic!("{:?}", error),
    };

    assert_eq!(response.action, "create".to_string());
    assert_eq!(response.node.value.unwrap(), "bar".to_string());
    assert_eq!(response.node.ttl.unwrap(), 60);
}

#[test]
fn create_does_not_replace_existing_key() {
    let client = TestClient::new();

    client.create("/test/foo", "bar", Some(60)).ok().unwrap();

    match client.create("/test/foo", "bar", None).err().unwrap() {
        Error::Etcd(error) => assert_eq!(error.message, "Key already exists".to_string()),
        _ => panic!("expected EtcdError due to pre-existing key"),
    };
}

#[test]
fn create_in_order() {
    let client = TestClient::new();

    let keys: Vec<String> = (1..4).map(|ref _key| {
        client.create_in_order(
            "/test/foo",
            "bar",
            None,
        ).ok().unwrap().node.key.unwrap()
    }).collect();

    assert!(keys[0] < keys[1]);
    assert!(keys[1] < keys[2]);
}

#[test]
fn create_in_order_must_operate_on_a_directory() {
    let client = TestClient::new();

    client.create("/test/foo", "bar", None).ok().unwrap();

    assert!(client.create_in_order("/test/foo", "baz", None).is_err());
}

#[test]
fn compare_and_delete() {
    let client = TestClient::new();

    let modified_index = client.create(
        "/test/foo",
        "bar",
        None
    ).ok().unwrap().node.modified_index.unwrap();

    let response = client.compare_and_delete(
        "/test/foo",
        Some("bar"),
        Some(modified_index)
    ).ok().unwrap();

    assert_eq!(response.action, "compareAndDelete".to_string());
}

#[test]
fn compare_and_delete_only_index() {
    let client = TestClient::new();

    let modified_index = client.create(
        "/test/foo",
        "bar",
        None
    ).ok().unwrap().node.modified_index.unwrap();

    let response = client.compare_and_delete(
        "/test/foo",
        None,
        Some(modified_index)
    ).ok().unwrap();

    assert_eq!(response.action, "compareAndDelete".to_string());
}

#[test]
fn compare_and_delete_only_value() {
    let client = TestClient::new();
    client.create("/test/foo", "bar", None).ok().unwrap();

    let response = client.compare_and_delete(
        "/test/foo",
        Some("bar"),
        None,
    ).ok().unwrap();

    assert_eq!(response.action, "compareAndDelete".to_string());
}

#[test]
fn compare_and_delete_requires_conditions() {
    let client = TestClient::new();
    client.create("/test/foo", "bar", None).ok().unwrap();

    match client.compare_and_delete("/test/foo", None, None).err().unwrap() {
        Error::InvalidConditions(message) => assert_eq!(
            message,
            "Current value or modified index is required."
        ),
        _ => panic!("expected Error::InvalidConditions"),
    }
}

#[test]
fn compare_and_swap() {
    let client = TestClient::new();

    let modified_index = client.create(
        "/test/foo",
        "bar",
        None
    ).ok().unwrap().node.modified_index.unwrap();

    let response = client.compare_and_swap(
        "/test/foo",
        "baz",
        Some(100),
        Some("bar"),
        Some(modified_index)
    ).ok().unwrap();

    assert_eq!(response.action, "compareAndSwap".to_string());
}

#[test]
fn compare_and_swap_only_index() {
    let client = TestClient::new();

    let modified_index = client.create(
        "/test/foo",
        "bar",
        None
    ).ok().unwrap().node.modified_index.unwrap();

    let response = client.compare_and_swap(
        "/test/foo",
        "baz",
        None,
        None,
        Some(modified_index)
    ).ok().unwrap();

    assert_eq!(response.action, "compareAndSwap".to_string());
}

#[test]
fn compare_and_swap_only_value() {
    let client = TestClient::new();
    client.create("/test/foo", "bar", None).ok().unwrap();

    let response = client.compare_and_swap(
        "/test/foo",
        "bar",
        None,
        Some("bar"),
        None,
    ).ok().unwrap();

    assert_eq!(response.action, "compareAndSwap".to_string());
}

#[test]
fn compare_and_swap_requires_conditions() {
    let client = TestClient::new();
    client.create("/test/foo", "bar", None).ok().unwrap();

    match client.compare_and_swap("/test/foo", "bar", None, None, None).err().unwrap() {
        Error::InvalidConditions(message) => assert_eq!(
            message,
            "Current value or modified index is required."
        ),
        _ => panic!("expected Error::InvalidConditions"),
    }
}

#[test]
fn get() {
    let client = TestClient::new();
    client.create("/test/foo", "bar", Some(60)).ok().unwrap();

    let response = client.get("/test/foo", false, false, false).ok().unwrap();

    assert_eq!(response.action, "get".to_string());
    assert_eq!(response.node.value.unwrap(), "bar".to_string());
    assert_eq!(response.node.ttl.unwrap(), 60);

}

#[test]
fn get_non_recursive() {
    let client = TestClient::new();

    client.set("/test/dir/baz", "blah", None).ok();
    client.set("/test/foo", "bar", None).ok();

    let response = client.get("/test", true, false, false).ok().unwrap();

    assert_eq!(response.node.dir.unwrap(), true);

    let nodes = response.node.nodes.unwrap();

    assert_eq!(nodes[0].clone().key.unwrap(), "/test/dir".to_string());
    assert_eq!(nodes[0].clone().dir.unwrap(), true);
    assert_eq!(nodes[1].clone().key.unwrap(), "/test/foo".to_string());
    assert_eq!(nodes[1].clone().value.unwrap(), "bar".to_string());
}

#[test]
fn get_recursive() {
    let client = TestClient::new();

    client.set("/test/dir/baz", "blah", None).ok();

    let response = client.get("/test", true, true, false).ok().unwrap();
    let nodes = response.node.nodes.unwrap();

    assert_eq!(
        nodes[0].clone().nodes.unwrap()[0].clone().value.unwrap(),
        "blah".to_string()
    );
}

#[test]
fn leader_stats() {
    let client = TestClient::new();

    client.leader_stats().unwrap();
}

#[test]
fn set() {
    let client = TestClient::new();

    let response = client.set("/test/foo", "baz", None).ok().unwrap();

    assert_eq!(response.action, "set".to_string());
    assert_eq!(response.node.value.unwrap(), "baz".to_string());
    assert!(response.node.ttl.is_none());
}

#[test]
fn set_dir() {
    let client = TestClient::new();

    assert!(client.set_dir("/test", None).is_ok());
    assert!(client.set_dir("/test", None).is_err());

    client.set("/test/foo", "bar", None).ok().unwrap();

    assert!(client.set_dir("/test/foo", None).is_ok());
}

#[test]
fn update() {
    let client = TestClient::new();
    client.create("/test/foo", "bar", None).ok().unwrap();

    let response = client.update("/test/foo", "blah", Some(30)).ok().unwrap();

    assert_eq!(response.action, "update".to_string());
    assert_eq!(response.node.value.unwrap(), "blah".to_string());
    assert_eq!(response.node.ttl.unwrap(), 30);
}

#[test]
fn update_requires_existing_key() {
    let client = TestClient::new();

    match client.update("/test/foo", "bar", None).err().unwrap() {
        Error::Etcd(error) => assert_eq!(error.message, "Key not found".to_string()),
        _ => panic!("expected EtcdError due to missing key"),
    };
}

#[test]
fn update_dir() {
    let client = TestClient::new();

    client.create_dir("/test", None).ok().unwrap();

    let response = client.update_dir("/test", Some(60)).ok().unwrap();

    assert_eq!(response.node.ttl.unwrap(), 60);
}

#[test]
fn update_dir_replaces_key() {
    let client = TestClient::new();

    client.set("/test/foo", "bar", None).ok().unwrap();

    let response = client.update_dir("/test/foo", Some(60)).ok().unwrap();

    assert_eq!(response.node.value.unwrap(), "");
    assert_eq!(response.node.ttl.unwrap(), 60);
}

#[test]
fn update_dir_requires_existing_dir() {
    let client = TestClient::new();

    assert!(client.update_dir("/test", None).is_err());
}

#[test]
fn delete() {
    let client = TestClient::new();
    client.create("/test/foo", "bar", None).ok().unwrap();

    let response = client.delete("/test/foo", false).ok().unwrap();

    assert_eq!(response.action, "delete");
}

#[test]
fn create_dir() {
    let client = TestClient::new();

    let response = client.create_dir("/test/dir", None).ok().unwrap();

    assert_eq!(response.action, "create".to_string());
    assert!(response.node.dir.unwrap());
    assert!(response.node.value.is_none());
}

#[test]
fn delete_dir() {
    let client = TestClient::new();
    client.create_dir("/test/dir", None).ok().unwrap();

    let response = client.delete_dir("/test/dir").ok().unwrap();

    assert_eq!(response.action, "delete");
}

#[test]
fn watch() {
    let child = spawn(|| {
        let client = Client::new("http://etcd:2379").unwrap();

        sleep(Duration::from_millis(50));

        client.set("/test/foo", "baz", None).ok().unwrap();
    });

    let client = TestClient::new();

    client.create("/test/foo", "bar", None).ok().unwrap();

    let response = client.watch("/test/foo", None, false).ok().unwrap();

    assert_eq!(response.node.value.unwrap(), "baz".to_string());

    child.join().ok().unwrap();
}

#[test]
fn watch_index() {
    let client = TestClient::new();

    let index = client.set("/test/foo", "bar", None).ok().unwrap().node.modified_index.unwrap();

    let response = client.watch("/test/foo", Some(index), false).ok().unwrap();

    assert_eq!(response.node.modified_index.unwrap(), index);
    assert_eq!(response.node.value.unwrap(), "bar".to_string());
}

#[test]
fn watch_recursive() {
    let child = spawn(|| {
        let client = Client::new("http://etcd:2379").unwrap();

        sleep(Duration::from_millis(50));

        client.set("/test/foo/bar", "baz", None).ok().unwrap();
    });

    let client = TestClient::new();

    let response = client.watch("/test", None, true).ok().unwrap();

    assert_eq!(response.node.key.unwrap(), "/test/foo/bar".to_string());
    assert_eq!(response.node.value.unwrap(), "baz".to_string());

    child.join().ok().unwrap();
}

#[test]
fn version() {
    let client = TestClient::new();

    let version = client.version().ok().unwrap();

    assert_eq!(version.etcdcluster.unwrap(), "2.3.0".to_string());
    assert_eq!(version.etcdserver.unwrap(), "2.3.0".to_string());
}
