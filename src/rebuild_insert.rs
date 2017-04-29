use std::collections::BTreeSet;
use std::cmp::Ord;

pub trait RebuildInsertion<T> {
    fn rebuild_insert(&mut self, T) -> bool;
}

impl<T> RebuildInsertion<T> for BTreeSet<T>
    where T: Ord
{
    fn rebuild_insert(&mut self, value: T) -> bool {
        // this will make this insertion as if with a rebuild after inserting.
        let flag = self.remove(&value);
        self.insert(value);
        flag
    }
}