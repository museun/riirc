use super::*;

pub struct Nicklist {
    parent: Rc<Window>,
    window: Window,
    ctx: Rc<Context>,
}

impl Nicklist {
    pub fn new(parent: Rc<Window>, ctx: Rc<Context>) -> Self {
        let bounds = parent.get_max_yx();
        let window = parent
            .subwin(0, 0, 0, bounds.1 - bounds.1 / 5)
            .expect("create nicklist");

        Self {
            parent,
            window: window.into(),
            ctx,
        }
    }

    pub fn is_visible(&self) -> bool {
        false
    }

    pub fn toggle(&self) {}
}

impl_recv!(Nicklist);
