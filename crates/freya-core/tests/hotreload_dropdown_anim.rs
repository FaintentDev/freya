use std::cell::RefCell;
use std::rc::Rc;

use freya::animation::*;
use freya::prelude::*;
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

// The exact user pattern: panel visibility gated on (open() || opacity > 0.),
// with opacity animated. After reload, click toggles open=true, but if the
// animation never runs, opacity stays 0 -> panel invisible but interactive.
fn app() -> impl IntoElement {
    let mut open = use_state(|| false);

    let opacity_anim = use_animation_transition(open, |from: bool, to: bool| {
        AnimNum::new(from as u8 as f32, to as u8 as f32).time(120).ease(Ease::Out)
    });
    let opacity = opacity_anim.read().value();

    let button = rect()
        .width(Size::px(120.))
        .height(Size::px(32.))
        .background(Color::BLUE)
        .on_press(move |_| open.toggle())
        .child(label().text(format!("open={} opacity={:.2}", open(), opacity)));

    let rows = (0..3usize)
        .map(|idx| {
            let mut open = open;
            rect()
                .key(idx)
                .height(Size::px(30.))
                .background(Color::WHITE)
                .on_press(move |_| {
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

#[test]
fn dropdown_anim_after_reload() {
    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));
    labels_of(&mut test, "A initial");

    test.click_cursor((60., 10.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(300));
    labels_of(&mut test, "B open (opacity should reach 1)");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(300));
    labels_of(&mut test, "C after reload");

    test.click_cursor((60., 10.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(300));
    labels_of(&mut test, "D open again (opacity must be 1)");
}
