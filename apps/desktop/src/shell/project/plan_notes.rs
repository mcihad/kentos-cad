//! Proje ayarları › Mimari › Plan notları (docs/adr/8084): the parcel's plan notes that give a value the regulation's
//! rules read over the municipality's and the national one (Planlı Alanlar İmar Yönetmeliği 69/3; 28/1 leaves the storey
//! heights to the plan) and the plan's building height and storey count 5/11 reads (docs/adr/8098) and its subasman
//! 4/1-yyy leaves to it (docs/adr/8158), a row each: Konu, Değer (m, or storeys), Kaynak, delete; Plan notu ekle.
//! KentOS does not read a note's words into a number: the architect writes the value and where it stands. What does not
//! hold is said under the rows and Kaydet waits; Kaydet writes them through `cad.settings.set`'s `arch.planNotes`.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Length};
use kentos_commands::{PlanNote, PlanNoteKey};
use kentos_ui::icon::icon;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::Message;

/// A plan note as typed.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub key: PlanNoteKey,
    pub value: String,
    pub source: String,
}

/// A row's change.
#[derive(Clone, Debug)]
pub enum Edit {
    Key(PlanNoteKey),
    Value(String),
    Source(String),
    Delete,
}

/// The project's notes as rows.
pub fn rows(notes: &[PlanNote]) -> Vec<Note> {
    notes
        .iter()
        .map(|n| Note {
            key: n.key,
            value: kentos_processing::text::js_number(n.value),
            source: n.source.clone(),
        })
        .collect()
}

/// A row changed or taken away.
pub fn edit(rows: &mut Vec<Note>, i: usize, edit: Edit) {
    let Some(r) = rows.get_mut(i) else { return };
    match edit {
        Edit::Key(k) => r.key = k,
        Edit::Value(t) => r.value = t,
        Edit::Source(t) => r.source = t,
        Edit::Delete => {
            rows.remove(i);
        }
    }
}

/// A new row: the first measure no row gives yet, else the first; empty.
pub fn new_row(rows: &[Note]) -> Note {
    let key = PlanNoteKey::ALL
        .into_iter()
        .find(|k| rows.iter().all(|r| r.key != *k))
        .unwrap_or(PlanNoteKey::ALL[0]);
    Note { key, value: String::new(), source: String::new() }
}

/// A value as typed, a comma or a point before its decimals.
fn value(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The rows read as the project's notes, or what does not hold.
pub fn read(rows: &[Note]) -> Result<Vec<PlanNote>, String> {
    let mut notes = Vec::with_capacity(rows.len());
    for r in rows {
        let Some(v) = value(&r.value) else {
            return Err(format!("“{}” plan notunun değeri bir sayı olmalı ({}).", r.key.label(), r.key.unit()));
        };
        notes.push(PlanNote { key: r.key, value: v, source: r.source.trim().to_owned() });
    }
    match PlanNote::list_problem(&notes) {
        Some((_, why)) => Err(why),
        None => Ok(notes),
    }
}

/// A value's example in its field: metres for a height, a count of storeys.
fn placeholder(key: PlanNoteKey) -> &'static str {
    match key {
        PlanNoteKey::BuildingHeight => "9,50",
        PlanNoteKey::Storeys => "3",
        PlanNoteKey::PlinthMost => "1,50",
        PlanNoteKey::PlinthLeast => "-0,50",
        _ => "4,00",
    }
}

/// The group: the rows with what each sends, what does not hold, Plan notu ekle and what the notes do.
pub(super) fn view<'a>(
    rows: &'a [Note],
    on: impl Fn(usize, Edit) -> Message + Copy + 'a,
    on_add: Message,
) -> Element<'a, Message> {
    let head = kentos_ui::widget::table::band(
        row![
            container(kentos_ui::widget::table::title("Konu"))
                .padding([0, 8])
                .width(Length::FillPortion(5)),
            container(kentos_ui::widget::table::title("Değer"))
                .padding([0, 8])
                .width(Length::FillPortion(2)),
            container(kentos_ui::widget::table::title("Kaynak"))
                .padding([0, 8])
                .width(Length::FillPortion(5)),
            iced::widget::space().width(Length::Fixed(26.0)),
        ]
        .spacing(6),
    );
    let mut body = Column::new().spacing(4);
    if !rows.is_empty() {
        body = body.push(head);
    }
    for (i, r) in rows.iter().enumerate() {
        let key = Select::new(
            PlanNoteKey::ALL.iter().map(|k| Choice::new(k.label())),
            PlanNoteKey::ALL.iter().position(|k| *k == r.key),
            move |j| on(i, Edit::Key(PlanNoteKey::ALL.get(j).copied().unwrap_or(PlanNoteKey::ALL[0]))),
        )
        .searchable(false);
        body = body.push(
            row![
                container(key).width(Length::FillPortion(5)),
                container(super::arch::field(placeholder(r.key), &r.value, value(&r.value).is_none(), move |t| {
                    on(i, Edit::Value(t))
                }))
                .width(Length::FillPortion(2)),
                container(super::arch::field(
                    "UİP 12.3.2019, plan notu 7",
                    &r.source,
                    r.source.trim().is_empty(),
                    move |t| on(i, Edit::Source(t))
                ))
                .width(Length::FillPortion(5)),
                super::arch::small("trash", true, on(i, Edit::Delete)),
            ]
            .spacing(6)
            .align_y(Center),
        );
    }
    if let Err(why) = read(rows) {
        body = body.push(label::caption(why).style(style::text::danger).width(Fill));
    }
    let add = button(
        row![icon(crate::icons::from_web(Some("plus"))).size(13.0), label::body("Plan notu ekle")]
            .spacing(5)
            .align_y(Center),
    )
    .on_press(on_add)
    .padding([4, 10])
    .style(style::button::secondary);
    body = body.push(add).push(label::caption(
        "Parselin uygulama imar planının notu bir değeri değiştiriyorsa buraya yazılır: Denetim › Mevzuat o değeri yerel \
         imar yönetmeliğinin ve Planlı Alanlar İmar Yönetmeliği'nin önüne koyar (69/3; kat yükseklikleri plana \
         bırakılmıştır, 28/1). Zemin kat yüksekliği ±0,00'daki katın, kat yüksekliği bütün zemin ve normal katların \
         sınırıdır (m). Bina yüksekliği ±0,00'dan son katın tavan döşemesinin üstüne (m), kat adedi zemin ve normal \
         katlardır; planın bu değerleri aşılamaz (5/11). Subasman kotu zemin katın taban döşemesinin binanın \
         ±0,00'ından en çok ve en az yüksekliğidir (m, altında eksiyle); yazılmazsa +1,20 ve ±0,00'dır (4/1-yyy). \
         KentOS notun sözünü okumaz: değeri ve notun yerini planı okuyan yazar.",
    ));
    super::group::<Message>("Plan notları", body)
}
