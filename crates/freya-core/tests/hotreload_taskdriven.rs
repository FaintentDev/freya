use std::cell::RefCell;
use std::rc::Rc;

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

// Test A: use_memo chain (pagination in products/browser.rs)
#[test]
fn memo_driven_label() {
    fn app() -> impl IntoElement {
        let mut page = use_state(|| 1i64);
        let data = use_state(|| vec![1, 2, 3]);

        let pagination = use_memo(move || {
            let page = *page.read();
            let total = data.read().len() as i64;
            Some((page, total, page > 1, page < total))
        });

        rect()
            .width(Size::px(300.))
            .height(Size::px(300.))
            .background(Color::BLUE)
            .on_press({
                let mut page = page;
                move |_| {
                    let next = *page.peek() + 1;
                    page.set_if_modified(next);
                }
            })
            .child(label().text(format!(
                "page:{} total:{:?}",
                page(),
                pagination.read()
            )))
    }

    let mut test = launch_test(app);
    labels_of(&mut test, "A initial");

    test.click_cursor((150., 150.));
    test.sync_and_update();
    labels_of(&mut test, "B after click (page:2)");

    test.simulate_hot_reload();
    test.sync_and_update();
    labels_of(&mut test, "C after reload (page:1)");

    test.click_cursor((150., 150.));
    test.sync_and_update();
    labels_of(&mut test, "D after click (page:2, memo must update)");
}

// Test B: use_side_effect relay (import_menu pattern: child writes -> effect -> derived)
#[test]
fn side_effect_relay() {
    fn app() -> impl IntoElement {
        let mut source = use_state(|| std::collections::HashSet::<i64>::new());
        let mut relayed = use_state(|| 0i64);

        use_side_effect(move || {
            let n = source.read().len() as i64;
            relayed.set(n);
        });

        rect()
            .width(Size::px(300.))
            .height(Size::px(300.))
            .background(Color::BLUE)
            .on_press(move |_| {
                let mut s = source.peek().clone();
                s.insert(7);
                source.set(s);
            })
            .child(label().text(format!("relayed:{}", relayed())))
    }

    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "E initial");

    test.click_cursor((150., 150.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "F after click (relayed:1)");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "G after reload (relayed:0)");

    test.click_cursor((150., 150.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "H after click (relayed:1)");
}

#[test]
fn probe_layout() {
    fn app() -> impl IntoElement {
        let mut page = use_state(|| 1i64);
        rect()
            .width(Size::px(300.))
            .height(Size::px(300.))
            .background(Color::BLUE)
            .on_press(move |_| {
                let next = *page.peek() + 1;
                page.set(next);
            })
            .child(label().text(format!("page:{}", page())))
    }
    let mut test = launch_test(app);
    test.click_cursor((50., 50.));
    test.sync_and_update();
    labels_of(&mut test, "probe after click (page:2?)");
}
