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

fn parent_sees(ls: &[String]) -> usize {
    ls.iter()
        .find_map(|l| l.strip_prefix("parent-sees:"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX)
}

fn first_row(ls: &[String]) -> String {
    ls.iter().find(|l| *l == "S" || *l == "U").cloned().unwrap_or_default()
}

// Exact user flow: data_table-style VSV whose rows write `selected` (State prop),
// parent reads it for a "Review"-style button AND relays via use_side_effect.
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
    let mut relayed = use_state(|| 0i64);

    // The user's relay effect (import_menu.rs:106-115)
    use_side_effect(move || {
        let n = selected.read().len() as i64;
        relayed.set(n);
    });

    let count = selected.read().len();
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .child(table(items.read().clone(), selected))
        .child(label().text(format!("parent-sees:{}", count)))
        .child(label().text(format!("relayed:{}", relayed())))
}

#[test]
fn vsv_selection_full_flow() {
    let mut test = launch_test(app);
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(30));
    labels_of(&mut test, "A initial");

    test.click_cursor((5., 5.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "B click row0");

    test.simulate_hot_reload();
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "C after reload");

    test.click_cursor((5., 40.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "D click row1");

    test.click_cursor((5., 5.));
    test.sync_and_update();
    test.poll(std::time::Duration::from_millis(1), std::time::Duration::from_millis(50));
    labels_of(&mut test, "E click row0");
}
