use etcd::members;
use futures::future::Future;
use tokio_core::reactor::Core;

use crate::test::TestClient;

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
