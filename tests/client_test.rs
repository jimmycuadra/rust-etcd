extern crate etcd;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_timer;

use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::thread::spawn;
use std::time::Duration;

use futures::future::{Future, join_all};
use futures::stream::Stream;
use futures::sync::oneshot::channel;
use hyper::client::{Client as Hyper, Connect, HttpConnector};
use hyper_tls::HttpsConnector;
use native_tls::{Certificate, Pkcs12, TlsConnector};
use tokio_core::reactor::Core;

use etcd::{Client, Error};
use etcd::kv::{self, FutureKeySpaceInfo, KeySpaceInfo, WatchError};
use etcd::stats;

/// Wrapper around Client that automatically cleans up etcd after each test.
struct TestClient<C> where C: Clone + Connect {
    c: Client<C>,
    core: Core,
    run_destructor: bool,
}

impl TestClient<HttpConnector> {
    /// Creates a new client for a test.
    fn new(core: Core) -> TestClient<HttpConnector> {
        let handle = core.handle();

        TestClient {
            c: Client::new(&handle, &["http://etcd:2379"], None).unwrap(),
            core: core,
            run_destructor: true,
        }
    }

    /// Creates a new client for a test that will not clean up the key space afterwards.
    fn no_destructor(core: Core) -> TestClient<HttpConnector> {
        let handle = core.handle();

        TestClient {
            c: Client::new(&handle, &["http://etcd:2379"], None).unwrap(),
            core: core,
            run_destructor: false,
        }
    }

    /// Creates a new HTTPS client for a test.
    fn https(core: Core, use_client_cert: bool) -> TestClient<HttpsConnector<HttpConnector>> {
        let mut ca_cert_file = File::open("/source/tests/ssl/ca.der").unwrap();
        let mut ca_cert_buffer = Vec::new();
        ca_cert_file.read_to_end(&mut ca_cert_buffer).unwrap();

        let mut builder = TlsConnector::builder().unwrap();
        builder.add_root_certificate(Certificate::from_der(&ca_cert_buffer).unwrap()).unwrap();

        if use_client_cert {
            let mut pkcs12_file = File::open("/source/tests/ssl/client.p12").unwrap();
            let mut pkcs12_buffer = Vec::new();
            pkcs12_file.read_to_end(&mut pkcs12_buffer).unwrap();

            builder.identity(Pkcs12::from_der(&pkcs12_buffer, "secret").unwrap()).unwrap();
        }

        let tls_connector = builder.build().unwrap();

        let handle = core.handle();

        let mut http_connector = HttpConnector::new(1, &handle);
        http_connector.enforce_http(false);
        let https_connector = HttpsConnector::from((http_connector, tls_connector));

        let hyper = Hyper::configure().connector(https_connector).build(&handle);

        TestClient {
            c: Client::custom(hyper, &["https://etcdsecure:2379"], None).unwrap(),
            core,
            run_destructor: use_client_cert,
        }
    }
}

impl<C> TestClient<C> where C: Clone + Connect {
    pub fn run<W, T, E>(&mut self, work: W) -> Result<T, E>
    where W: Future<Item = T, Error = E>
    {
        self.core.run(work)
    }
}

impl<C> Drop for TestClient<C> where C: Clone + Connect {
    fn drop(&mut self) {
        if self.run_destructor {
            let work = kv::delete(&self.c, "/test", true);
            self.core.run(work).unwrap();
        }
    }
}

impl<C> Deref for TestClient<C> where C: Clone + Connect {
    type Target = Client<C>;

    fn deref(&self) -> &Self::Target {
        &self.c
    }
}

