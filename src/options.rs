/// Possible conditions for "compare and delete" and "compare and swap" operations.
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
pub struct DeleteOptions<'a> {
    /// Conditions used for "compare and delete" or "compare and swap" operations.
    pub conditions: Option<ComparisonConditions<'a>>,
    /// Whether or not the key to be deleted is a directory.
    pub dir: Option<bool>,
    /// Whether or not keys within a directory should be deleted recursively.
    pub recursive: Option<bool>,
}

impl<'a> Default for DeleteOptions<'a> {
    /// Provides default values for the struct's fields.
    fn default() -> DeleteOptions<'a> {
        DeleteOptions {
            conditions: None,
            dir: None,
            recursive: None,
        }
    }
}
