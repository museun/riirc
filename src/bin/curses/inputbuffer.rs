pub struct InputBuffer {
    max: usize,
    buf: Vec<char>,
}

impl InputBuffer {
    pub fn new(max: usize) -> Self {
        InputBuffer { max, buf: vec![] }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn push(&mut self, ch: char) {
        self.buf.push(ch)
    }

    pub fn display(&self) -> &[char] {
        if self.buf.len() <= self.max {
            &self.buf
        } else {
            &self.buf[self.buf.len() - self.max..]
        }
    }

    pub fn get_line(&self) -> &[char] {
        &self.buf
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn delete(&mut self, pos: usize, before: bool) {
        if self.buf.is_empty() {
            return;
        }

        let pos = if before { pos - 1 } else { pos };
        let pos = if self.buf.len() > self.max {
            match self.buf.len().checked_sub(self.max - pos) {
                Some(pos) => pos,
                None => return,
            }
        } else {
            pos
        };

        if pos >= self.buf.len() {
            return;
        }

        self.buf.remove(pos);
    }
}
