extern crate etcd;

use etcd::{Client, Error};

#[test]
fn lifecycle() {
    let client = Client::new("http://etcd:2379").unwrap();

    // Create a key

    let create_response = client.create("/foo", "bar", Some(100)).ok().unwrap();

    assert_eq!(create_response.action, "create".to_string());
    assert_eq!(create_response.node.value.unwrap(), "bar".to_string());
    assert_eq!(create_response.node.ttl.unwrap(), 100);

    // Getting a key

    let get_response = client.get("/foo", false, false).ok().unwrap();

    assert_eq!(get_response.action, "get".to_string());
    assert_eq!(get_response.node.value.unwrap(), "bar".to_string());
    assert_eq!(get_response.node.ttl.unwrap(), 100);

    // Creating a key fails if it already exists

    match client.create("/foo", "bar", None).err().unwrap() {
        Error::Etcd(error) => assert_eq!(error.message, "Key already exists".to_string()),
        _ => panic!("expected EtcdError due to pre-existing key"),
    };

    // Deleting a key

    let delete_response = client.delete("/foo", false).ok().unwrap();

    assert_eq!(delete_response.action, "delete");
}
