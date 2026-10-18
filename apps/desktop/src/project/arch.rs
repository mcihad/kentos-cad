//! Proje ayarları › Mimari (docs/adr/0339 §3, §6): the project's levels, a row each (Ad, Kot, Kat yüksekliği; up,
//! down, delete; Kat ekle), and the detail the walls are drawn at (Ölçeğe göre, Şematik, Standart, Ayrıntılı). What
//! does not hold is said under the rows and Kaydet waits; Kaydet writes them through `cad.settings.set`'s `arch`,
//! which keeps a level a wall names.

use iced::widget::{Column, button, container, row, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_contracts::{ArchLevel, ArchSettings, WallDetail};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::Banner;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::Message;

/// A level as typed.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Its id: kept from the project's, a new one's made when it is added.
    pub id: String,
    pub name: String,
    pub elevation: String,
    pub height: String,
}

#[derive(Debug, Clone)]
pub enum Edit {
    Name(String),
    Elevation(String),
    Height(String),
    Up,
    Down,
    Delete,
}

/// The detail list: by the scale, then each.
/// The spaces' measurement methods Mimari offers (docs/adr/0342 §1): the office's profile; none: net.
const METHODS: [(Option<kentos_contracts::SpaceMethod>, &str); 3] = [
    (None, "Net (iç yüzler)"),
    (Some(kentos_contracts::SpaceMethod::Centerline), "Duvar ekseni"),
    (Some(kentos_contracts::SpaceMethod::Gross), "Brüt (dış yüzler)"),
];

const DETAILS: [(Option<WallDetail>, &str); 4] = [
    (None, "Ölçeğe göre"),
    (Some(WallDetail::Schematic), "Şematik"),
    (Some(WallDetail::Standard), "Standart"),
    (Some(WallDetail::Detailed), "Ayrıntılı"),
];

fn shown(n: Option<f64>) -> String {
    n.map(|v| format!("{v}")).unwrap_or_default()
}

fn number(s: &str) -> Result<Option<f64>, ()> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map(Some)
        .ok_or(())
}

/// The project's levels as rows.
pub fn rows(arch: Option<&ArchSettings>) -> Vec<Row> {
    arch.map_or_else(Vec::new, |a| {
        a.levels
            .iter()
            .map(|l| Row {
                id: l.id.clone(),
                name: l.name.clone(),
                elevation: shown(Some(l.elevation)),
                height: shown(l.height),
            })
            .collect()
    })
}

/// A new level: its name the next free “n. kat”, on the last one's top.
pub fn new_row(rows: &[Row]) -> Row {
    let n = rows.len();
    let name = if n == 0 {
        "Zemin kat".to_owned()
    } else {
        format!("{n}. kat")
    };
    let last = rows.last();
    let elevation = last.map_or(0.0, |r| {
        number(&r.elevation).ok().flatten().unwrap_or(0.0)
            + number(&r.height).ok().flatten().unwrap_or(3.0)
    });
    Row {
        id: kentos_domain::Uuid::now_v7().to_string(),
        name,
        elevation: shown(Some(elevation)),
        height: "3".to_owned(),
    }
}

pub fn edit(rows: &mut Vec<Row>, i: usize, e: Edit) {
    let n = rows.len();
    let Some(r) = rows.get_mut(i) else { return };
    match e {
        Edit::Name(t) => r.name = t,
        Edit::Elevation(t) => r.elevation = t,
        Edit::Height(t) => r.height = t,
        Edit::Up if i > 0 => rows.swap(i, i - 1),
        Edit::Down if i + 1 < n => rows.swap(i, i + 1),
        Edit::Up | Edit::Down => {}
        Edit::Delete => {
            rows.remove(i);
        }
    }
}

/// The rows and the detail read as the settings, or what does not hold.
pub fn read(
    rows: &[Row],
    detail: Option<WallDetail>,
    space_method: Option<kentos_contracts::SpaceMethod>,
) -> Result<ArchSettings, String> {
    let mut levels = Vec::with_capacity(rows.len());
    for r in rows {
        let elevation = match number(&r.elevation) {
            Ok(Some(v)) => v,
            _ => return Err(format!("“{}” katının kotu bir sayı olmalı.", r.name.trim())),
        };
        let height = number(&r.height)
            .map_err(|()| format!("“{}” katının yüksekliği bir sayı olmalı.", r.name.trim()))?;
        levels.push(ArchLevel {
            id: r.id.clone(),
            name: r.name.trim().to_owned(),
            elevation,
            height,
        });
    }
    let arch = ArchSettings {
        levels,
        detail,
        space_method,
    };
    match arch.problem() {
        Some((_, why)) => Err(why),
        None => Ok(arch),
    }
}

