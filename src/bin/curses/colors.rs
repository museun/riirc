use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ColorPair {
    pub bold: bool,
    pub fg: Color,
    pub bg: Color,
}

impl ColorPair {
    pub fn new<C>(bold: bool, fg: C, bg: Option<C>) -> Self
    where
        C: Into<Color>,
    {
        ColorPair {
            bold,
            fg: fg.into(),
            bg: bg
                .map(|bg| bg.into())
                .or_else(|| Some(0i16.into()))
                .unwrap(),
        }
    }

    pub fn bold(&mut self) -> Self {
        self.bold = !self.bold;
        *self
    }
}

impl From<Color> for i16 {
    fn from(c: Color) -> i16 {
        match c {
            Color::Black => pancurses::COLOR_BLACK,
            Color::Blue => pancurses::COLOR_BLUE,
            Color::Cyan => pancurses::COLOR_CYAN,
            Color::Green => pancurses::COLOR_GREEN,
            Color::Magenta => pancurses::COLOR_MAGENTA,
            Color::Red => pancurses::COLOR_RED,
            Color::White => pancurses::COLOR_WHITE,
            Color::Yellow => pancurses::COLOR_YELLOW,

            Color::BrightBlack => pancurses::COLOR_BLACK + 8,
            Color::BrightBlue => pancurses::COLOR_BLUE + 8,
            Color::BrightCyan => pancurses::COLOR_CYAN + 8,
            Color::BrightGreen => pancurses::COLOR_GREEN + 8,
            Color::BrightMagenta => pancurses::COLOR_MAGENTA + 8,
            Color::BrightRed => pancurses::COLOR_RED + 8,
            Color::BrightWhite => pancurses::COLOR_WHITE + 8,
            Color::BrightYellow => pancurses::COLOR_YELLOW + 8,
        }
    }
}

impl From<i16> for Color {
    fn from(c: i16) -> Self {
        match c {
            pancurses::COLOR_BLACK => Color::Black,
            pancurses::COLOR_BLUE => Color::Blue,
            pancurses::COLOR_CYAN => Color::Cyan,
            pancurses::COLOR_GREEN => Color::Green,
            pancurses::COLOR_MAGENTA => Color::Magenta,
            pancurses::COLOR_RED => Color::Red,
            pancurses::COLOR_WHITE => Color::White,
            pancurses::COLOR_YELLOW => Color::Yellow,

            c => match c - 8 {
                pancurses::COLOR_BLACK => Color::BrightBlack,
                pancurses::COLOR_BLUE => Color::BrightBlue,
                pancurses::COLOR_CYAN => Color::BrightCyan,
                pancurses::COLOR_GREEN => Color::BrightGreen,
                pancurses::COLOR_MAGENTA => Color::BrightMagenta,
                pancurses::COLOR_RED => Color::BrightRed,
                pancurses::COLOR_WHITE => Color::BrightWhite,
                pancurses::COLOR_YELLOW => Color::BrightYellow,
                _ => unreachable!(),
            },
        }
    }
}

impl From<Color> for ColorPair {
    fn from(ck: Color) -> ColorPair {
        ColorPair::new(false, ck, None)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Color {
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Magenta,
    Yellow,
    White,

    BrightBlack,
    BrightBlue,
    BrightGreen,
    BrightCyan,
    BrightRed,
    BrightMagenta,
    BrightYellow,
    BrightWhite,
}

impl Color {
    pub fn bold(self) -> ColorPair {
        let mut color: ColorPair = self.into();
        color.bold()
    }
}

// expensive..
#[derive(Debug, PartialEq, Clone)]
pub struct Output {
    pub data: String,
    pub colors: Vec<(::std::ops::Range<usize>, ColorPair)>,
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

impl Output {
    pub fn new() -> OutputBuilder {
        OutputBuilder::new()
    }
}