#[test]
fn create() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);

    let work = kv::create(&client, "/test/foo", "bar", Some(60)).and_then(|response| {
        let node = response.node.unwrap();

        assert_eq!(response.action, "create");
        assert_eq!(node.value.unwrap(), "bar");
        assert_eq!(node.ttl.unwrap(), 60);

        Ok(())
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn create_does_not_replace_existing_key() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", Some(60)).and_then(move |_| {
        kv::create(&inner_client, "/test/foo", "bar", Some(60)).then(|result| {
            match result {
                Ok(_) => panic!("expected EtcdError due to pre-existing key"),
                Err(errors) => {
                    for error in errors {
                        match error {
                            Error::Api(ref error) => assert_eq!(error.message, "Key already exists"),
                            _ => panic!("expected EtcdError due to pre-existing key"),
                        }
                    }
                }
            }

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn create_in_order() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);

    let requests: Vec<FutureKeySpaceInfo> = (1..4).map(|_| {
        kv::create_in_order( &client, "/test/foo", "bar", None)
    }).collect();

    let work = join_all(requests).and_then(|mut ksis: Vec<KeySpaceInfo>| {
        ksis.sort_by_key(|ksi| ksi.node.as_ref().unwrap().modified_index.unwrap());

        let keys: Vec<String> = ksis
            .into_iter()
            .map(|ksi| ksi.node.unwrap().key.unwrap())
            .collect();

        assert!(keys[0] < keys[1]);
        assert!(keys[1] < keys[2]);

        Ok(())
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn create_in_order_must_operate_on_a_directory() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|_| {
        kv::create_in_order(&inner_client, "/test/foo", "baz", None).then(|result| {
            assert!(result.is_err());

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn compare_and_delete() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|ksi| {
        let index = ksi.node.unwrap().modified_index.unwrap();

        kv::compare_and_delete(&inner_client, "/test/foo", Some("bar"), Some(index))
            .and_then(|ksi| {
                assert_eq!(ksi.action, "compareAndDelete");

                Ok(())
            })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn compare_and_delete_only_index() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|ksi| {
        let index = ksi.node.unwrap().modified_index.unwrap();

        kv::compare_and_delete(&inner_client, "/test/foo", None, Some(index)).and_then(|ksi| {
            assert_eq!(ksi.action, "compareAndDelete");

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn compare_and_delete_only_value() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|_| {
        kv::compare_and_delete(&inner_client, "/test/foo", Some("bar"), None).and_then(|ksi| {
            assert_eq!(ksi.action, "compareAndDelete");

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn compare_and_delete_requires_conditions() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|_| {
        kv::compare_and_delete(&inner_client, "/test/foo", None, None).then(|result| {
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

    assert!(client.run(work).is_ok());
}

#[test]
fn test_compare_and_swap() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|ksi| {
        let index = ksi.node.unwrap().modified_index.unwrap();

        kv::compare_and_swap(
            &inner_client,
            "/test/foo",
            "baz",
            Some(100),
            Some("bar"),
            Some(index),
        ).and_then(|ksi| {
            assert_eq!(ksi.action, "compareAndSwap");

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn compare_and_swap_only_index() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|ksi| {
        let index = ksi.node.unwrap().modified_index.unwrap();

        kv::compare_and_swap(
            &inner_client,
            "/test/foo",
            "baz",
            None,
            None,
            Some(index),
        ).and_then(|ksi| {
            assert_eq!(ksi.action, "compareAndSwap");

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn compare_and_swap() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|_| {
        kv::compare_and_swap(
            &inner_client,
            "/test/foo",
            "baz",
            None,
            Some("bar"),
            None,
        ).and_then(|ksi| {
            assert_eq!(ksi.action, "compareAndSwap");

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn compare_and_swap_requires_conditions() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None).and_then(|_| {
        kv::compare_and_swap(
            &inner_client,
            "/test/foo",
            "baz",
            None,
            None,
            None,
        ).then(|result| {
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

    assert!(client.run(work).is_ok());
}

#[test]
fn get() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", Some(60)).and_then(|_| {
        kv::get(&inner_client, "/test/foo", false, false, false).and_then(|ksi| {
            assert_eq!(ksi.action, "get");

            let node = ksi.node.unwrap();

            assert_eq!(node.value.unwrap(), "bar");
            assert_eq!(node.ttl.unwrap(), 60);

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn get_non_recursive() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = join_all(vec![
        kv::set(&client, "/test/dir/baz", "blah", None),
        kv::set(&client, "/test/foo", "bar", None)
    ]).and_then(|_| {
        kv::get(&inner_client, "/test", true, false, false).and_then(|ksi| {
            let node = ksi.node.unwrap();

            assert_eq!(node.dir.unwrap(), true);

            let nodes = node.nodes.unwrap();

            assert_eq!(nodes[0].clone().key.unwrap(), "/test/dir");
            assert_eq!(nodes[0].clone().dir.unwrap(), true);
            assert_eq!(nodes[1].clone().key.unwrap(), "/test/foo");
            assert_eq!(nodes[1].clone().value.unwrap(), "bar");

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn get_recursive() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::set(&client, "/test/dir/baz", "blah", None).and_then(|_| {
        kv::get(&inner_client, "/test", true, true, false).and_then(|ksi| {
            let nodes = ksi.node.unwrap().nodes.unwrap();

            assert_eq!(nodes[0].clone().nodes.unwrap()[0].clone().value.unwrap(), "blah");

            Ok(())
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn https() {
    let core = Core::new().unwrap();
    let mut client = TestClient::https(core, true);

    let work = kv::set(&client, "/test/foo", "bar", Some(60));

    assert!(client.run(work).is_ok());
}

#[test]
fn https_without_valid_client_certificate() {
    let core = Core::new().unwrap();
    let mut client = TestClient::https(core, false);

    let work = kv::set(&client, "/test/foo", "bar", Some(60));

    assert!(client.run(work).is_err());
}

#[test]
fn leader_stats() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = stats::leader_stats(&client);

    println!("{:?}", client.run(work));
    // assert!(client.run(work).is_ok());
}

#[test]
fn self_stats() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = stats::self_stats(&client).collect().and_then(|_|  Ok(()));

    assert!(client.run(work).is_ok());
}

#[test]
fn set() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);

    let work = kv::set(&client, "/test/foo", "baz", None).and_then(|ksi| {
        assert_eq!(ksi.action, "set");

        let node = ksi.node.unwrap();

        assert_eq!(node.value.unwrap(), "baz");
        assert!(node.ttl.is_none());

        Ok(())
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn set_dir() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::set_dir(&client, "/test", None).and_then(|_| {
        kv::set_dir(&inner_client, "/test", None).then(|result| {
            match result {
                Ok(_) => panic!("set_dir should fail on an existing dir"),
                Err(_) => Ok(()),
            }
        }).and_then(|_| {
            kv::set(&inner_client, "/test/foo", "bar", None).and_then(|_| {
                kv::set_dir(&inner_client, "/test/foo", None)
            })
        })
    });

    assert!(client.run(work).is_ok());
}

#[test]
fn store_stats() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = stats::store_stats(&client).collect().and_then(|_|  Ok(()));

    assert!(client.run(work).is_ok());
}

// #[test]
// fn update() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     client.create("/test/foo", "bar", None).ok().unwrap();

//     let response = client.update("/test/foo", "blah", Some(30)).ok().unwrap();
//     let node = response.node.unwrap();

//     assert_eq!(response.action, "update");
//     assert_eq!(node.value.unwrap(), "blah");
//     assert_eq!(node.ttl.unwrap(), 30);
// }

// #[test]
// fn update_requires_existing_key() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     for error in client.update("/test/foo", "bar", None).err().unwrap().iter() {
//         match error {
//             &Error::Api(ref error) => assert_eq!(error.message, "Key not found"),
//             _ => panic!("expected EtcdError due to missing key"),
//         }
//     };
// }

// #[test]
// fn update_dir() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     client.create_dir("/test", None).ok().unwrap();

//     let response = client.update_dir("/test", Some(60)).ok().unwrap();

//     assert_eq!(response.node.unwrap().ttl.unwrap(), 60);
// }

// #[test]
// fn update_dir_replaces_key() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     client.set("/test/foo", "bar", None).ok().unwrap();

//     let response = client.update_dir("/test/foo", Some(60)).ok().unwrap();
//     let node = response.node.unwrap();

//     assert_eq!(node.value.unwrap(), "");
//     assert_eq!(node.ttl.unwrap(), 60);
// }

// #[test]
// fn update_dir_requires_existing_dir() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     assert!(client.update_dir("/test", None).is_err());
// }

// #[test]
// fn delete() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     client.create("/test/foo", "bar", None).ok().unwrap();

//     let response = client.delete("/test/foo", false).ok().unwrap();

//     assert_eq!(response.action, "delete");
// }

// #[test]
// fn create_dir() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     let response = client.create_dir("/test/dir", None).ok().unwrap();
//     let node = response.node.unwrap();

//     assert_eq!(response.action, "create");
//     assert!(node.dir.is_some());
//     assert!(node.value.is_none());
// }

// #[test]
// fn delete_dir() {
//     let mut core = Core::new().unwrap();
//     let handle = core.handle();
//     let client = TestClient::new(&handle);

//     client.create_dir("/test/dir", None).ok().unwrap();

//     let response = client.delete_dir("/test/dir").ok().unwrap();

//     assert_eq!(response.action, "delete");
// }

#[test]
fn watch() {
    let (tx, rx) = channel();

    let child = spawn(move || {
        let core = Core::new().unwrap();
        let mut client = TestClient::no_destructor(core);
        let inner_client = client.clone();

        let work = rx.then(|_| kv::set(&inner_client, "/test/foo", "baz", None));

        assert!(client.run(work).is_ok());
    });

    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None)
        .map_err(|errors| WatchError::Other(errors))
        .and_then(move |_| {
            tx.send(()).unwrap();

            kv::watch(&inner_client, "/test/foo", None, false, None).and_then(|ksi| {
                assert_eq!(ksi.node.unwrap().value.unwrap(), "baz");

                Ok(())
            })
        });

    assert!(client.run(work).is_ok());

    child.join().ok().unwrap();
}

#[test]
fn watch_cancel() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::create(&client, "/test/foo", "bar", None)
        .map_err(|errors| WatchError::Other(errors))
        .and_then(move |_| {
            kv::watch(&inner_client, "/test/foo", None, false, Some(Duration::from_millis(1)))
        });

    match client.run(work) {
        Ok(_) => panic!("expected WatchError::Timeout"),
        Err(WatchError::Timeout) => {}
        Err(_) => panic!("expected WatchError::Timeout"),
    }
}

#[test]
fn watch_index() {
    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);
    let inner_client = client.clone();

    let work = kv::set(&client, "/test/foo", "bar", None)
        .map_err(|errors| WatchError::Other(errors))
        .and_then(move |ksi| {
            let index = ksi.node.unwrap().modified_index.unwrap();

            kv::watch(&inner_client, "/test/foo", Some(index), false, None).and_then(move |ksi| {
                let node = ksi.node.unwrap();

                assert_eq!(node.modified_index.unwrap(), index);
                assert_eq!(node.value.unwrap(), "bar");

                Ok(())
            })
        });

    assert!(client.run(work).is_ok());
}

#[test]
fn watch_recursive() {
    let (tx, rx) = channel();

    let child = spawn(move || {
        let core = Core::new().unwrap();
        let mut client = TestClient::no_destructor(core);
        let inner_client = client.clone();

        let work = rx.then(|_| kv::set(&inner_client, "/test/foo/bar", "baz", None));

        assert!(client.run(work).is_ok());
    });

    let core = Core::new().unwrap();
    let mut client = TestClient::new(core);

    tx.send(()).unwrap();

    let work = kv::watch(&client, "/test", None, true, None).and_then(|ksi| {
        let node = ksi.node.unwrap();

        assert_eq!(node.key.unwrap(), "/test/foo/bar");
        assert_eq!(node.value.unwrap(), "baz");

        Ok(())
    });

    assert!(client.run(work).is_ok());

    child.join().ok().unwrap();
}

#[test]
fn versions() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = client.versions().collect().and_then(|version_infos| {
        for version_info in version_infos {
            assert_eq!(version_info.cluster_version, "2.3.0");
            assert_eq!(version_info.server_version, "2.3.7");
        }

        Ok(())
    });

    assert!(client.run(work).is_ok());
}
