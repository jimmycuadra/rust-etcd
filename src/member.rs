use url::{ParseError, Url};

/// An etcd cluster member.
#[derive(Debug)]
pub struct Member {
    /// The URL of the member.
    pub endpoint: Url,
}

impl Member {
    pub fn new(url: &str) -> Result<Member, ParseError> {
        Ok(Member {
            endpoint: try!(Url::parse(url)),
        })
    }
}
