/// Possible conditions for "compare and delete" and "compare and swap" operations.
#[derive(Debug)]
pub struct ComparisonConditions<'a> {
    /// The etcd modified index the key must have before the operation is performed.
    pub modified_index: Option<u64>,
    /// The value the key must have before the operation is performed.
    pub value: Option<&'a str>,
}

impl<'a> ComparisonConditions<'a> {
    /// Returns a boolean indicating whether or not both conditions are unset.
    pub fn is_empty(&self) -> bool {
        self.modified_index.is_none() && self.value.is_none()
    }
}

/// Controls the various different ways a delete operation can be performed.
#[derive(Debug, Default)]
pub struct DeleteOptions<'a> {
    /// Conditions used for "compare and delete" operations.
    pub conditions: Option<ComparisonConditions<'a>>,
    /// Whether or not the key to be deleted is a directory.
    pub dir: Option<bool>,
    /// Whether or not keys within a directory should be deleted recursively.
    pub recursive: Option<bool>,
}

/// Controls the various different ways a get operation can be performed.
#[derive(Debug, Default)]
pub struct GetOptions {
    /// Whether or not to use read linearization to avoid stale data.
    pub strong_consistency: bool,
    /// Whether or not keys within a directory should be included in the response.
    pub recursive: bool,
    /// Whether or not directory contents will be sorted within the response.
    pub sort: Option<bool>,
    /// Whether or not to wait for a change.
    pub wait: bool,
    /// The etcd index to use as a lower bound when watching a key.
    pub wait_index: Option<u64>,
}

/// Controls the various different ways a create, update, or set operation can be performed.
#[derive(Debug, Default)]
pub struct SetOptions<'a> {
    /// Conditions used for "compare and swap" operations.
    pub conditions: Option<ComparisonConditions<'a>>,
    /// Whether or not to use the "create in order" API.
    pub create_in_order: bool,
    /// Whether or not the key being operated on is or should be a directory.
    pub dir: Option<bool>,
    /// Whether or not the key being operated on must already exist.
    pub prev_exist: Option<bool>,
    /// Time to live in seconds.
    pub ttl: Option<u64>,
    /// New value for the key.
    pub value: Option<&'a str>,
}
