use std::thread::{sleep, spawn};
use std::time::Duration;

use etcd::kv::{self, Action, GetOptions, KeyValueInfo, WatchError, WatchOptions};
use etcd::{Error, Response};
use futures::future::{join_all, Future};
use futures::sync::oneshot::channel;

use crate::test::TestClient;

mod test;

#[test]
fn create() {
    let mut client = TestClient::new();

    let work = kv::create(&client, "/test/foo", "bar", Some(60)).and_then(|res| {
        let node = res.data.node;

        assert_eq!(res.data.action, Action::Create);
        assert_eq!(node.value.unwrap(), "bar");
        assert_eq!(node.ttl.unwrap(), 60);

        Ok(())
    });

    client.run(work);
}

#[test]
fn create_does_not_replace_existing_key() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", Some(60)).and_then(move |_| {
        kv::create(&inner_client, "/test/foo", "bar", Some(60)).then(|result| {
            match result {
                Ok(_) => panic!("expected EtcdError due to pre-existing key"),
                Err(errors) => {
                    for error in errors {
                        match error {
                            Error::Api(ref error) => {
                                assert_eq!(error.message, "Key already exists")
                            }
                            _ => panic!("expected EtcdError due to pre-existing key"),
                        }
                    }
                }
            }

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn create_in_order() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let requests =
        (1..4).map(move |_| Box::new(kv::create_in_order(&inner_client, "/test/foo", "bar", None)));

    let work = join_all(requests).and_then(|res: Vec<Response<KeyValueInfo>>| {
        let mut kvis: Vec<KeyValueInfo> = res.into_iter().map(|response| response.data).collect();
        kvis.sort_by_key(|ref kvi| kvi.node.modified_index);

        let keys: Vec<String> = kvis.into_iter().map(|kvi| kvi.node.key.unwrap()).collect();

        assert!(keys[0] < keys[1]);
        assert!(keys[1] < keys[2]);

        Ok(())
    });

    client.run(work);
}

#[test]
fn create_in_order_must_operate_on_a_directory() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::create_in_order(&inner_client, "/test/foo", "baz", None).then(|result| {
            assert!(result.is_err());

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn compare_and_delete() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |res| {
        let index = res.data.node.modified_index;

        kv::compare_and_delete(&inner_client, "/test/foo", Some("bar"), index).and_then(|res| {
            assert_eq!(res.data.action, Action::CompareAndDelete);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn compare_and_delete_only_index() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |res| {
        let index = res.data.node.modified_index;

        kv::compare_and_delete(&inner_client, "/test/foo", None, index).and_then(|res| {
            assert_eq!(res.data.action, Action::CompareAndDelete);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn compare_and_delete_only_value() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::compare_and_delete(&inner_client, "/test/foo", Some("bar"), None).and_then(|res| {
            assert_eq!(res.data.action, Action::CompareAndDelete);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn compare_and_delete_requires_conditions() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::compare_and_delete(&inner_client, "/test/foo", None, None).then(|result| match result {
            Ok(_) => panic!("expected Error::InvalidConditions"),
            Err(errors) => {
                if errors.len() == 1 {
                    match errors[0] {
                        Error::InvalidConditions => Ok(()),
                        _ => panic!("expected Error::InvalidConditions"),
                    }
                } else {
                    panic!("expected a single error: Error::InvalidConditions");
                }
            }
        })
    });

    client.run(work);
}

#[test]
fn test_compare_and_swap() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |res| {
        let index = res.data.node.modified_index;

        kv::compare_and_swap(
            &inner_client,
            "/test/foo",
            "baz",
            Some(100),
            Some("bar"),
            index,
        )
        .and_then(|res| {
            assert_eq!(res.data.action, Action::CompareAndSwap);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn compare_and_swap_only_index() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |res| {
        let index = res.data.node.modified_index;

        kv::compare_and_swap(&inner_client, "/test/foo", "baz", None, None, index).and_then(|res| {
            assert_eq!(res.data.action, Action::CompareAndSwap);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn compare_and_swap() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::compare_and_swap(&inner_client, "/test/foo", "baz", None, Some("bar"), None).and_then(
            |res| {
                assert_eq!(res.data.action, Action::CompareAndSwap);

                Ok(())
            },
        )
    });

    client.run(work);
}

#[test]
fn compare_and_swap_requires_conditions() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::compare_and_swap(&inner_client, "/test/foo", "baz", None, None, None).then(|result| {
            match result {
                Ok(_) => panic!("expected Error::InvalidConditions"),
                Err(errors) => {
                    if errors.len() == 1 {
                        match errors[0] {
                            Error::InvalidConditions => Ok(()),
                            _ => panic!("expected Error::InvalidConditions"),
                        }
                    } else {
                        panic!("expected a single error: Error::InvalidConditions");
                    }
                }
            }
        })
    });

    client.run(work);
}

#[test]
fn get() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", Some(60)).and_then(move |_| {
        kv::get(&inner_client, "/test/foo", GetOptions::default()).and_then(|res| {
            assert_eq!(res.data.action, Action::Get);

            let node = res.data.node;

            assert_eq!(node.value.unwrap(), "bar");
            assert_eq!(node.ttl.unwrap(), 60);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn get_non_recursive() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = join_all(vec![
        kv::set(&client, "/test/dir/baz", "blah", None),
        kv::set(&client, "/test/foo", "bar", None),
    ])
    .and_then(move |_| {
        kv::get(
            &inner_client,
            "/test",
            GetOptions {
                sort: true,
                ..Default::default()
            },
        )
        .and_then(|res| {
            let node = res.data.node;

            assert_eq!(node.dir.unwrap(), true);

            let nodes = node.nodes.unwrap();

            assert_eq!(nodes[0].clone().key.unwrap(), "/test/dir");
            assert_eq!(nodes[0].clone().dir.unwrap(), true);
            assert_eq!(nodes[1].clone().key.unwrap(), "/test/foo");
            assert_eq!(nodes[1].clone().value.unwrap(), "bar");

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn get_recursive() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::set(&client, "/test/dir/baz", "blah", None).and_then(move |_| {
        kv::get(
            &inner_client,
            "/test",
            GetOptions {
                recursive: true,
                sort: true,
                ..Default::default()
            },
        )
        .and_then(|res| {
            let nodes = res.data.node.nodes.unwrap();

            assert_eq!(
                nodes[0].clone().nodes.unwrap()[0].clone().value.unwrap(),
                "blah"
            );

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn get_root() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", Some(60)).and_then(move |_| {
        kv::get(&inner_client, "/", GetOptions::default()).and_then(|res| {
            assert_eq!(res.data.action, Action::Get);

            let node = res.data.node;

            assert!(node.created_index.is_none());
            assert!(node.modified_index.is_none());
            assert_eq!(node.nodes.unwrap().len(), 1);
            assert_eq!(node.dir.unwrap(), true);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn https() {
    let mut client = TestClient::https(true);

    let work = kv::set(&client, "/test/foo", "bar", Some(60));

    client.run(work);
}

#[test]
fn https_without_valid_client_certificate() {
    let mut client = TestClient::https(false);

    let work: Box<dyn Future<Item = (), Error = ()> + Send> =
        Box::new(kv::set(&client, "/test/foo", "bar", Some(60)).then(|res| {
            assert!(res.is_err());

            Ok(())
        }));

    client.run(work);
}

#[test]
fn set() {
    let mut client = TestClient::new();

    let work = kv::set(&client, "/test/foo", "baz", None).and_then(|res| {
        assert_eq!(res.data.action, Action::Set);

        let node = res.data.node;

        assert_eq!(node.value.unwrap(), "baz");
        assert!(node.ttl.is_none());

        Ok(())
    });

    client.run(work);
}

#[test]
fn set_and_refresh() {
    let mut client = TestClient::new();

    let work = kv::set(&client, "/test/foo", "baz", Some(30)).and_then(|res| {
        assert_eq!(res.data.action, Action::Set);

        let node = res.data.node;

        assert_eq!(node.value.unwrap(), "baz");
        assert!(node.ttl.is_some());

        Ok(())
    });

    client.run(work);

    let work = kv::refresh(&client, "/test/foo", 30).and_then(|res| {
        assert_eq!(res.data.action, Action::Update);

        let node = res.data.node;

        assert_eq!(node.value.unwrap(), "baz");
        assert!(node.ttl.is_some());

        Ok(())
    });

    client.run(work);
}

#[test]
fn set_dir() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::set_dir(&client, "/test", None).and_then(move |_| {
        kv::set_dir(&inner_client, "/test", None)
            .then(|result| match result {
                Ok(_) => panic!("set_dir should fail on an existing dir"),
                Err(_) => Ok(()),
            })
            .and_then(move |_| {
                kv::set(&inner_client, "/test/foo", "bar", None)
                    .and_then(move |_| kv::set_dir(&inner_client, "/test/foo", None))
            })
    });

    client.run(work);
}

#[test]
fn update() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::update(&inner_client, "/test/foo", "blah", Some(30)).and_then(|res| {
            assert_eq!(res.data.action, Action::Update);

            let node = res.data.node;

            assert_eq!(node.value.unwrap(), "blah");
            assert_eq!(node.ttl.unwrap(), 30);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn update_requires_existing_key() {
    let mut client = TestClient::no_destructor();

    let work = kv::update(&client, "/test/foo", "bar", None).then(|result| {
        match result {
            Err(ref errors) => match errors[0] {
                Error::Api(ref error) => assert_eq!(error.message, "Key not found"),
                _ => panic!("expected EtcdError due to missing key"),
            },
            _ => panic!("expected EtcdError due to missing key"),
        }

        let result: Result<(), ()> = Ok(());

        result
    });

    client.run(work);
}

#[test]
fn update_dir() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create_dir(&client, "/test", None).and_then(move |_| {
        kv::update_dir(&inner_client, "/test", Some(60)).and_then(|res| {
            assert_eq!(res.data.node.ttl.unwrap(), 60);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn update_dir_replaces_key() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::set(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::update_dir(&inner_client, "/test/foo", Some(60)).and_then(|res| {
            let node = res.data.node;

            assert_eq!(node.value.unwrap(), "");
            assert_eq!(node.ttl.unwrap(), 60);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn update_dir_requires_existing_dir() {
    let mut client = TestClient::no_destructor();

    let work: Box<dyn Future<Item = (), Error = ()> + Send> =
        Box::new(kv::update_dir(&client, "/test", None).then(|res| {
            assert!(res.is_err());

            Ok(())
        }));

    client.run(work);
}

#[test]
fn delete() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(move |_| {
        kv::delete(&inner_client, "/test/foo", false).and_then(|res| {
            assert_eq!(res.data.action, Action::Delete);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn create_dir() {
    let mut client = TestClient::new();

    let work = kv::create_dir(&client, "/test/dir", None).and_then(|res| {
        assert_eq!(res.data.action, Action::Create);

        let node = res.data.node;

        assert!(node.dir.is_some());
        assert!(node.value.is_none());

        Ok(())
    });

    client.run(work);
}

#[test]
fn delete_dir() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create_dir(&client, "/test/dir", None).and_then(move |_| {
        kv::delete_dir(&inner_client, "/test/dir").and_then(|res| {
            assert_eq!(res.data.action, Action::Delete);

            Ok(())
        })
    });

    client.run(work);
}

#[test]
fn watch() {
    let (tx, rx) = channel();

    let child = spawn(move || {
        let mut client = TestClient::no_destructor();
        let inner_client = client.clone();

        let work = rx.then(move |_| kv::set(&inner_client, "/test/foo", "baz", None));

        client.run(work);
    });

    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None)
        .map_err(|errors| WatchError::Other(errors))
        .and_then(move |_| {
            tx.send(()).unwrap();

            kv::watch(&inner_client, "/test/foo", WatchOptions::default()).and_then(|res| {
                assert_eq!(res.data.node.value.unwrap(), "baz");

                Ok(())
            })
        });

    client.run(work);

    child.join().ok().unwrap();
}

#[test]
fn watch_cancel() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work: Box<dyn Future<Item = (), Error = ()> + Send> = Box::new(
        kv::create(&client, "/test/foo", "bar", None)
            .map_err(|errors| WatchError::Other(errors))
            .and_then(move |_| {
                kv::watch(
                    &inner_client,
                    "/test/foo",
                    WatchOptions {
                        timeout: Some(Duration::from_millis(1)),
                        ..Default::default()
                    },
                )
            })
            .then(|res| match res {
                Ok(_) => panic!("expected WatchError::Timeout"),
                Err(WatchError::Timeout) => Ok(()),
                Err(_) => panic!("expected WatchError::Timeout"),
            }),
    );

    client.run(work);
}

#[test]
fn watch_index() {
    let mut client = TestClient::new();
    let inner_client = client.clone();

    let work = kv::set(&client, "/test/foo", "bar", None)
        .map_err(|errors| WatchError::Other(errors))
        .and_then(move |res| {
            let index = res.data.node.modified_index;

            kv::watch(
                &inner_client,
                "/test/foo",
                WatchOptions {
                    index: index,
                    ..Default::default()
                },
            )
            .and_then(move |res| {
                let node = res.data.node;

                assert_eq!(node.modified_index, index);
                assert_eq!(node.value.unwrap(), "bar");

                Ok(())
            })
        });

    client.run(work);
}

#[test]
fn watch_recursive() {
    let (tx, rx) = channel();

    let child = spawn(move || {
        let mut client = TestClient::no_destructor();
        let inner_client = client.clone();

        let work = rx.then(move |_| {
            let duration = Duration::from_millis(100);
            sleep(duration);
            kv::set(&inner_client, "/test/foo/bar", "baz", None)
        });

        client.run(work);
    });

    let mut client = TestClient::new();

    tx.send(()).unwrap();

    let work = kv::watch(
        &client,
        "/test",
        WatchOptions {
            recursive: true,
            timeout: Some(Duration::from_millis(1000)),
            ..Default::default()
        },
    )
    .and_then(|res| {
        let node = res.data.node;

        assert_eq!(node.key.unwrap(), "/test/foo/bar");
        assert_eq!(node.value.unwrap(), "baz");

        Ok(())
    });

    client.run(work);

    child.join().ok().unwrap();
}
