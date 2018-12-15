use futures::{Future, Stream};

use crate::test::TestClient;

mod test;

#[test]
fn health() {
    let mut client = TestClient::no_destructor();

    let work = client.health().collect().and_then(|responses| {
        for response in responses {
            assert_eq!(response.data.health, "true");
        }

        Ok(())
    });

    client.run(work);
}
#[test]
fn versions() {
    let mut client = TestClient::no_destructor();

    let work = client.versions().collect().and_then(|responses| {
        for response in responses {
            assert_eq!(response.data.cluster_version, "2.3.0");
            assert_eq!(response.data.server_version, "2.3.8");
        }

        Ok(())
    });

    client.run(work);
}
