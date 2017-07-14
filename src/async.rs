use std::mem::replace;
use std::vec::IntoIter;

use futures::{Async, Future, Poll};

use error::Error;
use kv::{FutureSingleMemberKeySpaceInfo, KeySpaceInfo};
use member::Member;

/// Executes the given closure with each cluster member and short-circuit returns the first
/// successful result. If all members are exhausted without success, the final error is
/// returned.
pub fn first_ok<F>(members: Vec<Member>, callback: F) -> FirstOk<F>
where
    F: Fn(&Member) -> FutureSingleMemberKeySpaceInfo,
{
    FirstOk {
        callback,
        current_future: None,
        errors: Vec::with_capacity(members.len()),
        members: members.into_iter(),
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct FirstOk<F>
where
    F: Fn(&Member) -> FutureSingleMemberKeySpaceInfo,
{
    callback: F,
    current_future: Option<FutureSingleMemberKeySpaceInfo>,
    errors: Vec<Error>,
    members: IntoIter<Member>,
}

impl<F> Future for FirstOk<F>
where
    F: Fn(&Member) -> FutureSingleMemberKeySpaceInfo,
{
    type Item = KeySpaceInfo;
    type Error = Vec<Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(mut current_future) = self.current_future.take() {
            match current_future.poll() {
                Ok(Async::NotReady) => {
                    self.current_future = Some(current_future);

                    Ok(Async::NotReady)
                }
                Ok(Async::Ready(key_space_info)) => Ok(Async::Ready(key_space_info)),
                Err(error) => {
                    self.errors.push(error);

                    self.poll()
                }
            }
        } else {
            match self.members.next() {
                Some(member) => {
                    self.current_future = Some((self.callback)(&member));

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
