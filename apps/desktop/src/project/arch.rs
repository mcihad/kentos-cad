//! Proje ayarları › Mimari (docs/adr/0339 §3, §6): the project's levels, a row each (Ad, Kot, Kat yüksekliği; up,
//! down, delete; Kat ekle), and the detail the walls are drawn at (Ölçeğe göre, Şematik, Standart, Ayrıntılı). What
//! does not hold is said under the rows and Kaydet waits; Kaydet writes them through `cad.settings.set`'s `arch`,
//! which keeps a level a wall names.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Length};
use kentos_commands::{ArchLevel, ArchSettings, WallDetail};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::Banner;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::Message;

/// A level the rows take away that objects or a sheet's map name (docs/adr/0720 §3): why it stays, under the
/// levels, with Nesneleri seç when objects name it; nothing otherwise. Kaydet waits meanwhile.
pub fn in_use_view<'a>(
    in_use: Option<&kentos_commands::CommandError>,
    select: Message,
) -> Element<'a, Message> {
    let Some(e) = in_use else {
        return Column::new().into();
    };
    // Said before Kaydet: it cannot go, rather than it did not.
    let banner = Banner::warning(e.message.replacen("; kaldırılmadı.", "; kaldırılamaz.", 1));
    if e.uids.is_empty() {
        banner.into()
    } else {
        banner
            .action(format!("Nesneleri seç ({})", e.uids.len()), select)
            .into()
    }
}

/// A level as typed.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// Its id: kept from the project's, a new one's made when it is added.
    pub id: String,
    pub name: String,
    pub elevation: String,
    pub height: String,
    pub prefix: String,
}

#[derive(Debug, Clone)]
pub enum Edit {
    Name(String),
    Elevation(String),
    Height(String),
    Prefix(String),
    Up,
    Down,
    Delete,
}

/// The detail list: by the scale, then each.
/// The spaces' measurement methods Mimari offers (docs/adr/0342 §1): the office's profile; none: net.
const METHODS: [(Option<kentos_commands::SpaceMethod>, &str); 3] = [
    (None, "Net (iç yüzler)"),
    (
        Some(kentos_commands::SpaceMethod::Centerline),
        "Duvar ekseni",
    ),
    (
        Some(kentos_commands::SpaceMethod::Gross),
        "Brüt (dış yüzler)",
    ),
];

const DETAILS: [(Option<WallDetail>, &str); 4] = [
    (None, "Otomatik"),
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
                prefix: a
                    .space_number_prefixes
                    .get(&l.id)
                    .cloned()
                    .unwrap_or_default(),
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
        prefix: String::new(),
    }
}

