//! Klavyeyle odak geçişi: Tab sonraki, Shift+Tab önceki odaklanabilir öğeye gider (metin ve sayı
//! kutuları, KentOS UI'ın seçim kutusu, anahtar, onay kutusu ve parçalı seçimi). Odak yalnız en üstteki
//! **kapsamın** içinde döner: bir iletişim kutusu kendini kapsam olarak işaretler ([`scope_id`]), Tab onun
//! son öğesinden ilkine döner, arkadaki pencereye (komut satırına) kaçmaz. Kapsam yoksa bütün pencere.
//!
//! ```ignore
//! // Uygulamanın tuş yönlendiricisinde, hiçbir öğenin yakalamadığı Tab:
//! Named::Tab => kentos_ui::focus::next(shift),
//! ```
//!
//! iced'in kendi `focus_next`'i bütün ağacı dolaşır ve kapsam bilmez; burada iki geçiş vardır: önce
//! odaklanabilirler ve kapsamlar sayılır, sonra yalnız seçilen öğe odak alır, öbürleri bırakır.

use iced::Rectangle;
use iced::Task;
use iced::advanced::widget::operation::{self, Focusable, Operation, Outcome};
use iced::advanced::widget::{Id, operate};

/// Kendini odak kapsamı sayan kabın kimliği (iletişim kutusu).
pub const SCOPE: &str = "kentos-focus-scope";

/// Odak kapsamının kimliği: [`Dialog`](crate::widget::Dialog) kabını bununla işaretler.
pub fn scope_id() -> Id {
    Id::new(SCOPE)
}

/// Tab (`back`: Shift+Tab): en üstteki kapsamın içinde sonraki ya da önceki odaklanabilir öğe; hiçbiri
/// odakta değilse ilki (ya da sonuncusu).
pub fn next<T: Send + 'static>(back: bool) -> Task<T> {
    step(back, false)
}

/// Tab, yalnız kapsamın içinde bir öğe odaktayken: çizim alanında Tab'ın kendi anlamı (kenetin sıradaki
/// adayı) olduğunda, bir panelin kutusu odakta değilse hiçbir şey yapmaz.
pub fn next_if_focused<T: Send + 'static>(back: bool) -> Task<T> {
    step(back, true)
}

fn step<T: Send + 'static>(back: bool, only_if_focused: bool) -> Task<T> {
    operate(step_operation(back, only_if_focused)).discard()
}

/// Odak adımının kendisi: önce sayım, sonra odağın verilmesi. Görev olmadan çalıştıranlar için (pencere
/// açmadan çizen testler, `kentos_ui::snapshot`'ın `operate`'i).
pub fn step_operation(back: bool, only_if_focused: bool) -> impl Operation<()> + 'static {
    let survey = Survey {
        items: Vec::new(),
        scopes: Vec::new(),
        back,
        only_if_focused,
    };
    operation::then::<Plan, (), Apply>(survey, |plan| Apply { plan, current: 0 })
}

/// Bir odaklanabilir öğe: yeri ve odakta olup olmadığı, ziyaret sırasıyla.
#[derive(Clone, Copy)]
struct Item {
    bounds: Rectangle,
    focused: bool,
}

/// Odak alacak öğenin ziyaret sırasındaki yeri.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Plan {
    target: usize,
}

struct Survey {
    items: Vec<Item>,
    /// Kapsamlar: başladıkları öğe sırası ve sınırları, ziyaret sırasıyla (sonuncusu en üstteki).
    scopes: Vec<(usize, Rectangle)>,
    back: bool,
    only_if_focused: bool,
}

impl Operation<Plan> for Survey {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Plan>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id.is_some_and(|id| *id == scope_id()) {
            self.scopes.push((self.items.len(), bounds));
        }
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.items.push(Item {
            bounds,
            focused: state.is_focused(),
        });
    }

    fn finish(&self) -> Outcome<Plan> {
        match choose(&self.items, &self.scopes, self.back, self.only_if_focused) {
            Some(target) => Outcome::Some(Plan { target }),
            None => Outcome::None,
        }
    }
}

