use etcd::stats;
use futures::{Future, Stream};

use crate::test::TestClient;

mod test;

#[test]
fn leader_stats() {
    let mut client = TestClient::no_destructor();

    let work = stats::leader_stats(&client);

    client.run(work);
}

#[test]
fn self_stats() {
    let mut client = TestClient::no_destructor();

    let work = stats::self_stats(&client).collect().and_then(|_| Ok(()));

    client.run(work);
}

#[test]
fn store_stats() {
    let mut client = TestClient::no_destructor();

    let work = stats::store_stats(&client).collect().and_then(|_| Ok(()));

    client.run(work);
}
