use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use pancurses;

// do I actually want to store the nicknames?
pub struct NicklistWindow {
    parent: Arc<pancurses::Window>,
    window: pancurses::Window,
    shown: AtomicBool,
}

impl NicklistWindow {
    pub fn new(parent: Arc<pancurses::Window>) -> Self {
        let bounds = parent.get_max_yx();
        let window = parent
            .subwin(bounds.0 - 1, bounds.1 / 5, 0, bounds.1 - bounds.1 / 5)
            .expect("create nicklist");

        window.draw_box(0, 0);
        window.overlay(&parent);
        window.refresh();

        NicklistWindow {
            window,
            parent,
            shown: AtomicBool::new(false),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.shown.load(Ordering::Relaxed)
    }

    pub fn toggle(&self) {
        if !self.shown.load(Ordering::Relaxed) {
            self.window.overlay(&self.parent);
        } else {
            self.window.clear();
            self.parent.overlay(&self.window);
        }
        self.window.refresh();
        self.parent.refresh();
        self.shown.fetch_xor(true, Ordering::Relaxed);

        trace!("toggled: {}", self.shown.load(Ordering::Relaxed));
    }

    pub fn update(&self, list: &[&str]) {
        self.window.clear();
        self.window.draw_box(0, 0);

        for name in list {
            self.window.addstr(name);
            self.window.addstr("\n");
        }

        for i in 0..10 {
            self.window.addstr(
                ::std::iter::repeat('F')
                    .take(10)
                    .collect::<Vec<_>>()
                    .iter()
                    .fold(String::new(), |mut a, &c| {
                        a.push(c);
                        a
                    }),
            );
            self.window.addstr("\n");
        }
        self.window.refresh();
    }
}
