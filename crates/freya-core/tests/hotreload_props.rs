use std::cell::RefCell;
use std::rc::Rc;

use freya::prelude::*;
use freya_testing::prelude::*;

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, e| Label::try_downcast(e).map(|l| l.text.to_string()))
}

// Minimal, VSV-free demonstration of the stale-props mechanism:
// a component whose props carry closures and whose PartialEq ignores them.
#[derive(Clone)]
struct Handle {
    read: Rc<dyn Fn() -> i64>,
    inc: Rc<dyn Fn()>,
}

impl PartialEq for Handle {
    fn eq(&self, _: &Self) -> bool {
        // closures are not comparable; components rely on this being "equal"
        // so that plain re-renders of the parent do not re-render the child.
        true
    }
}

#[derive(PartialEq)]
struct Widget {
    handle: Handle,
}

impl Component for Widget {
    fn render(&self) -> impl IntoElement {
        let handle = self.handle.clone();
        let read = self.handle.read.clone();
        rect()
            .width(Size::px(150.))
            .height(Size::px(40.))
            .background(Color::BLUE)
            .on_press(move |_| (handle.inc)())
            .child(label().text(format!("widget:{}", read())))
    }
}

fn app() -> impl IntoElement {
    let mut count = use_state(|| 0i64);

    let read = {
        let count = count;
        move || *count.read()
    };
    let inc = {
        let count = count;
        move || {
            let next = *count.peek() + 1;
            *count.write_unchecked() = next;
        }
    };

    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(Widget {
            handle: Handle { read: Rc::new(read), inc: Rc::new(inc) },
        })
        .child(label().text(format!("parent:{}", count())))
}

#[test]
fn stale_props_after_reload() {
    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));

    let (x, y) = test
        .find(|node, e| {
            Label::try_downcast(e).filter(|l| l.text.starts_with("widget:")).map(|_| {
                let a = node.layout().area;
                (a.min_x() as f64 + a.width() as f64 / 2., a.min_y() as f64 + a.height() as f64 / 2.)
            })
        })
        .expect("widget");

    test.click_cursor((x, y));
    test.sync_and_update();
    println!("pre-reload: {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent:1"), "pre-reload works");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("post-reload: {:?}", labels(&test));

    test.click_cursor((x, y));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    println!("post-click: {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent:1"), "post-reload click reaches parent");
}
