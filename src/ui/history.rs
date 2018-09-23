use super::*;

#[derive(Debug)]
pub struct History {
    queue: BoundedSet<String>,
    pos: i32,
}

impl Default for History {
    fn default() -> Self {
        Self {
            queue: BoundedSet::new(32),
            pos: -1,
        }
    }
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.pos = -1;
    }

    pub fn append(&mut self, data: impl Into<String>) {
        self.queue.insert(data.into());
        self.pos = -1;
    }

    pub fn backward(&mut self) -> Option<&String> {
        if self.queue.is_empty() || self.pos as usize == self.queue.len() {
            return None;
        }

        self.pos += 1;
        self.queue.iter().rev().nth(self.pos as usize)
    }

    pub fn forward(&mut self) -> Option<&String> {
        if self.queue.is_empty() {
            return None;
        }

        if self.pos > 0 {
            self.pos -= 1;
        }

        self.queue.iter().rev().nth(self.pos as usize)
    }
}
