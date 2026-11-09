//! Owned menu tree, focus, selection and event policy.
use crate::engine::input::InputSession;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Widget {
    Choice(usize),
    Slider(usize),
    Button(usize),
    Label(usize),
    Text,
    Control(usize),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Up,
    Down,
    Left,
    Right,
    Select,
    Cancel,
    Delete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    None,
    Enter(usize, bool),
    Finish,
    EditKeys,
    ChoiceChanged(usize),
    GammaStep(i32),
    EditText,
    Rebind(usize),
    Clear(usize),
}

impl Event {
    pub(super) fn direction(input: &InputSession) -> Option<Self> {
        let keys = input.pulsed().menu;
        [
            (5, Self::Up),
            (6, Self::Down),
            (7, Self::Left),
            (8, Self::Right),
        ]
        .into_iter()
        .find_map(|(key, event)| (keys[key] != 0).then_some(event))
    }
}

pub(super) struct MenuState {
    pub(super) children: [Vec<Widget>; 8],
    pub(super) current: usize,
    pub(super) focus: [usize; 8],
    pub(super) choices: [usize; 24],
    pub(super) highlight: [usize; 24],
    pub(super) counts: [usize; 24],
    pub(super) sliders: [i32; 4],
    pub(super) binding_column: usize,
}

impl MenuState {
    pub(super) fn new(counts: [usize; 24]) -> Self {
        use Widget::{Button as B, Choice as C, Control as K, Label as L, Slider as S, Text};
        Self {
            children: [
                vec![B(2), B(3), B(4), B(5), B(6), B(7), B(0)],
                vec![C(0), C(23), C(10), S(3), C(2), C(3), B(1)],
                vec![S(0), S(1), S(2), C(14), C(9), C(21), C(22), B(1)],
                vec![C(4), C(5), C(6), C(7), C(8), C(13), C(11), C(17), B(1)],
                vec![L(0), B(1)],
                vec![C(18), C(19), L(1), B(8), B(1)],
                vec![C(12), C(15), C(16), B(1)],
                vec![
                    C(20),
                    L(2),
                    Text,
                    K(0),
                    K(1),
                    K(2),
                    K(3),
                    K(4),
                    K(5),
                    K(6),
                    B(9),
                ],
            ],
            current: 0,
            focus: [0; 8],
            choices: [0; 24],
            highlight: [0; 24],
            counts,
            sliders: [0; 4],
            binding_column: 0,
        }
    }

    pub(super) fn focused(&self) -> Widget {
        self.children[self.current][self.focus[self.current]]
    }

    pub(super) fn enter(&mut self, menu: usize, down: bool) {
        let previous = self.focused();
        self.current = menu;
        let focus = if down { 0 } else { self.focus[menu] };
        self.focus_at(focus, previous);
    }

    fn focus_at(&mut self, mut index: usize, previous: Widget) {
        while matches!(self.children[self.current][index], Widget::Label(_)) {
            index += 1;
        }
        self.focus[self.current] = index;
        match self.focused() {
            Widget::Choice(choice) => self.highlight[choice] = self.choices[choice],
            Widget::Control(_) if matches!(previous, Widget::Control(_)) => (),
            _ => self.binding_column = 0,
        }
    }

    fn move_focus(&mut self, up: bool) {
        let previous = self.focused();
        let count = self.children[self.current].len();
        let mut index = self.focus[self.current];
        loop {
            index = if up {
                (index + count - 1) % count
            } else {
                (index + 1) % count
            };
            if !matches!(self.children[self.current][index], Widget::Label(_)) {
                break;
            }
        }
        self.focus_at(index, previous);
    }

    pub(super) fn event(&mut self, event: Event) -> Action {
        match event {
            Event::Up | Event::Down => {
                self.move_focus(event == Event::Up);
                return Action::None;
            }
            Event::Cancel => {
                let index = self.children[self.current].len() - 1;
                self.focus_at(index, self.focused());
                return self.event(Event::Select);
            }
            _ => (),
        }
        match self.focused() {
            Widget::Choice(choice) => self.choice_event(choice, event),
            Widget::Slider(3) if matches!(event, Event::Left | Event::Right) => {
                Action::GammaStep(if event == Event::Left { -1 } else { 1 })
            }
            Widget::Slider(slider) if matches!(event, Event::Left | Event::Right) => {
                self.sliders[slider] = (self.sliders[slider]
                    + if event == Event::Left { -5 } else { 5 })
                .clamp(0, 100);
                Action::None
            }
            Widget::Button(button) if event == Event::Select => button_action(button),
            Widget::Text if event == Event::Select => Action::EditText,
            Widget::Control(control) => self.control_event(control, event),
            _ => Action::None,
        }
    }

    fn choice_event(&mut self, choice: usize, event: Event) -> Action {
        let count = self.counts[choice];
        match event {
            Event::Left => self.highlight[choice] = (self.highlight[choice] + count - 1) % count,
            Event::Right => self.highlight[choice] = (self.highlight[choice] + 1) % count,
            Event::Select => {
                self.choices[choice] = self.highlight[choice];
                return Action::ChoiceChanged(choice);
            }
            _ => (),
        }
        Action::None
    }

    fn control_event(&mut self, control: usize, event: Event) -> Action {
        match event {
            Event::Left | Event::Right => {
                self.binding_column = 1 - self.binding_column;
                Action::None
            }
            Event::Select => Action::Rebind(control),
            Event::Delete => Action::Clear(control),
            _ => Action::None,
        }
    }
}

fn button_action(button: usize) -> Action {
    match button {
        0 => Action::Finish,
        1 => Action::Enter(0, false),
        2 => Action::Enter(1, true),
        3 => Action::Enter(3, true),
        4 => Action::Enter(2, true),
        5 => Action::Enter(4, true),
        6 | 9 => Action::Enter(5, true),
        7 => Action::Enter(6, true),
        8 => Action::EditKeys,
        _ => unreachable!("setup button index"),
    }
}
