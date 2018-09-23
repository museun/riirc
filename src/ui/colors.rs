#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ColorPair {
    pub bold: bool,
    pub underline: bool,
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
            underline: false,
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
