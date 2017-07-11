use hyper::Uri;
use hyper::error::UriError;

/// An etcd cluster member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Member {
    /// The URI of the member.
    pub endpoint: Uri,
}

impl Member {
    pub fn new(uri: &str) -> Result<Member, UriError> {
        Ok(Member {
            endpoint: uri.parse()?,
        })
    }
}
