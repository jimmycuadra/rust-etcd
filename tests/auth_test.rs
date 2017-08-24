extern crate etcd;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_timer;

use futures::future::Future;
use tokio_core::reactor::Core;
use etcd::auth;

use test::TestClient;

mod test;

#[test]
fn auth() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = auth::status(&client).and_then(|res| {
        assert_eq!(res.data, false);

        Ok(())
    });

    assert!(client.run(work).is_ok());
}
