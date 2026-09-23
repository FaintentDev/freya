use std::cell::RefCell;
use std::rc::Rc;

use freya::prelude::*;
use freya_testing::prelude::*;

fn dump(test: &mut TestingRunner, tag: &str) -> Vec<String> {
    let out = Rc::new(RefCell::new(Vec::new()));
    let out2 = out.clone();
    test.find_many(move |_, e| {
        out2.borrow_mut().push(Label::try_downcast(e).map(|l| l.text.to_string()));
        None::<()>
    });
    let s: Vec<String> = out.borrow().iter().flatten().cloned().collect();
    println!("{tag}: {}", s.join(" | "));
    s
}

#[derive(PartialEq, Clone)]
struct Item { id: i64 }

fn table(items: Rc<Vec<Item>>, mut selected: State<std::collections::HashSet<i64>>) -> impl IntoElement {
    let body = VirtualScrollView::new_with_data(items.clone(), move |item, items| {
        let row = &items[item.index];
        let id = row.id;
        let is_selected = selected.read().contains(&id);
        rect()
            .height(Size::px(35.))
            .background(if is_selected { Color::GREEN } else { Color::RED })
            .on_press(move |_| {
                let mut s = selected.peek().clone();
                if !s.remove(&id) {
                    s.insert(id);
                }
                selected.set(s);
            })
            .child(label().text(if is_selected { "S" } else { "U" }))
            .into()
    })
    .length(items.len())
    .item_size(35.);

    ScrollView::new().height(Size::px(300.)).child(body)
}

fn app() -> impl IntoElement {
    let mut selected = use_state(|| std::collections::HashSet::<i64>::new());
    let items = use_state(|| Rc::new((0..20).map(|i| Item { id: i }).collect::<Vec<_>>()));
    let count = selected.read().len();
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(table(items.read().clone(), selected))
        .child(label().text(format!("parent-sees:{}", count)))
}

fn parent_sees(labels: &[String]) -> usize {
    labels
        .iter()
        .find_map(|l| l.strip_prefix("parent-sees:"))
        .and_then(|v| v.parse().ok())
        .unwrap()
}

fn first_row(labels: &[String]) -> String {
    labels.iter().find(|l| *l == "S" || *l == "U").cloned().unwrap_or_default()
}

#[test]
fn hot_reload_orphans_virtual_scroll_view_state() {
    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));

    test.click_cursor((5., 5.));
    test.sync_and_update();
    let a = dump(&mut test, "A");
    assert_eq!(parent_sees(&a), 1, "pre-reload selection should reach parent");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    let b = dump(&mut test, "B");
    assert_eq!(parent_sees(&b), 0, "hook state is reset by reload");
    // observation only: is the stale row element replaced?

    test.click_cursor((5., 40.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(100));
    let c = dump(&mut test, "C");
    assert_eq!(parent_sees(&c), 1, "post-reload selection must reach parent");
}
