use std::cell::RefCell;
use std::rc::Rc;

use freya::prelude::*;
use freya_testing::prelude::*;


fn center_of(test: &TestingRunner, text: &str) -> Option<(f32, f32)> {
    test.find(|node, e| {
        Label::try_downcast(e)
            .filter(|l| l.text.as_ref() == text)
            .map(|_| {
                let a = node.layout().area;
                (a.min_x() + a.width() / 2., a.min_y() + a.height() / 2.)
            })
    })
}

fn labels(test: &TestingRunner) -> Vec<String> {
    test.find_many(|_, e| Label::try_downcast(e).map(|l| l.text.to_string()))
}

fn parent_sees(ls: &[String]) -> i64 {
    ls.iter()
        .find_map(|l| l.strip_prefix("parent-sees:"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(-1)
}

fn first_row(ls: &[String]) -> String {
    ls.iter().find(|l| *l == "S" || *l == "U").cloned().unwrap_or_default()
}

// ---------------------------------------------------------------- Case 1: VSV
// State written from inside a VirtualScrollView builder closure.
#[derive(PartialEq, Clone)]
struct Item { id: i64 }

fn vsv_table(items: Rc<Vec<Item>>, mut selected: State<std::collections::HashSet<i64>>) -> impl IntoElement {
    let body = VirtualScrollView::new_with_data(items.clone(), move |item, items| {
        let row = &items[item.index];
        let id = row.id;
        let is_selected = selected.read().contains(&id);
        rect()
            .height(Size::px(35.))
            .background(if is_selected { Color::GREEN } else { Color::RED })
            .on_press(move |_| {
                let mut s = selected.peek().clone();
                if !s.remove(&id) { s.insert(id); }
                selected.set(s);
            })
            .child(label().text(if is_selected { "S" } else { "U" }))
            .into()
    })
    .length(items.len())
    .item_size(35.);
    ScrollView::new().height(Size::px(300.)).child(body)
}

fn vsv_app() -> impl IntoElement {
    let mut selected = use_state(|| std::collections::HashSet::<i64>::new());
    let items = use_state(|| Rc::new((0..20).map(|i| Item { id: i }).collect::<Vec<_>>()));
    let count = selected.read().len();
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(vsv_table(items.read().clone(), selected))
        .child(label().text(format!("parent-sees:{}", count)))
}

#[test]
fn case1_virtual_scroll_view() {
    let mut test = launch_test(vsv_app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));
    test.click_cursor((5., 5.));
    test.sync_and_update();
    assert_eq!(parent_sees(&labels(&test)), 1, "pre-reload");
    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    let b = labels(&test);
    println!("[case1 B] {:?}", b);
    test.click_cursor((5., 40.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    let c = labels(&test);
    println!("[case1 C] {:?}", c);
    assert_eq!(parent_sees(&c), 1, "post-reload selection reaches parent");
}

// ------------------------------------------------- Case 2: always-equal props
// A struct Component with a `Writable` prop (PartialEq always true) and local state.
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
            .width(Size::px(120.))
            .height(Size::px(60.))
            .background(Color::BLUE)
            .on_press(move |_| open.toggle())
            .child(label().text(format!("picker-open:{} n:{}", open(), value.0.len())))
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

fn picker_app() -> impl IntoElement {
    let value = use_state(|| Selection(vec![]));
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(Picker { value: value.into_writable() })
        .child(label().text(format!("parent-n:{}", value.read().0.len())))
}

#[test]
fn case2_always_equal_props_component() {
    let mut test = launch_test(picker_app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));
    // open picker
    let (x, y) = center_of(&test, "picker-open:false n:0").expect("picker label");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    assert!(labels(&test).iter().any(|l| l.starts_with("picker-open:true")), "picker opens pre-reload");
    // increment via writable
    let (x, y) = center_of(&test, "inc").expect("inc button");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    println!("[case2 pre-inc] {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent-n:1"), "writable reaches parent pre-reload");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));

    println!("[case2 A pre] {:?}", labels(&test));
    println!("[case2 after reload] {:?}", labels(&test));
    // open again (label text may be stale, match by prefix)
    let (x, y) = test
        .find(|node, e| {
            Label::try_downcast(e)
                .filter(|l| l.text.starts_with("picker-open:"))
                .map(|_| {
                    let a = node.layout().area;
                    (a.min_x() + a.width() / 2., a.min_y() + a.height() / 2.)
                })
        })
        .expect("picker label after reload");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    println!("[case2 B opened post] {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l.starts_with("picker-open:true")), "picker opens after reload");

    // increment again
    let (x, y) = center_of(&test, "inc").expect("inc button after reload");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    println!("[case2 C inc post] {:?}", labels(&test));
    assert!(labels(&test).iter().any(|l| l == "parent-n:1"), "writable reaches parent after reload");
}

// ------------------------------------- Case 3: fn pagination inside a VSV row
#[derive(PartialEq, Clone)]
struct RowData { id: i64 }

fn pagination_row(mut page: State<i64>) -> impl IntoElement {
    let btn = move |p: i64| {
        Button::new()
            .child(label().text(p.to_string()))
            .on_press(move |_| page.set_if_modified(p))
    };
    rect()
        .horizontal()
        .child(btn(1))
        .child(btn(2))
        .child(label().text(format!("cur:{}", page())))
}

fn vsv_pagination_app() -> impl IntoElement {
    let mut page = use_state(|| 1i64);
    let rows = Rc::new(vec![RowData { id: 0 }]);
    let body = VirtualScrollView::new_with_data(rows, move |_, _| {
        pagination_row(page)
            .into_element()
    })
    .length(1usize)
    .item_size(30.);
    rect()
        .width(Size::fill())
        .height(Size::px(200.))
        .child(body)
        .child(label().text(format!("parent-page:{}", page())))
}

#[test]
fn case3_pagination_in_virtual_scroll_view() {
    let mut test = launch_test(vsv_pagination_app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));
    let (x, y) = center_of(&test, "2").expect("page 2 button");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    assert!(labels(&test).iter().any(|l| l == "parent-page:2"), "pagination reaches parent pre-reload");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));

    let (x, y) = center_of(&test, "2").expect("page 2 button after reload");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    assert!(labels(&test).iter().any(|l| l == "parent-page:2"), "pagination reaches parent after reload");
}

// ------------------------------- Case 4: internal state, PartialEq-equal props
// If this recovers on both main and the PR, then the PR changes nothing for the
// actual failure mode (props-based state capture).
#[derive(PartialEq)]
struct Counter {
    base: i64,
}

impl Component for Counter {
    fn render(&self) -> impl IntoElement {
        let mut count = use_state(|| self.base);
        rect()
            .width(Size::px(150.))
            .height(Size::px(40.))
            .background(Color::BLUE)
            .on_press(move |_| {
                let next = *count.peek() + 1;
                count.set(next);
            })
            .child(label().text(format!("counter:{}", count())))
    }
}

fn counter_app() -> impl IntoElement {
    rect().width(Size::fill()).height(Size::fill()).child(Counter { base: 0 })
}

#[test]
fn case4_internal_state_equal_props() {
    let mut test = launch_test(counter_app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));
    let (x, y) = center_of(&test, "counter:0").expect("counter");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    assert!(labels(&test).iter().any(|l| l == "counter:1"), "pre-reload");
    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    println!("[case4 after reload] {:?}", labels(&test));
    let (x, y) = center_of(&test, "counter:0").expect("counter after reload");
    test.click_cursor((x as f64, y as f64));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    assert!(labels(&test).iter().any(|l| l == "counter:1"), "internal state recovers after reload");
}
