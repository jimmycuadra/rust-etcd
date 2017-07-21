extern crate etcd;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_timer;


use futures::future::Future;
use tokio_core::reactor::Core;
use etcd::members;

use test::TestClient;

mod test;

#[test]
fn list() {
    let core = Core::new().unwrap();
    let mut client = TestClient::no_destructor(core);

    let work = members::list(&client).and_then(|res| {
        let members = res.data;
        let member = &members[0];

        assert_eq!(member.name, "default");

        Ok(())
    });

    assert!(client.run(work).is_ok());
}
