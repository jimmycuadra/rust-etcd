use std::mem::replace;
use std::vec::IntoIter;

use futures::{Async, Future, Poll};
use hyper::Uri;

/// Executes the given closure with each cluster member and short-circuit returns the first
/// successful result. If all members are exhausted without success, the final error is
/// returned.
pub fn first_ok<F, T>(endpoints: Vec<Uri>, callback: F) -> FirstOk<F, T>
where
    F: Fn(&Uri) -> T,
    T: Future,
{
    let max_errors = endpoints.len();

    FirstOk {
        callback,
        current_future: None,
        endpoints: endpoints.into_iter(),
        errors: Vec::with_capacity(max_errors),
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless polled"]
pub struct FirstOk<F, T>
where
    F: Fn(&Uri) -> T,
    T: Future,
{
    callback: F,
    current_future: Option<T>,
    endpoints: IntoIter<Uri>,
    errors: Vec<T::Error>,
}

impl<F, T> Future for FirstOk<F, T>
where
    F: Fn(&Uri) -> T,
    T: Future,
{
    type Item = T::Item;
    type Error = Vec<T::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(mut current_future) = self.current_future.take() {
            match current_future.poll() {
                Ok(Async::NotReady) => {
                    self.current_future = Some(current_future);

                    Ok(Async::NotReady)
                }
                Ok(Async::Ready(item)) => Ok(Async::Ready(item)),
                Err(error) => {
                    self.errors.push(error);

                    self.poll()
                }
            }
        } else {
            match self.endpoints.next() {
                Some(endpoint) => {
                    self.current_future = Some((self.callback)(&endpoint));

                    self.poll()
                }
                None => {
                    let errors = replace(&mut self.errors, vec![]);

                    Err(errors)
                }
            }
        }
    }
}
