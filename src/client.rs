use url::{ParseError, Url};

pub struct Response {
    pub node: Node,
}

pub struct Node {
    pub dir: Option<bool>,
    pub value: Option<String>,
}

pub struct EtcdError;

pub struct Client {
    root_url: String,
}

impl Client {
    pub fn new(root_url: &str) -> Result<Client, ParseError> {
        let url = try!(Url::parse(root_url));
        let client = Client {
            root_url: format!("{}", url.serialize()),
        };

        Ok(client)
    }

    pub fn default() -> Client {
        Client {
            root_url: "http://127.0.0.1:2379/".to_string(),
        }
    }

    pub fn get(&self, key: &str) -> Result<Response, EtcdError> {
        Err(EtcdError)
    }

    pub fn mk(&self, key: &str, value: &str, ttl: Option<u64>) -> Result<Response, EtcdError> {
        Err(EtcdError)
    }

    pub fn mkdir(&self, key: &str, ttl: Option<u64>) -> Result<Response, EtcdError> {
        Err(EtcdError)
    }

    pub fn set(
        &self,
        key: &str,
        value: &str,
        ttl: Option<u64>,
        prev_value: Option<&str>,
        prev_index: Option<u64>
    ) -> Result<Response, EtcdError> {
        Err(EtcdError)
    }
}

mod mk_tests {
    use super::Client;

    #[test]
    fn mk_key() {
        let client = Client::default();

        assert!(client.mk("/foo", "bar", None).is_ok());
        assert_eq!(client.get("/foo").ok().unwrap().node.value.unwrap(), "bar");
    }

    #[test]
    fn mk_key_failure() {
        let client = Client::default();

        assert!(client.mk("/foo", "bar", None).is_ok());
        assert!(client.mk("/foo", "bar", None).is_err());
    }
}

mod mkdir_tests {
    use super::Client;

    #[test]
    fn mkdir() {
        let client = Client::default();

        assert!(client.mkdir("/foo", None).ok().unwrap().node.dir.unwrap());
    }

    #[test]
    fn mkdir_failure() {
        let client = Client::default();

        assert!(client.mkdir("/foo", None).is_ok());
        assert!(client.mkdir("/foo", None).is_err());
    }
}

mod get_tests {
    use super::Client;

    #[test]
    fn set_and_get_key() {
        let client = Client::default();

        assert!(client.set("/foo", "bar", None, None, None).is_ok());
        assert_eq!(client.get("/foo").ok().unwrap().node.value.unwrap(), "bar");
    }
}
