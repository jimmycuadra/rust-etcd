use std::collections::HashMap;

use url::Url;

#[derive(Debug)]
pub struct UrlWithQueryPairs<'a> {
    pub pairs: &'a HashMap<&'a str, String>,
    pub url: String,
}

impl<'a> UrlWithQueryPairs<'a> {
    pub fn parse(&self) -> Url {
        let mut url = Url::parse(&self.url[..]).unwrap();
        let pairs = self.pairs.iter().map(|(k, v)| (*k, &(*v)[..]));

        url.set_query_from_pairs(pairs);

        url
    }
}
