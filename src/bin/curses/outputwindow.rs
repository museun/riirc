use super::*;
use pancurses;

pub struct OutputWindow {
    window: pancurses::Window,
}

impl OutputWindow {
    pub fn new(window: pancurses::Window) -> Self {
        Self { window }
    }

    // pub fn writeln(&self, s: impl AsRef<str>) {
    //     self.window.addstr(s.as_ref());
    //     self.window.addstr("\n");
    //     self.window.refresh();
    // }

    pub fn clear(&self) {
        self.window.erase();
        self.window.refresh();
    }
}

impl Window for OutputWindow {
    fn window(&self) -> &pancurses::Window {
        &self.window
    }
}