pub fn edit(rows: &mut Vec<Row>, i: usize, e: Edit) {
    let n = rows.len();
    let Some(r) = rows.get_mut(i) else { return };
    match e {
        Edit::Name(t) => r.name = t,
        Edit::Elevation(t) => r.elevation = t,
        Edit::Height(t) => r.height = t,
        Edit::Prefix(t) => r.prefix = t,
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
    space_method: Option<kentos_commands::SpaceMethod>,
    choices: Option<&ArchSettings>,
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
        phase_view: None,
        // The level and view the drawing opens with are not this form's (docs/adr/0720 §1).
        active_level: None,
        level_view: None,
        space_names: choices.map_or_else(Vec::new, |a| {
            a.space_names.iter().map(|s| s.trim().to_owned()).collect()
        }),
        space_usages: choices.map_or_else(Vec::new, |a| {
            a.space_usages.iter().map(|s| s.trim().to_owned()).collect()
        }),
        space_number_prefixes: rows
            .iter()
            .filter_map(|r| {
                let prefix = r.prefix.trim();
                (!prefix.is_empty()).then(|| (r.id.clone(), prefix.to_owned()))
            })
            .collect(),
        // The audit's exceptions are Denetim's, not this form's (docs/adr/0732 §3).
        audit_exceptions: Vec::new(),
        // The door and window types are their window's (docs/adr/0772 §1).
        opening_types: choices.map_or_else(Vec::new, |a| a.opening_types.clone()),
        // So is the building's program (docs/adr/0773).
        program: Default::default(),
        // So are the room modules (docs/adr/0775).
        modules: choices.map_or_else(Vec::new, |a| a.modules.clone()),
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
    kentos_ui::widget::text_field(holder, value)
        .on_input(on)
        .style(style::field::validated(wrong))
        .into()
}

/// The section.
pub(super) fn view<'a>(
    rows: &'a [Row],
    detail: Option<WallDetail>,
    on: impl Fn(usize, Edit) -> Message + Copy + 'a,
    on_add: Message,
    on_detail: impl Fn(Option<WallDetail>) -> Message + 'a,
    method: Option<kentos_commands::SpaceMethod>,
    on_method: impl Fn(Option<kentos_commands::SpaceMethod>) -> Message + 'a,
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
                "Otomatik: modelde yaklaştıkça duvar bileşenleri görünür. 1:30 ve daha büyükte ayrıntılı, 1:75'e dek standart, daha küçükte şematik gösterilir. Pafta ve PDF kendi ölçeğini kullanır; taramaların çizim ölçeği değişmez.",
            )),
    );
    let read_now = read(rows, detail, method, None);
    let head = kentos_ui::widget::table::band(
        row![
            container(kentos_ui::widget::table::title("Kat"))
                .padding([0, 8])
                .width(Length::FillPortion(5)),
            container(kentos_ui::widget::table::title("Kot (m)"))
                .padding([0, 8])
                .width(Length::FillPortion(3)),
            container(kentos_ui::widget::table::title("Kat yüksekliği (m)"))
                .padding([0, 8])
                .width(Length::FillPortion(3)),
            iced::widget::space().width(Length::Fixed(3.0 * 26.0 + 2.0 * 6.0)),
        ]
        .spacing(6),
    );
    let mut table = Column::new().spacing(4).push(head);
    for (i, r) in rows.iter().enumerate() {
        table = table.push(
            Column::new()
                .spacing(4)
                .push(
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
                )
                .push(
                    row![
                        label::caption("Mahal numara öneki"),
                        container(field(
                            "Z- gibi; boş bırakılabilir",
                            &r.prefix,
                            false,
                            move |t| on(i, Edit::Prefix(t))
                        ))
                        .width(Fill),
                    ]
                    .spacing(8)
                    .align_y(Center),
                ),
        );
        if let Some(first) = read_now
            .as_ref()
            .ok()
            .and_then(|a| a.space_first_number(&r.id))
        {
            table =
                table.push(label::caption(format!("Seçilebilir başlangıç: {first}")).width(Fill));
        }
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
    .padding(kentos_ui::theme::metrics::action())
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

#[derive(Clone, Copy, Debug)]
pub enum SpaceList {
    Names,
    Usages,
}

#[derive(Clone, Debug)]
pub enum ChoiceEdit {
    Add,
    Value(usize, String),
    Up(usize),
    Down(usize),
    Delete(usize),
}

#[derive(Clone, Debug)]
pub struct ChoiceEvent {
    pub list: SpaceList,
    pub edit: ChoiceEdit,
}

pub fn edit_choices(arch: &mut ArchSettings, event: ChoiceEvent) {
    let values = match event.list {
        SpaceList::Names => &mut arch.space_names,
        SpaceList::Usages => &mut arch.space_usages,
    };
    match event.edit {
        ChoiceEdit::Add if values.len() < 256 => values.push(String::new()),
        ChoiceEdit::Value(i, value) if i < values.len() => values[i] = value,
        ChoiceEdit::Up(i) if i > 0 && i < values.len() => values.swap(i, i - 1),
        ChoiceEdit::Down(i) if i < values.len() && i + 1 < values.len() => values.swap(i, i + 1),
        ChoiceEdit::Delete(i) if i < values.len() => {
            values.remove(i);
        }
        _ => {}
    }
}

/// Separate suggestions retain the existing free-text fields and source values.
pub(super) fn choices_view<'a>(
    arch: Option<&'a ArchSettings>,
    on: impl Fn(ChoiceEvent) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let mut out=Column::new().spacing(14).push(label::caption(
        "Listeler yeni veya seçili mahalda seçim kolaylığı sağlar. Serbest metin yazılabilir; liste ve önek değişiklikleri mevcut mahalleri değiştirmez. Kat öneki araçta açıkça seçilir."
    ).width(Fill));
    for (kind, title, values) in [
        (
            SpaceList::Names,
            "Mahal adları",
            arch.map_or(&[][..], |a| a.space_names.as_slice()),
        ),
        (
            SpaceList::Usages,
            "Kullanım türleri",
            arch.map_or(&[][..], |a| a.space_usages.as_slice()),
        ),
    ] {
        let mut list = Column::new().spacing(6);
        let mut seen = std::collections::BTreeSet::new();
        for (i, value) in values.iter().enumerate() {
            let emit = move |edit| on(ChoiceEvent { list: kind, edit });
            let trimmed = value.trim();
            let invalid = trimmed.is_empty()
                || trimmed.chars().count() > 256
                || trimmed.chars().any(char::is_control)
                || !seen.insert(trimmed);
            list = list.push(
                row![
                    container(field("değer", value, invalid, move |v| {
                        emit(ChoiceEdit::Value(i, v))
                    }))
                    .width(Fill),
                    small("chevronUp", i > 0, emit(ChoiceEdit::Up(i))),
                    small(
                        "chevronDown",
                        i + 1 < values.len(),
                        emit(ChoiceEdit::Down(i))
                    ),
                    small("trash", true, emit(ChoiceEdit::Delete(i)))
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        // Keep the reason beside the affected list, including in a compact window.
        let normalized = values.iter().map(|s| s.trim().to_owned()).collect();
        let probe = match kind {
            SpaceList::Names => ArchSettings {
                space_names: normalized,
                ..Default::default()
            },
            SpaceList::Usages => ArchSettings {
                space_usages: normalized,
                ..Default::default()
            },
        };
        if let Some((_, why)) = probe.problem() {
            list = list.push(label::caption(why).style(style::text::danger).width(Fill));
        }
        list = list.push(
            button(label::body("Değer ekle"))
                .on_press_maybe((values.len() < 256).then(|| {
                    on(ChoiceEvent {
                        list: kind,
                        edit: ChoiceEdit::Add,
                    })
                }))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary),
        );
        out = out.push(super::group::<Message>(title, list));
    }
    out.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> ArchSettings {
        serde_json::from_value(serde_json::json!({
            "levels":[{"id":"zemin","name":"Zemin kat","elevation":0.}],
            "spaceNames":["Mutfak","mutfak"],"spaceUsages":["Konut"],
            "spaceNumberPrefixes":{"zemin":"Z-"}
        }))
        .unwrap()
    }

    #[test]
    fn project_form_preserves_exact_choices_and_only_an_explicit_prefix_edit_changes_the_start() {
        let a = project();
        let mut form = rows(Some(&a));
        assert_eq!(read(&form, None, None, Some(&a)).unwrap(), a);
        edit(&mut form, 0, Edit::Name("Giriş katı".into()));
        let before = read(&form, None, None, Some(&a)).unwrap();
        assert_eq!(before.space_first_number("zemin").as_deref(), Some("Z-001"));
        assert_eq!(before.space_names, a.space_names);
        edit(&mut form, 0, Edit::Prefix(" G- ".into()));
        let changed = read(&form, None, None, Some(&a)).unwrap();
        assert_eq!(
            changed.space_first_number("zemin").as_deref(),
            Some("G-001")
        );
        assert_eq!(changed.space_usages, a.space_usages);
        edit(&mut form, 0, Edit::Delete);
        let cleared = read(&form, None, None, Some(&a)).unwrap();
        assert!(cleared.space_number_prefixes.is_empty());
        assert_eq!(cleared.space_names, a.space_names);
    }

    #[test]
    fn list_draft_keeps_order_rejects_duplicates_and_ignores_stale_indices() {
        let mut a = project();
        let change = |a: &mut ArchSettings, edit| {
            edit_choices(
                a,
                ChoiceEvent {
                    list: SpaceList::Names,
                    edit,
                },
            )
        };
        for edit in [
            ChoiceEdit::Value(usize::MAX, "lost".into()),
            ChoiceEdit::Up(usize::MAX),
            ChoiceEdit::Down(usize::MAX),
            ChoiceEdit::Delete(usize::MAX),
        ] {
            change(&mut a, edit);
            assert_eq!(a, project());
        }
        change(&mut a, ChoiceEdit::Add);
        assert!(read(&rows(Some(&a)), None, None, Some(&a)).is_err());
        change(&mut a, ChoiceEdit::Value(2, "Çalışma".into()));
        change(&mut a, ChoiceEdit::Up(2));
        assert_eq!(a.space_names, ["Mutfak", "Çalışma", "mutfak"]);
        change(&mut a, ChoiceEdit::Value(1, " Mutfak ".into()));
        assert!(read(&rows(Some(&a)), None, None, Some(&a)).is_err());
        change(&mut a, ChoiceEdit::Delete(1));
        assert_eq!(a, project());
    }
}
