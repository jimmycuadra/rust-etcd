use std::io::Read;

use hyper::status::StatusCode;
use rustc_serialize::json;
use url::{ParseError, Url};
use url::form_urlencoded::serialize_owned;

use error::Error;
use http;
use response::Response;

#[derive(Debug)]
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

    pub fn create(&self, key: &str, value: &str, ttl: Option<u64>) -> Result<Response, Error> {
        let url = self.build_url(key);
        let mut options = vec![];

        options.push(("value".to_string(), value.to_string()));
        options.push(("prevExist".to_string(), "false".to_string()));

        if ttl.is_some() {
            options.push(("ttl".to_string(), format!("{}", ttl.unwrap())));
        }

        let body = serialize_owned(&options);

        match http::put(url, body) {
            Ok(mut response) => {
                let mut response_body = String::new();

                response.read_to_string(&mut response_body).unwrap();

                println!("{:?} - {:?} - {:?}", response.status, response.headers, response_body);

                match response.status {
                    StatusCode::Created => Ok(json::decode(&response_body).unwrap()),
                    _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
                }
            },
            Err(error) => Err(Error::Http(error)),
        }
    }

    pub fn delete(&self, key: &str, recursive: bool) -> Result<Response, Error> {
        let url = self.build_url(key);
        let mut options = vec![];

        options.push(("recursive".to_string(), format!("{}", recursive)));

        let body = serialize_owned(&options);

        match http::delete(url, body) {
            Ok(mut response) => {
                let mut response_body = String::new();

                response.read_to_string(&mut response_body).unwrap();

                println!("{:?} - {:?} - {:?}", response.status, response.headers, response_body);

                match response.status {
                    StatusCode::Ok => Ok(json::decode(&response_body).unwrap()),
                    _ => Err(Error::Etcd(json::decode(&response_body).unwrap())),
                }
            },
            Err(error) => Err(Error::Http(error)),
        }
    }

    // private

    fn build_url(&self, path: &str) -> String {
        format!("{}v2/keys{}", self.root_url, path)
    }

}

#[cfg(test)]
mod client_tests {
    use super::Client;
    use error::Error;

    #[test]
    fn lifecycle() {
        let client = Client::new("http://etcd:2379").unwrap();

        // Create a key

        let create_response = client.create("/foo", "bar", Some(100)).ok().unwrap();

        assert_eq!(create_response.action, "create".to_string());
        assert_eq!(create_response.node.value.unwrap(), "bar".to_string());
        assert_eq!(create_response.node.ttl.unwrap(), 100);

        // Creating a key fails if it already exists

        match client.create("/foo", "bar", None).err().unwrap() {
            Error::Etcd(error) => assert_eq!(error.message, "Key already exists".to_string()),
            _ => panic!("expected EtcdError due to pre-existing key"),
        };

        // Deleting a key

        let delete_response = client.delete("/foo", false).ok().unwrap();

        assert_eq!(delete_response.action, "delete");
    }
}
