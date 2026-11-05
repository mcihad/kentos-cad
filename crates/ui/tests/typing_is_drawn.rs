//! Yazılan her harf hemen çizilir (docs/adr/0976): iced 0.14 pencereyi yalnız bir mesaj ya da bir bileşenin istediği
//! kare için çizer. Yazılanı kendi tutan alanlar (sayı girişi, düzenlenen hücre, çip girişi) tuşta mesaj göndermez;
//! kareyi kendileri istemezse harf imlecin sonraki yanıp sönmesini ya da bir zamanlayıcıyı bekliyordu (sahibin ölçtüğü
//! bir buçuk saniye: Proje ayarları › Birim ve ölçek, Nesne denetçisi, Poz metraj ızgarası). Burada her alan, kökün
//! güvencesi (`presented`) olmadan, kendisi sınanır.

#![cfg(feature = "snapshot")]

use iced::widget::container;
use iced::{Element, Event, Point, Size, keyboard};
use kentos_ui::snapshot::{Input, Snapshot};
use kentos_ui::widget::{
    ChipInput, Choice, EditCell, InputField, NumberInput, SearchBox, Select, Suggest, text_field,
};

#[derive(Debug, Clone)]
enum Message {
    Text(String),
    Number(f64),
    Chips(Vec<String>),
    Pick(usize),
}

#[derive(Default)]
struct Form {
    text: String,
    number: f64,
    chips: Vec<String>,
    commits: Vec<String>,
    numbers: Vec<f64>,
}

fn update(form: &mut Form, message: Message) {
    match message {
        Message::Text(t) => {
            form.commits.push(t.clone());
            form.text = t;
        }
        Message::Number(v) => {
            form.numbers.push(v);
            form.number = v;
        }
        Message::Chips(c) => form.chips = c,
        Message::Pick(i) => form.commits.push(i.to_string()),
    }
}

/// The field at the window's top left, 300 px wide; pressed at `FIELD`.
fn placed<'a>(field: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(container(field).width(300)).padding(10).into()
}

const FIELD: Point = Point::new(60.0, 22.0);

/// Pressed, then `typed` key by key: whether each key's frame is drawn at once.
fn keys(view: fn(&Form) -> Element<'_, Message>, form: &mut Form, typed: &str) -> Vec<bool> {
    let mut snapshot = Snapshot::new(Size::new(420.0, 300.0)).expect("a renderer");
    snapshot.settle(form, view, &mut update);
    snapshot.input(form, view, &mut update, Input::Click(FIELD));
    snapshot
        .drawn(form, view, &mut update, Input::Type(typed.into()))
        .into_iter()
        .filter(|(e, _)| matches!(e, Event::Keyboard(keyboard::Event::KeyPressed { .. })))
        .map(|(_, drawn)| drawn)
        .collect()
}

fn number(form: &Form) -> Element<'_, Message> {
    placed(NumberInput::new(form.number, Message::Number).decimals(2))
}

fn cell(form: &Form) -> Element<'_, Message> {
    placed(EditCell::new(form.text.clone(), Message::Text))
}

fn numeric_cell(form: &Form) -> Element<'_, Message> {
    placed(EditCell::new(form.text.clone(), Message::Text).numeric(true))
}

fn chips(form: &Form) -> Element<'_, Message> {
    placed(ChipInput::new(&form.chips, Message::Chips))
}

fn plain(form: &Form) -> Element<'_, Message> {
    placed(text_field("Ad", &form.text).on_input(Message::Text))
}

fn field(form: &Form) -> Element<'_, Message> {
    placed(InputField::new("Ad", &form.text).on_input(Message::Text))
}

fn search(form: &Form) -> Element<'_, Message> {
    placed(SearchBox::new(form.text.clone(), "Ara", Message::Text))
}

fn suggest(form: &Form) -> Element<'_, Message> {
    placed(Suggest::new(form.text.clone(), "Yaz").on_input(Message::Text))
}

/// Sayı girişi (Proje ayarları › Ondalık basamak, yazı yükseklikleri): the value is the field's until Enter.
#[test]
fn a_number_field_draws_each_key_at_once() {
    let mut form = Form { number: 2.5, ..Form::default() };
    assert_eq!(keys(number, &mut form, "12"), [true, true]);
    assert!(form.numbers.is_empty(), "nothing goes to the application before Enter: {:?}", form.numbers);
}

/// Düzenlenen hücre (Nesne denetçisi, Poz metraj ve yol pencerelerinin ızgaraları): text and numbers alike.
#[test]
fn an_edited_cell_draws_each_key_at_once() {
    let mut form = Form { text: "Yol".into(), ..Form::default() };
    assert_eq!(keys(cell, &mut form, "ab"), [true, true]);
    let mut form = Form { text: "1".into(), ..Form::default() };
    assert_eq!(keys(numeric_cell, &mut form, "25"), [true, true]);
    assert!(form.commits.is_empty(), "nothing goes to the application before Enter: {:?}", form.commits);
}

#[test]
fn a_chip_field_draws_each_key_at_once() {
    let mut form = Form::default();
    assert_eq!(keys(chips, &mut form, "kat"), [true, true, true]);
}

/// The fields that send each change to the application are drawn with the application's frame.
#[test]
fn the_fields_that_send_each_key_are_drawn_too() {
    for (name, view) in [
        ("text_field", plain as fn(&Form) -> Element<'_, Message>),
        ("InputField", field),
        ("SearchBox", search),
        ("Suggest", suggest),
    ] {
        let mut form = Form::default();
        assert_eq!(keys(view, &mut form, "ab"), [true, true], "{name}");
        assert_eq!(form.text, "ab", "{name}");
    }
}

/// A searchable list's search, typed in its open panel.
#[test]
fn a_searchable_list_draws_each_key_at_once() {
    fn list(_: &Form) -> Element<'_, Message> {
        placed(
            Select::new(["Ankara", "Sivas", "Kayseri"].into_iter().map(Choice::new), Some(0), Message::Pick)
                .searchable(true),
        )
    }
    let mut form = Form::default();
    assert_eq!(keys(list, &mut form, "si"), [true, true]);
}
