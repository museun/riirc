use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct NicklistWindow {
    parent: CWindow,
    window: CWindow,
    shown: AtomicBool,
    state: Arc<ui::State>,
}

impl NicklistWindow {
    pub fn new(parent: CWindow, state: Arc<ui::State>) -> Self {
        let bounds = parent.get_max_yx();
        let window = parent
            .subwin(0, 0, 0, bounds.1 - bounds.1 / 5)
            .expect("create nicklist");

        window.refresh();
        Self {
            parent,
            window: window.into(),
            shown: AtomicBool::new(false),
            state,
        }
    }
}

impl Window for NicklistWindow {
    fn window(&self) -> CWindow {
        self.window
    }
}

impl ui::Outputter for NicklistWindow {
    fn output(&self, output: ui::Output, eol: bool) {
        let window = self.window();
        for (r, cp) in output.colors.iter() {
            insert_into(&window, *cp, &output.data[r.start..r.end], false)
        }
        if eol {
            self.window().addch('\n');
        }
    }

    fn clear(&self) {
        self.window().clear();
    }
}

impl ui::NicklistWindow for NicklistWindow {
    fn is_visible(&self) -> bool {
        self.shown.load(Ordering::Relaxed)
    }

    fn toggle(&self) {
        self.window.clear();
        self.window.draw_box(0, 0);

        let bounds = self.parent.get_max_yx();
        let w = bounds.1 / 5;
        if !self.shown.load(Ordering::Relaxed) {
            self.window.resize(w, bounds.0 - 1);
        } else {
            self.window.resize(bounds.0 - 1, 0);
        }

        self.window.refresh();
        self.parent.refresh();
        self.shown.fetch_xor(true, Ordering::Relaxed);

        trace!("toggled: {}", self.shown.load(Ordering::Relaxed));
    }
}
