use super::*;
use std::collections::BTreeMap;

pub trait Outputter {
    fn output(&self, output: impl Into<Output>, eol: bool);
}

#[derive(Debug, PartialEq, Clone)]
pub struct Output {
    pub data: String,
    pub colors: Vec<(::std::ops::Range<usize>, ColorPair)>,
}

impl Output {
    pub fn new() -> OutputBuilder {
        OutputBuilder::new()
    }
}

pub struct OutputBuilder {
    parts: Vec<String>,
    colors: BTreeMap<usize, ColorPair>,
    index: usize,
}

impl OutputBuilder {
    pub fn new() -> Self {
        Self {
            parts: vec![],
            colors: BTreeMap::new(),
            index: 0,
        }
    }

    pub fn bold(&mut self) -> &mut Self {
        let entry = self.colors.entry(self.index);
        entry
            .or_insert_with(|| ColorPair::new(true, Color::White, None))
            .bold = true;

        self
    }

    pub fn underline(&mut self) -> &mut Self {
        let entry = self.colors.entry(self.index);
        entry
            .or_insert_with(|| ColorPair::new(false, Color::White, None))
            .underline = true;

        self
    }

    pub fn fg(&mut self, c: impl Into<Color>) -> &mut Self {
        let c = c.into();
        let entry = self.colors.entry(self.index);
        entry.or_insert_with(|| ColorPair::new(false, c, None)).fg = c;
        self
    }

    pub fn bg(&mut self, c: impl Into<Color>) -> &mut Self {
        let c = c.into();
        let entry = self.colors.entry(self.index);
        entry
            .or_insert_with(|| ColorPair::new(false, Color::White, Some(c)))
            .bg = c;
        self
    }

    pub fn add<T>(&mut self, s: T) -> &mut Self
    where
        T: AsRef<str>,
    {
        self.colors
            .entry(self.index)
            .or_insert_with(|| ColorPair::new(false, Color::White, None));

        let s = s.as_ref().to_owned();
        self.index += s.len();
        self.parts.push(s);
        self
    }

    pub fn build(&self) -> Output {
        Output {
            colors: self.colors.iter().zip(self.parts.iter()).fold(
                Vec::new(),
                |mut a, ((k, v), s)| {
                    a.push(((*k..*k + s.len()), *v));
                    a
                },
            ),
            data: self.parts.iter().fold(String::new(), |mut a, c| {
                a.push_str(c);
                a
            }),
        }
    }
}
