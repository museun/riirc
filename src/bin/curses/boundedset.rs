use std::collections::VecDeque;
use std::fmt::Debug;

#[derive(Debug)]
pub struct BoundedSet<T: Debug + Clone> {
    data: VecDeque<T>,
    max: usize,
}

impl<T> BoundedSet<T>
where
    T: Debug + Clone + PartialEq,
{
    pub fn new(max: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max),
            max,
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn insert(&mut self, item: T) -> Option<T> {
        // TODO implement a binary search for this
        if let Some(_) = self.data.iter().find(|&t| *t == item) {
            return None;
        }

        let out = if self.data.len() == self.data.capacity() {
            self.data.pop_front()
        } else {
            None
        };

        self.data.push_back(item);
        out
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.data.iter()
    }
}
