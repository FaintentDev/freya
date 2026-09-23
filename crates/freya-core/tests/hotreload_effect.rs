use freya::prelude::*;
use freya_testing::prelude::*;

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, e| Label::try_downcast(e).map(|l| l.text.to_string()))
}

fn center_of(test: &TestingRunner, pred: impl Fn(&str) -> bool) -> (f64, f64) {
    test.find(|node, e| {
        Label::try_downcast(e).filter(|l| pred(&l.text)).map(|_| {
            let a = node.layout().area;
            (a.min_x() as f64 + a.width() as f64 / 2., a.min_y() as f64 + a.height() as f64 / 2.)
        })
    })
    .expect("target label")
}

// A component with BOTH:
//  - internal hook state (`open`)
//  - an always-equal prop carrying state captured in a closure (`Writable`)
#[derive(Default, Clone, PartialEq)]
struct Selection(Vec<i64>);

#[derive(PartialEq)]
struct Picker {
    value: Writable<Selection>,
}

impl Component for Picker {
    fn render(&self) -> impl IntoElement {
        let mut open = use_state(|| false);
        let value = self.value.read();
        let mut value2 = self.value.clone();
        rect()
            .width(Size::px(150.))
            .height(Size::px(80.))
            .background(Color::BLUE)
            .on_press(move |_| open.toggle())
            .child(label().text(format!("open:{} n:{}", open(), value.0.len())))
            .maybe_child(open().then(|| {
                Button::new()
                    .on_press(move |_| {
                        let mut v = value2.peek().0.clone();
                        v.push(1);
                        value2.set(Selection(v));
                    })
                    .child(label().text("inc"))
            }))
    }
}

fn app() -> impl IntoElement {
    let value = use_state(|| Selection(vec![]));
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(Picker { value: value.into_writable() })
        .child(label().text(format!("parent-n:{}", value.read().0.len())))
}

#[test]
fn internal_state_refreshes_after_reload() {
    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));

    // bump n to 1 so the always-equal prop is observably stale after reload
    let (x, y) = center_of(&test, |t| t.starts_with("open:"));
    test.click_cursor((x, y)); // open
    test.sync_and_update();
    let (x, y) = center_of(&test, |t| t == "inc");
    test.click_cursor((x, y)); // n -> 1
    test.sync_and_update();
    assert!(labels(&test).iter().any(|l| l == "parent-n:1"));

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));

    let after = labels(&test);
    println!("after reload: {:?}", after);

    // After reload hook state is reset; the label must be rebuilt from fresh state.
    // (The stale-prop bug surfaced here as `n:1` on stock main.)
    assert!(after.iter().any(|l| l == "open:false n:0"), "stale prop state must not survive reload, got {after:?}");
    assert!(after.iter().any(|l| l == "parent-n:0"), "parent reset");
}

#[test]
fn prop_carried_state_propagates_after_reload() {
    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));

    // open and increment: the Writable write must reach the parent scope.
    let (x, y) = center_of(&test, |t| t.starts_with("open:"));
    test.click_cursor((x, y));
    test.sync_and_update();
    let (x, y) = center_of(&test, |t| t == "inc");
    test.click_cursor((x, y));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));

    let after = labels(&test);
    println!("after click: {:?}", after);
    assert!(after.iter().any(|l| l == "parent-n:1"), "prop-carried state reaches parent, got {after:?}");
}
