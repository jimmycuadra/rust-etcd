use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use etcd::{kv, Client};
use futures::Future;
use hyper::client::connect::Connect;
use hyper::client::{Client as Hyper, HttpConnector};
use hyper_tls::HttpsConnector;
use native_tls::{Certificate, Identity, TlsConnector};
use tokio::runtime::Runtime;

/// Wrapper around Client that automatically cleans up etcd after each test.
pub struct TestClient<C>
where
    C: Clone + Connect + Sync + 'static,
{
    c: Client<C>,
    run_destructor: bool,
    runtime: Runtime,
}

impl TestClient<HttpConnector> {
    /// Creates a new client for a test.
    #[allow(dead_code)]
    pub fn new() -> TestClient<HttpConnector> {
        TestClient {
            c: Client::new(&["http://etcd:2379"], None).unwrap(),
            run_destructor: true,
            runtime: Runtime::new().expect("failed to create Tokio runtime"),
        }
    }

    /// Creates a new client for a test that will not clean up the key space afterwards.
    #[allow(dead_code)]
    pub fn no_destructor() -> TestClient<HttpConnector> {
        TestClient {
            c: Client::new(&["http://etcd:2379"], None).unwrap(),
            run_destructor: false,
            runtime: Runtime::new().expect("failed to create Tokio runtime"),
        }
    }

    /// Creates a new HTTPS client for a test.
    #[allow(dead_code)]
    pub fn https(use_client_cert: bool) -> TestClient<HttpsConnector<HttpConnector>> {
        let mut ca_cert_file = File::open("/source/tests/ssl/ca.der").unwrap();
        let mut ca_cert_buffer = Vec::new();
        ca_cert_file.read_to_end(&mut ca_cert_buffer).unwrap();

        let mut builder = TlsConnector::builder();
        builder.add_root_certificate(Certificate::from_der(&ca_cert_buffer).unwrap());

        if use_client_cert {
            let mut pkcs12_file = File::open("/source/tests/ssl/client.p12").unwrap();
            let mut pkcs12_buffer = Vec::new();
            pkcs12_file.read_to_end(&mut pkcs12_buffer).unwrap();

            builder.identity(Identity::from_pkcs12(&pkcs12_buffer, "secret").unwrap());
        }

        let tls_connector = builder.build().unwrap();

        let mut http_connector = HttpConnector::new(1);
        http_connector.enforce_http(false);
        let https_connector = HttpsConnector::from((http_connector, tls_connector));

        let hyper = Hyper::builder().build(https_connector);

        TestClient {
            c: Client::custom(hyper, &["https://etcdsecure:2379"], None).unwrap(),
            run_destructor: true,
            runtime: Runtime::new().expect("failed to create Tokio runtime"),
        }
    }
}

impl<C> TestClient<C>
where
    C: Clone + Connect + Sync + 'static,
{
    #[allow(dead_code)]
    pub fn run<F, O, E>(&mut self, future: F)
    where
        F: Future<Item = O, Error = E> + Send + 'static,
        O: Send + 'static,
        E: Send + 'static,
    {
        let _ = self.runtime.block_on(future.map(|_| ()).map_err(|_| ()));
    }
}

impl<C> Drop for TestClient<C>
where
    C: Clone + Connect + Sync + 'static,
{
    fn drop(&mut self) {
        if self.run_destructor {
            let future = kv::delete(&self.c, "/test", true)
                .map(|_| ())
                .map_err(|_| ());

            let _ = self.runtime.block_on(future);
        }
    }
}

impl<C> Deref for TestClient<C>
where
    C: Clone + Connect + Sync + 'static,
{
    type Target = Client<C>;

    fn deref(&self) -> &Self::Target {
        &self.c
    }
}
