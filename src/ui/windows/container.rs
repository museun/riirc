use super::*;
use std::cell::RefCell;

pub struct Container {
    window: Rc<Window>,
    input: Rc<RefCell<Input>>,
    output: Rc<Output>,
    nicklist: Rc<Nicklist>,
    ctx: Rc<Context>,
    queue: Rc<ui::MessageQueue<ui::Request>>,
}

impl Container {
    pub fn new(state: Rc<ui::State>) -> Self {
        // each window should have its own message queue
        // impl_recv can merge them into the container queue
        let queue = Rc::new(ui::MessageQueue::new());
        let ctx = Rc::new(Context {
            state: Rc::clone(&state),
            queue: Rc::clone(&queue),
        });

        let container = pancurses::initscr();
        pancurses::start_color();
        pancurses::use_default_colors();

        let max = pancurses::COLOR_PAIRS();
        let (l, r, s, h) = if max >= 16 {
            (0x0F, 0xF0, 0x04, 0x10)
        } else {
            (0x07, 0x38, 0x03, 0x08)
        };

        for n in (0..max).map(|s| s as i16) {
            if n < h {
                pancurses::init_pair(n, n, -1);
            } else {
                pancurses::init_pair(n, n & l, (n & r) >> s);
            }
        }

        pancurses::curs_set(1);
        pancurses::noecho();

        let window = Rc::new(container.into());
        let input = Input::new(Rc::clone(&window), Rc::clone(&ctx));
        let output = Output::new(Rc::clone(&window), Rc::clone(&ctx));
        let nicklist = Nicklist::new(Rc::clone(&window), Rc::clone(&ctx));

        Self {
            window,
            input: Rc::new(RefCell::new(input)),
            output: Rc::new(output),
            nicklist: Rc::new(nicklist),
            ctx,
            queue,
        }
    }

    pub fn add_and_clear(&mut self) {
        self.input.borrow_mut().add_history();
        self.input.borrow_mut().clear_input();
    }

    pub fn step(&mut self) -> ReadType {
        self.input.borrow_mut().read_input()
    }

    pub fn output(&self) -> Rc<Output> {
        Rc::clone(&self.output)
    }

    pub fn input(&self) -> Rc<RefCell<Input>> {
        Rc::clone(&self.input)
    }

    pub fn nicklist(&self) -> Rc<Nicklist> {
        Rc::clone(&self.nicklist)
    }
}

impl_recv!(Container);
