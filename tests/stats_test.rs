extern crate etcd;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio_core;

use etcd::stats;
use futures::{Future, Stream};
use tokio_core::reactor::Core;

use test::TestClient;

mod test;

#[test]
fn leader_stats() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = stats::leader_stats(&client);

    assert!(client.run(work).is_ok());
}

#[test]
fn self_stats() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = stats::self_stats(&client).collect().and_then(|_| Ok(()));

    assert!(client.run(work).is_ok());
}

#[test]
fn store_stats() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = stats::store_stats(&client).collect().and_then(|_| Ok(()));

    assert!(client.run(work).is_ok());
}
