use std::fs::File;
use std::io::Read;
use std::ops::Deref;

use etcd::{Client, kv};
use futures::Future;
use hyper::client::{Client as Hyper, Connect, HttpConnector};
use hyper_tls::HttpsConnector;
use native_tls::{Certificate, Pkcs12, TlsConnector};
use tokio_core::reactor::Core;

/// Wrapper around Client that automatically cleans up etcd after each test.
pub struct TestClient<C>
where
    C: Clone + Connect,
{
    c: Client<C>,
    core: Core,
    run_destructor: bool,
}

impl TestClient<HttpConnector> {
    /// Creates a new client for a test.
    #[allow(dead_code)]
    pub fn new(core: Core) -> TestClient<HttpConnector> {
        let handle = core.handle();

        TestClient {
            c: Client::new(&handle, &["http://etcd:2379"], None).unwrap(),
            core: core,
            run_destructor: true,
        }
    }

    /// Creates a new client for a test that will not clean up the key space afterwards.
    #[allow(dead_code)]
    pub fn no_destructor(core: Core) -> TestClient<HttpConnector> {
        let handle = core.handle();

        TestClient {
            c: Client::new(&handle, &["http://etcd:2379"], None).unwrap(),
            core: core,
            run_destructor: false,
        }
    }

    /// Creates a new HTTPS client for a test.
    #[allow(dead_code)]
    pub fn https(core: Core, use_client_cert: bool) -> TestClient<HttpsConnector<HttpConnector>> {
        let mut ca_cert_file = File::open("/source/tests/ssl/ca.der").unwrap();
        let mut ca_cert_buffer = Vec::new();
        ca_cert_file.read_to_end(&mut ca_cert_buffer).unwrap();

        let mut builder = TlsConnector::builder().unwrap();
        builder
            .add_root_certificate(Certificate::from_der(&ca_cert_buffer).unwrap())
            .unwrap();

        if use_client_cert {
            let mut pkcs12_file = File::open("/source/tests/ssl/client.p12").unwrap();
            let mut pkcs12_buffer = Vec::new();
            pkcs12_file.read_to_end(&mut pkcs12_buffer).unwrap();

            builder
                .identity(Pkcs12::from_der(&pkcs12_buffer, "secret").unwrap())
                .unwrap();
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

impl<C> TestClient<C>
where
    C: Clone + Connect,
{
    #[allow(dead_code)]
    pub fn run<W, T, E>(&mut self, work: W) -> Result<T, E>
    where
        W: Future<Item = T, Error = E>,
    {
        self.core.run(work)
    }
}

impl<C> Drop for TestClient<C>
where
    C: Clone + Connect,
{
    fn drop(&mut self) {
        if self.run_destructor {
            let work = kv::delete(&self.c, "/test", true);
            self.core.run(work).unwrap();
        }
    }
}

impl<C> Deref for TestClient<C>
where
    C: Clone + Connect,
{
    type Target = Client<C>;

    fn deref(&self) -> &Self::Target {
        &self.c
    }
}
