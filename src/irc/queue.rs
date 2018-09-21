use std::collections::VecDeque;

#[derive(Debug)]
pub struct Queue<T>
where
    T: ::std::fmt::Debug,
{
    queue: VecDeque<T>,
}

impl<T> Queue<T>
where
    T: ::std::fmt::Debug,
{
    pub fn new(size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(size),
        }
    }

    pub fn push(&mut self, data: T) -> Option<T> {
        let out = if self.queue.len() == self.queue.capacity() {
            self.queue.pop_front()
        } else {
            None
        };

        self.queue.push_back(data);
        out
    }

    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    pub fn back(&self) -> Option<&T> {
        self.queue.back()
    }

    pub fn nth_from_end(&self, n: usize) -> Option<&T> {
        self.queue.iter().rev().nth(n)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.len() == 0
    }

    pub fn clear(&mut self) {
        self.queue.clear()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.queue.iter()
    }
}
