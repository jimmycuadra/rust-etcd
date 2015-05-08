pub struct CompareAndDeleteConditions<'a> {
    pub modified_index: Option<u64>,
    pub value: Option<&'a str>,
}

impl<'a> CompareAndDeleteConditions<'a> {
    pub fn is_empty(&self) -> bool {
        self.modified_index.is_none() && self.value.is_none()
    }
}

impl<'a> Clone for CompareAndDeleteConditions<'a> {
    fn clone(&self) -> CompareAndDeleteConditions<'a> {
        CompareAndDeleteConditions {
            modified_index: self.modified_index.clone(),
            value: self.value.clone(),
        }
    }
}