fn small(glyph: &'static str, enabled: bool, on: Message) -> Element<'static, Message> {
    button(container(icon(crate::icons::from_web(Some(glyph))).size(14.0)).center_x(Length::Fill))
        .on_press_maybe(enabled.then_some(on))
        .padding([4, 0])
        .width(Length::Fixed(26.0))
        .style(style::button::ghost)
        .into()
}

fn field<'a>(
    holder: &str,
    value: &'a str,
    wrong: bool,
    on: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    kentos_ui::widget::focus_ring(
        text_input(holder, value)
            .on_input(on)
            .padding([4, 7])
            .size(typography::body())
            .style(style::field::validated(wrong)),
    )
    .into()
}

/// The section.
pub(super) fn view<'a>(
    rows: &'a [Row],
    detail: Option<WallDetail>,
    on: impl Fn(usize, Edit) -> Message + Copy + 'a,
    on_add: Message,
    on_detail: impl Fn(Option<WallDetail>) -> Message + 'a,
    method: Option<kentos_contracts::SpaceMethod>,
    on_method: impl Fn(Option<kentos_contracts::SpaceMethod>) -> Message + 'a,
) -> Element<'a, Message> {
    let methods = Select::new(
        METHODS.iter().map(|(_, l)| Choice::new(*l)),
        METHODS.iter().position(|(m, _)| *m == method),
        move |i| on_method(METHODS.get(i).and_then(|(m, _)| *m)),
    )
    .searchable(false);
    let method_box = super::group::<Message>(
        "Mahallerin ölçme yöntemi",
        Column::new()
            .spacing(6)
            .push(container(methods).width(typography::scaled(200.0)))
            .push(label::caption(
                "Yeni mahalin sınırı ve alanı bu yöntemle bulunur; ofisin ya da kurumun kuralıdır. KentOS bir yöntemin yürürlükteki mevzuata uyduğunu söylemez.",
            )),
    );
    let details = Select::new(
        DETAILS.iter().map(|(_, l)| Choice::new(*l)),
        DETAILS.iter().position(|(d, _)| *d == detail),
        move |i| on_detail(DETAILS.get(i).and_then(|(d, _)| *d)),
    )
    .searchable(false);
    let detail_box = super::group::<Message>(
        "Duvarların görünüşü",
        Column::new()
            .spacing(6)
            .push(container(details).width(typography::scaled(200.0)))
            .push(label::caption(
                "Ölçeğe göre: 1:30 ve daha büyükte ayrıntılı (bileşenler ve taramaları), 1:75'e dek standart, daha küçükte şematik (dolgu). Paftanın haritası kendi ölçeğiyle çizer.",
            )),
    );
    let read_now = read(rows, detail, method);
    let head = row![
        container(label::caption("Kat")).width(Length::FillPortion(5)),
        container(label::caption("Kot (m)")).width(Length::FillPortion(3)),
        container(label::caption("Kat yüksekliği (m)")).width(Length::FillPortion(3)),
        iced::widget::space().width(Length::Fixed(3.0 * 26.0 + 2.0 * 6.0)),
    ]
    .spacing(6);
    let mut table = Column::new().spacing(4).push(head);
    for (i, r) in rows.iter().enumerate() {
        table = table.push(
            row![
                container(field(
                    "kat adı",
                    &r.name,
                    r.name.trim().is_empty(),
                    move |t| on(i, Edit::Name(t))
                ))
                .width(Length::FillPortion(5)),
                container(field(
                    "0",
                    &r.elevation,
                    !matches!(number(&r.elevation), Ok(Some(_))),
                    move |t| on(i, Edit::Elevation(t))
                ))
                .width(Length::FillPortion(3)),
                container(field(
                    "boş",
                    &r.height,
                    number(&r.height).is_err(),
                    move |t| on(i, Edit::Height(t))
                ))
                .width(Length::FillPortion(3)),
                small("chevronUp", i > 0, on(i, Edit::Up)),
                small("chevronDown", i + 1 < rows.len(), on(i, Edit::Down)),
                small("trash", true, on(i, Edit::Delete)),
            ]
            .spacing(6)
            .align_y(Center),
        );
    }
    if let Err(why) = &read_now {
        table = table.push(
            label::caption(why.clone())
                .style(style::text::danger)
                .width(Fill),
        );
    }
    let add = button(
        row![
            icon(crate::icons::from_web(Some("plus"))).size(13.0),
            label::body("Kat ekle")
        ]
        .spacing(5)
        .align_y(Center),
    )
    .on_press(on_add)
    .padding([4, 10])
    .style(style::button::secondary);
    let levels: Element<'a, Message> = if rows.is_empty() {
        Banner::info("Projenin katı yok. Duvarları katlara bağlamak için kat ekleyin; kot katın döşemesinin yüksekliğidir.").into()
    } else {
        table.into()
    };
    Column::new()
        .spacing(14)
        .push(detail_box)
        .push(method_box)
        .push(super::group::<Message>(
            "Katlar",
            Column::new().spacing(8).push(levels).push(add),
        ))
        .into()
}
