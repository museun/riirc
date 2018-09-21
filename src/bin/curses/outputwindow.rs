use super::*;

pub struct OutputWindow {
    window: InnerWindow<OutputWindow>,
}

impl OutputWindow {
    pub fn resize(&self, cols: i32, rows: i32) {
        self.window.resize(rows, cols);
    }

    pub fn clear(&self) {
        self.window.erase();
        self.window.refresh();
    }
}

impl WindowBuilder<Self> for OutputWindow {
    fn create(parent: CWindow) -> CWindow {
        let (h, w) = parent.get_max_yx();
        let window = parent
            .subwin(h - 1, w, 0, 0)
            .expect("create output subwindow");

        window.setscrreg(0, window.get_max_y());
        window.scrollok(true);

        Arc::new(window)
    }

    fn new(window: InnerWindow<Self>) -> Self {
        Self { window }
    }
}

impl Window<Self> for OutputWindow {
    fn window(&self) -> InnerWindow<Self> {
        self.window
    }
}

/*

*/

/*


// what
pub trait Window {
    fn window(&self) -> &pancurses::Window;

    fn output(&self, output: &Output, eol: bool) {
        for (r, cp) in output.colors.iter() {
            self.insert(*cp, &output.data[r.start..r.end], false)
        }
        if eol {
            self.window().addch('\n');
        }
    }

    // TODO impl background colors
    fn insert(&self, color: impl Into<ColorPair>, s: impl AsRef<str>, eol: bool) {
        let window = self.window();
        let (y, x) = window.get_cur_yx();

        let color = color.into();
        let s = s.as_ref();
        window.addstr(s);

        let (ny, nx) = window.get_cur_yx();
        window.mvchgat(
            y,
            x,
            s.len() as i32,
            if color.bold {
                pancurses::A_BOLD
            } else {
                pancurses::A_NORMAL
            },
            color.fg.into(),
        );
        window.mv(ny, nx);
        if eol {
            window.addch('\n');
        }
        window.refresh();
    }
}

*/