/// Odak alacak öğe: en üstteki kapsamın içindeki görünür öğeler arasında odaktakinin sonraki ya da
/// öncekisi (sondan başa döner); odakta öğe yoksa ilki ya da sonuncusu, `only_if_focused`'ta hiçbiri.
fn choose(
    items: &[Item],
    scopes: &[(usize, Rectangle)],
    back: bool,
    only_if_focused: bool,
) -> Option<usize> {
    let inside = |i: usize, item: &Item| -> bool {
        let shown = item.bounds.width > 0.5 && item.bounds.height > 0.5;
        shown
            && match scopes.last() {
                Some((start, bounds)) => i >= *start && bounds.contains(item.bounds.center()),
                None => true,
            }
    };
    let candidates: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(i, item)| inside(*i, item))
        .map(|(i, _)| i)
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let at = candidates.iter().position(|&i| items[i].focused);
    match at {
        Some(k) => {
            let n = candidates.len();
            let next = if back { (k + n - 1) % n } else { (k + 1) % n };
            Some(candidates[next])
        }
        None if only_if_focused => None,
        None if back => candidates.last().copied(),
        None => candidates.first().copied(),
    }
}

/// İkinci geçiş: seçilen öğe odak alır, odaktaki başka her öğe (kapsamın dışındakiler de) bırakır.
struct Apply {
    plan: Plan,
    current: usize,
}

impl<T> Operation<T> for Apply {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        if self.current == self.plan.target {
            state.focus();
        } else if state.is_focused() {
            state.unfocus();
        }
        self.current += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(x: f32, focused: bool) -> Item {
        Item {
            bounds: Rectangle::new(iced::Point::new(x, 10.0), iced::Size::new(20.0, 10.0)),
            focused,
        }
    }

    #[test]
    fn tab_goes_round_the_topmost_scope_only() {
        // Two boxes behind the dialog (one focused there before it opened), three in it.
        let items = [
            item(0.0, false),
            item(30.0, false),
            item(200.0, false),
            item(230.0, true),
            item(260.0, false),
        ];
        let dialog = Rectangle::new(iced::Point::new(190.0, 0.0), iced::Size::new(100.0, 40.0));
        let scopes = [(2, dialog)];
        assert_eq!(choose(&items, &scopes, false, false), Some(4));
        assert_eq!(choose(&items, &scopes, true, false), Some(2));
        // From the last one, round to the dialog's first, not the command line behind it.
        let mut last = items;
        last[3].focused = false;
        last[4].focused = true;
        assert_eq!(choose(&last, &scopes, false, false), Some(2));
        // Nothing focused: the first, or with Shift the last.
        let mut none = items;
        none[3].focused = false;
        assert_eq!(choose(&none, &scopes, false, false), Some(2));
        assert_eq!(choose(&none, &scopes, true, false), Some(4));
        assert_eq!(choose(&none, &scopes, false, true), None);
    }

