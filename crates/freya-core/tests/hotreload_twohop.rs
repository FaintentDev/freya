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

#[derive(Default, Clone, PartialEq)]
struct Selection(Vec<i64>);

// Two-hop: parent owns State, passes Writable to Child, Child passes the SAME
// Writable deeper into a fn. Parent must observe child writes.
#[derive(PartialEq)]
struct Parent {
    value: Writable<Selection>,
}

impl Component for Parent {
    fn render(&self) -> impl IntoElement {
        let n = self.value.read().0.len();
        // pass the prop down unchanged, like ControlBar does
        let child = Child { value: self.value.clone() };
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(child)
            .child(label().text(format!("parent-n:{}", n)))
    }
}

#[derive(PartialEq)]
struct Child {
    value: Writable<Selection>,
}

impl Component for Child {
    fn render(&self) -> impl IntoElement {
        let value_r = self.value.read();
        let mut value_w = self.value.clone();
        let btn = rect()
            .width(Size::px(150.))
            .height(Size::px(40.))
            .background(Color::BLUE)
            .on_press(move |_| {
                let mut v = value_w.peek().0.clone();
                v.push(1);
                value_w.set(Selection(v));
            })
            .child(label().text(format!("child-n:{}", value_r.0.len())));
        rect().width(Size::fill()).height(Size::fill()).child(btn)
    }
}

fn app() -> impl IntoElement {
    let value = use_state(Selection::default);
    rect().child(Parent { value: value.into_writable() })
}

#[test]
fn two_hop_writable_propagation() {
    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));

    let (x, y) = test
        .find(|node, e| {
            Label::try_downcast(e).filter(|l| l.text.starts_with("child-n:")).map(|_| {
                let a = node.layout().area;
                (a.min_x() as f64 + a.width() as f64 / 2., a.min_y() as f64 + a.height() as f64 / 2.)
            })
        })
        .expect("child label");

    test.click_cursor((x, y));
    test.sync_and_update();
    println!("pre-reload: {:?}", labels_of_labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent-n:1"), "pre-reload write reaches parent");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("post-reload: {:?}", labels(&test));

    let (x, y) = test
        .find(|node, e| {
            Label::try_downcast(e).filter(|l| l.text.starts_with("child-n:")).map(|_| {
                let a = node.layout().area;
                (a.min_x() as f64 + a.width() as f64 / 2., a.min_y() as f64 + a.height() as f64 / 2.)
            })
        })
        .expect("child label after reload");

    test.click_cursor((x, y));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("post-click: {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent-n:1"), "post-reload write reaches parent");
}

fn labels_of_labels(test: &TestingRunner) -> Vec<String> {
    labels(test)
}

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, e| Label::try_downcast(e).map(|l| l.text.to_string()))
}

// Variant: the child renders the inner component through a .map()/.maybe_child
// conditional (like .maybe_child in ControlBar) and the state write happens
// inside a ScrollView of keyed rows.
#[derive(PartialEq)]
struct Parent2 {
    value: Writable<Selection>,
}

impl Component for Parent2 {
    fn render(&self) -> impl IntoElement {
        let n = self.value.read().0.len();
        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(Child2 { value: self.value.clone() })
            .child(label().text(format!("parent-n:{}", n)))
    }
}

#[derive(PartialEq)]
struct Child2 {
    value: Writable<Selection>,
}

impl Component for Child2 {
    fn render(&self) -> impl IntoElement {
        let value_r = self.value.read();
        let mut open = use_state(|| false);

        let rows = (0..3usize)
            .map(|idx| {
                let mut value_w = self.value.clone();
                let mut open = open;
                rect()
                    .key(idx)
                    .height(Size::px(30.))
                    .background(Color::WHITE)
                    .on_press(move |_| {
                        let mut v = value_w.peek().0.clone();
                        v.push(idx as i64);
                        value_w.set(Selection(v));
                        open.set(false);
                    })
                    .child(label().text(format!("row {idx}")))
            })
            .collect::<Vec<_>>();

        let panel = rect()
            .position(Position::new_absolute().top(34.))
            .background(Color::YELLOW)
            .child(ScrollView::new().children(rows));

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .on_press(move |_| open.toggle())
            .child(label().text(format!("child-n:{}", value_r.0.len())))
            .maybe_child(open().then(|| panel))
    }
}

fn app2() -> impl IntoElement {
    let value = use_state(Selection::default);
    rect().child(Parent2 { value: value.into_writable() })
}

#[test]
fn two_hop_row_writes() {
    let mut test = launch_test(app2);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));

    // open panel
    test.click_cursor((250., 15.));
    test.sync_and_update();
    // click row 1
    test.click_cursor((20., 50.));
    test.sync_and_update();
    println!("pre-reload: {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent-n:1"), "pre-reload");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("post-reload: {:?}", labels(&test));

    test.click_cursor((250., 15.));
    test.sync_and_update();
    test.click_cursor((20., 50.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("post-click: {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent-n:1"), "post-reload");
}

// Final variant replicating ControlBar exactly: parent owns `statuses` State,
// a use_side_effect relays it into `filters`, DropdownFilter writes to `statuses`
// via Writable. The side_effect (task) must re-spawn after reload.
#[derive(PartialEq)]
struct Parent3 {
    filters: State<i64>,
}

impl Component for Parent3 {
    fn render(&self) -> impl IntoElement {
        let mut statuses = use_state(Selection::default);
        let mut filters = self.filters;

        use_side_effect(move || {
            let n = statuses.read().0.len() as i64;
            println!("[relay] statuses.len()={} -> filters.set({})", n, n);
            filters.set(n);
        });

        rect()
            .width(Size::fill())
            .height(Size::fill())
            .child(Child3 { value: statuses.into_writable() })
            .child(label().text(format!("filters-set-to:{}", *filters.read())))
    }
}

#[derive(PartialEq)]
struct Child3 {
    value: Writable<Selection>,
}

impl Component for Child3 {
    fn render(&self) -> impl IntoElement {
        let value_r = self.value.read();
        let mut value_w = self.value.clone();
        rect()
            .width(Size::px(150.))
            .height(Size::px(40.))
            .background(Color::BLUE)
            .on_press(move |_| {
                let mut v = value_w.peek().0.clone();
                v.push(1);
                value_w.set(Selection(v));
            })
            .child(label().text(format!("child-n:{}", value_r.0.len())))
    }
}

fn app3() -> impl IntoElement {
    let filters = use_state(|| 0i64);
    rect().child(Parent3 { filters })
}

#[test]
fn side_effect_relay_two_hop() {
    let mut test = launch_test(app3);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));

    test.click_cursor((75., 20.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    println!("pre-reload: {:?}", labels(&test));
    println!("PRE-RELOAD-LABELS {:?}", labels(&test));

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("post-reload: {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "filters-set-to:0"), "hook reset");

    test.click_cursor((75., 20.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("post-click: {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "filters-set-to:1"), "relay must re-spawn after reload");
}
