use std::cell::RefCell;
use std::rc::Rc;

use freya::prelude::*;
use freya::animation::*;

use freya_testing::prelude::*;

fn labels_of(test: &mut TestingRunner, tag: &str) {
    let out = Rc::new(RefCell::new(Vec::new()));
    let out2 = out.clone();
    test.find_many(move |_, e| {
        out2.borrow_mut().push(Label::try_downcast(e).map(|l| l.text.to_string()));
        None::<()>
    });
    println!("{tag}: {:?}", out.borrow());
}

// Replicates the user's dropdown exactly: Writable prop + animation + panel with rows
#[derive(Clone, PartialEq, Default)]
struct Selection(Vec<i64>);

#[derive(PartialEq)]
struct Dropdown {
    value: Writable<Selection>,
}

impl Component for Dropdown {
    fn render(&self) -> impl IntoElement {
        let mut open = use_state(|| false);
        let value = self.value.read();

        let opacity_anim = use_animation_transition(open, |from: bool, to: bool| {
            AnimNum::new(from as u8 as f32, to as u8 as f32).time(120).ease(Ease::Out)
        });
        let opacity = opacity_anim.read().value();

        let button = rect()
            .width(Size::px(120.))
            .height(Size::px(32.))
            .background(Color::BLUE)
            .on_press(move |_| open.toggle())
            .child(label().text(format!("open={} n={}", open(), value.0.len())));

        let rows = (0..3usize)
            .map(|idx| {
                let mut value = self.value.clone();
                rect()
                    .key(idx)
                    .height(Size::px(30.))
                    .background(Color::WHITE)
                    .on_press(move |_| {
                        let mut v = value.peek().0.clone();
                        v.push(idx as i64);
                        value.set(Selection(v));
                        open.set(false);
                    })
                    .child(label().text(format!("row {idx}")))
            })
            .collect::<Vec<_>>();

        let panel = rect()
            .width(Size::px(120.))
            .position(Position::new_absolute().top(34.))
            .opacity(opacity)
            .background(Color::YELLOW)
            .child(ScrollView::new().children(rows));

        rect()
            .width(Size::px(120.))
            .child(button)
            .maybe_child((open() || opacity > 0.).then(|| panel))
    }
}

#[derive(PartialEq)]
struct Screen;

impl Component for Screen {
    fn render(&self) -> impl IntoElement {
        let value = use_state(|| Selection(vec![]));
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(Dropdown { value: value.into_writable() })
    }
}

#[test]
fn dropdown_opacity_panel() {
    let mut test = launch_test(|| Screen.into_element());
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));
    labels_of(&mut test, "A initial");

    test.click_cursor((60., 10.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(300));
    labels_of(&mut test, "B open (panel should be in tree)");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(300));
    labels_of(&mut test, "C after reload");

    test.click_cursor((60., 10.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(300));
    labels_of(&mut test, "D open again (panel should appear)");
}