    #[test]
    fn without_a_scope_the_whole_window_and_hidden_ones_are_skipped() {
        let mut items = [item(0.0, true), item(30.0, false), item(60.0, false)];
        items[1].bounds.width = 0.0;
        assert_eq!(choose(&items, &[], false, false), Some(2));
        assert_eq!(choose(&items, &[], true, false), Some(2));
        assert_eq!(choose(&[], &[], false, false), None);
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod window_tests {
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::Operation;
    use iced::advanced::widget::operation::Focusable;
    use iced::widget::{column, stack, text_input};
    use iced::{Element, Rectangle, Size};

    use crate::snapshot::{Input, Snapshot};
    use crate::widget::tree_view::{Check, check_box};
    use crate::widget::{Choice, Dialog, Select};

    #[derive(Clone, Debug)]
    enum Message {
        Typed(String),
        Picked(usize),
        Toggled,
    }

    struct Form {
        name: String,
        kind: Option<usize>,
        visible: bool,
        said: Vec<String>,
    }

    fn view(form: &Form) -> Element<'_, Message> {
        let behind = text_input("Komut", "")
            .id("behind")
            .on_input(Message::Typed);
        let dialog = Dialog::new("Alanlar").max_height(360.0).push(column![
            text_input("Ad", &form.name)
                .id("name")
                .on_input(Message::Typed),
            Select::new(
                ["Metin", "Tam sayı", "Tarih"].map(Choice::new),
                form.kind,
                Message::Picked,
            ),
            check_box(
                if form.visible {
                    Check::Checked
                } else {
                    Check::Unchecked
                },
                Some(Message::Toggled),
            ),
            text_input("Not", "").id("note").on_input(Message::Typed),
        ]);
        // The window behind fills it, as the application's does; the dialog stands over it.
        stack![
            iced::widget::container(behind).height(iced::Fill),
            iced::widget::container(dialog).center(iced::Fill)
        ]
        .into()
    }

    fn update(form: &mut Form, message: Message) {
        form.said.push(format!("{message:?}"));
        match message {
            Message::Typed(t) => form.name = t,
            Message::Picked(i) => form.kind = Some(i),
            Message::Toggled => form.visible = !form.visible,
        }
    }

    /// The focused widget's place among all focusables, in visiting order.
    fn focused(snapshot: &mut Snapshot, form: &Form) -> Option<usize> {
        struct Find(Arc<Mutex<Option<usize>>>, usize);
        impl Operation for Find {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }
            fn focusable(
                &mut self,
                _id: Option<&iced::advanced::widget::Id>,
                _bounds: Rectangle,
                state: &mut dyn Focusable,
            ) {
                if state.is_focused() {
                    *self.0.lock().unwrap() = Some(self.1);
                }
                self.1 += 1;
            }
        }
        let found = Arc::new(Mutex::new(None));
        snapshot.operate(view(form), Box::new(Find(found.clone(), 0)));
        *found.lock().unwrap()
    }

    #[test]
    fn tab_goes_round_the_dialog_s_fields_and_space_ticks_the_box() {
        let mut snapshot = Snapshot::software(Size::new(640.0, 400.0)).expect("a renderer");
        let mut form = Form {
            name: String::new(),
            kind: Some(0),
            visible: false,
            said: Vec::new(),
        };
        let mut update = update;
        snapshot.settle(&mut form, view, &mut update);
        // Focusables in order: the box behind (0), then the dialog's name (1), list (2), check (3), note (4).
        let tab = |snapshot: &mut Snapshot, form: &Form, back: bool| {
            snapshot.operate(view(form), Box::new(super::step_operation(back, false)));
        };
        assert_eq!(focused(&mut snapshot, &form), None);
        tab(&mut snapshot, &form, false);
        assert_eq!(
            focused(&mut snapshot, &form),
            Some(1),
            "the dialog's first field"
        );
        for want in [2, 3, 4, 1] {
            tab(&mut snapshot, &form, false);
            assert_eq!(
                focused(&mut snapshot, &form),
                Some(want),
                "round the dialog, never behind it"
            );
        }
        tab(&mut snapshot, &form, true);
        assert_eq!(
            focused(&mut snapshot, &form),
            Some(4),
            "Shift+Tab goes back round"
        );
        // To the check box, and Space ticks it.
        tab(&mut snapshot, &form, true);
        assert_eq!(focused(&mut snapshot, &form), Some(3));
        snapshot.input(
            &mut form,
            view,
            &mut update,
            Input::Key(iced::keyboard::key::Named::Space),
        );
        assert!(form.visible, "{:?}", form.said);
        // To the list: ↓ chooses the next kind without opening it.
        tab(&mut snapshot, &form, true);
        assert_eq!(focused(&mut snapshot, &form), Some(2));
        snapshot.input(
            &mut form,
            view,
            &mut update,
            Input::Key(iced::keyboard::key::Named::ArrowDown),
        );
        assert_eq!(form.kind, Some(1), "{:?}", form.said);
    }
}
