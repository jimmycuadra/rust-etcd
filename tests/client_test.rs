extern crate etcd;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio_core;

use futures::{Future, Stream};
use tokio_core::reactor::Core;

use test::TestClient;

mod test;

#[test]
fn health() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = client.health().collect().and_then(|responses| {
        for response in responses {
            assert_eq!(response.data.health, "true");
        }

        Ok(())
    });

    assert!(client.run(work).is_ok());
}
#[test]
fn versions() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = client.versions().collect().and_then(|responses| {
        for response in responses {
            assert_eq!(response.data.cluster_version, "2.3.0");
            assert_eq!(response.data.server_version, "2.3.7");
        }

        Ok(())
    });

    assert!(client.run(work).is_ok());
}
