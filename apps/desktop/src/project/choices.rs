//! Proje ayarları' Datum dönüşümleri (docs/adr/0168 §3, §6; the web's
//! `datumGroup` in `ui/settings/ProjectSettingsDialog.ts`): for each of the
//! registry's three datum pairs, EPSG's way, the project's seven parameters
//! or an NTv2 grid of the device's library. What is typed is checked field
//! by field (`kentos_project::choice_form`, the shared cases'); while a form
//! has something to put right, Kaydet waits.

use std::fmt;

use iced::widget::{Column, column, row, text_input};
use iced::{Element, Fill};
use kentos_contracts::{Convention, DatumTransform};
use kentos_project::choice_form::{
    self, Form, GridRef, Method, PAIRS, PARAMETERS, Problems, datum_name, epsg_text,
};
use kentos_ui::theme::typography;
use kentos_ui::widget::Banner;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// A change to a pair's form.
#[derive(Debug, Clone)]
pub enum Edit {
    Method(Method),
    Reversed(bool),
    Name(String),
    /// One of `PARAMETERS` by its place.
    Parameter(usize, String),
    Convention(Convention),
    Grid(String),
    Accuracy(String),
}

/// The three pairs' forms as typed, and what is wrong in each.
#[derive(Debug, Clone)]
pub struct Choices {
    forms: [Form; 3],
    problems: [Problems; 3],
}

impl Choices {
    /// The forms of the project's choices: EPSG's way where it has none.
    pub fn of(choices: &[DatumTransform]) -> Self {
        Self {
            forms: PAIRS.map(|p| choice_form::form_of(p, choices)),
            problems: Default::default(),
        }
    }

    /// Takes an edit of pair `i`'s form; the draft's choices are built again
    /// from the forms that build.
    pub fn edit(
        &mut self,
        i: usize,
        edit: Edit,
        grids: &[GridRef],
        draft: &mut Vec<DatumTransform>,
    ) {
        let Some(form) = self.forms.get_mut(i) else {
            return;
        };
        match edit {
            Edit::Method(m) => {
                // A new choice starts named by its direction, for the user to finish.
                if form.method == Method::Epsg && m != Method::Epsg && form.name.trim().is_empty() {
                    let (a, b) = PAIRS[i];
                    let (from, to) = if form.reversed { (b, a) } else { (a, b) };
                    form.name = format!("{} → {}: ", datum_name(from), datum_name(to));
                }
                form.method = m;
            }
            Edit::Reversed(r) => form.reversed = r,
            Edit::Name(t) => form.name = t,
            Edit::Parameter(k, t) => {
                if let Some(p) = form.parameters.get_mut(k) {
                    *p = t;
                }
            }
            Edit::Convention(c) => form.convention = c,
            Edit::Grid(id) => form.grid = id,
            Edit::Accuracy(t) => form.accuracy = t,
        }
        draft.clear();
        for (i, pair) in PAIRS.into_iter().enumerate() {
            match choice_form::build(pair, &self.forms[i], grids) {
                Ok(choice) => {
                    draft.extend(choice);
                    self.problems[i].clear();
                }
                Err(p) => self.problems[i] = p,
            }
        }
    }

    /// Whether a form has something to put right: Kaydet waits.
    pub fn blocked(&self) -> bool {
        self.problems.iter().any(|p| !p.is_empty())
    }
}

/// A method as the segmented control writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Way(Method);

impl fmt::Display for Way {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Method::Epsg => "EPSG",
            Method::Helmert => "7 parametre",
            Method::Grid => "NTv2 ızgarası",
        })
    }
}

/// A rotation convention as the segmented control writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rule(Convention);

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Convention::PositionVector => "Konum vektörü (9606)",
            Convention::CoordinateFrame => "Koordinat çerçevesi (9607)",
        })
    }
}

/// A direction as the segmented control writes it: the pair's order or the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Way2 {
    reversed: bool,
    label: &'static str,
}

impl fmt::Display for Way2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

/// The parameters' captions, in `PARAMETERS`' order.
const CAPTIONS: [&str; 7] = [
    "ΔX (m)",
    "ΔY (m)",
    "ΔZ (m)",
    "rX (″)",
    "rY (″)",
    "rZ (″)",
    "Ölçek farkı (ppm)",
];

/// A typed field with its caption and, under it, what is wrong.
fn field<'a>(
    caption: &'a str,
    value: &'a str,
    width: f32,
    problem: Option<&'static str>,
    on: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let input = text_input("", value)
        .on_input(on)
        .padding([5, 8])
        .size(typography::body())
        .width(width)
        .style(style::field::input);
    let mut c = column![
        label::caption(caption),
        kentos_ui::widget::focus_ring(input)
    ]
    .spacing(4);
    if let Some(p) = problem {
        c = c.push(
            label::caption(p)
                .style(style::text::danger)
                .width(width.max(180.0)),
        );
    }
    c.into()
}

/// Two names of a pair's direction: the pair's order and the reverse.
fn directions(i: usize) -> [Way2; 2] {
    let (a, b) = PAIRS[i];
    let label = |x, y| -> &'static str {
        // The six names, made once.
        match (datum_name(x), datum_name(y)) {
            ("ED50", "TUREF") => "ED50 → TUREF",
            ("TUREF", "ED50") => "TUREF → ED50",
            ("ED50", "WGS 84") => "ED50 → WGS 84",
            ("WGS 84", "ED50") => "WGS 84 → ED50",
            ("TUREF", "WGS 84") => "TUREF → WGS 84",
            _ => "WGS 84 → TUREF",
        }
    };
    [
        Way2 {
            reversed: false,
            label: label(a, b),
        },
        Way2 {
            reversed: true,
            label: label(b, a),
        },
    ]
}

impl App {
    /// The grids a datum choice may name: the device's library, and the
    /// ones the project's choices name already that it does not have.
    pub(crate) fn choice_grids(&self) -> Vec<GridRef> {
        let mut out: Vec<GridRef> = self
            .grids
            .entries
            .iter()
            .map(|e| GridRef {
                id: e.id.clone(),
                file: e.file.clone(),
                size: e.size,
            })
            .collect();
        if let Some(doc) = &self.document {
            for g in doc
                .settings()
                .datum_transforms
                .iter()
                .filter_map(|t| t.grid.as_ref())
            {
                if !out.iter().any(|r| r.id == g.id) {
                    out.push(GridRef {
                        id: g.id.clone(),
                        file: g.file.clone(),
                        size: g.size,
                    });
                }
            }
        }
        out
    }
}

/// Datum dönüşümleri: each pair's way, and the fields of the one chosen.
pub fn view<'a>(
    choices: &'a Choices,
    grids: Vec<GridRef>,
    on: impl Fn(usize, Edit) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let mut list = Column::new().spacing(14).push(label::caption(
        "Kayıttaki datumlar arasında EPSG'nin yolu yerine projenin seçimi: yedi parametre ya da bu cihazdaki bir NTv2 ızgarası. Seçim yalnız kendi çiftini değiştirir; ters yönü aynı dönüşümün tersidir.",
    ));
    if choices.blocked() {
        list = list.push(Banner::warning(
            "Datum dönüşümlerinde düzeltilecek alan var; düzeltilene dek Kaydet kapalı.",
        ));
    }
    for (i, form) in choices.forms.iter().enumerate() {
        let (a, b) = PAIRS[i];
        let problems = &choices.problems[i];
        let problem = |key: &str| problems.get(key).copied();
        let ways = Segmented::new(
            [Way(Method::Epsg), Way(Method::Helmert), Way(Method::Grid)],
            Way(form.method),
            {
                let on = on.clone();
                move |w| on(i, Edit::Method(w.0))
            },
        );
        let head = row![
            label::body(format!("{} ↔ {}", datum_name(a), datum_name(b))).width(Fill),
            ways
        ]
        .spacing(16)
        .align_y(iced::Center);
        let mut pair = column![head].spacing(10);
        if form.method == Method::Epsg {
            pair = pair.push(label::caption(format!(
                "EPSG'nin yolu: {}",
                epsg_text(PAIRS[i])
            )));
            list = list.push(pair);
            continue;
        }
        let ways = directions(i);
        let direction = Segmented::new(ways, ways[usize::from(form.reversed)], {
            let on = on.clone();
            move |w: Way2| on(i, Edit::Reversed(w.reversed))
        });
        pair = pair.push(
            row![
                field("Ad", &form.name, 300.0, problem("name"), {
                    let on = on.clone();
                    move |t| on(i, Edit::Name(t))
                }),
                column![label::caption("Yön"), direction].spacing(4),
            ]
            .spacing(16),
        );
        let accuracy = field(
            "Doğruluk (m)",
            &form.accuracy,
            110.0,
            problem("accuracy"),
            {
                let on = on.clone();
                move |t| on(i, Edit::Accuracy(t))
            },
        );
        if form.method == Method::Helmert {
            let parameter = |k: usize| {
                let on = on.clone();
                field(
                    CAPTIONS[k],
                    &form.parameters[k],
                    if k == 6 { 140.0 } else { 110.0 },
                    problem(PARAMETERS[k]),
                    move |t| on(i, Edit::Parameter(k, t)),
                )
            };
            pair = pair
                .push(row![parameter(0), parameter(1), parameter(2)].spacing(12))
                .push(row![parameter(3), parameter(4), parameter(5), parameter(6)].spacing(12));
            let rule = Segmented::new(
                [
                    Rule(Convention::PositionVector),
                    Rule(Convention::CoordinateFrame),
                ],
                Rule(form.convention),
                {
                    let on = on.clone();
                    move |r: Rule| on(i, Edit::Convention(r.0))
                },
            );
            pair = pair.push(
                row![
                    column![label::caption("Dönüklüklerin kuralı"), rule].spacing(4),
                    accuracy
                ]
                .spacing(16),
            );
        } else {
            let selected = grids.iter().position(|g| g.id == form.grid);
            let ids: Vec<String> = grids.iter().map(|g| g.id.clone()).collect();
            let pick = Select::new(
                grids.iter().map(|g| {
                    Choice::new(g.file.clone()).detail(g.id.chars().take(12).collect::<String>())
                }),
                selected,
                {
                    let on = on.clone();
                    move |k| on(i, Edit::Grid(ids.get(k).cloned().unwrap_or_default()))
                },
            )
            .placeholder("Izgara seçin…");
            let mut grid = column![
                label::caption("Izgara"),
                iced::widget::container(pick).width(300)
            ]
            .spacing(4);
            if let Some(p) = problem("grid") {
                grid = grid.push(label::caption(p).style(style::text::danger));
            }
            pair = pair.push(row![grid, accuracy].spacing(16));
        }
        list = list.push(pair);
    }
    super::group("Datum dönüşümleri", list)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proje ayarları' Datum dönüşümleri: ED50–TUREF by seven parameters
    /// with a translation still to type (Kaydet waits), ED50–WGS 84 by a
    /// grid of the library, TUREF–WGS 84 EPSG's way; light at 1440 × 900,
    /// dark at 1100 × 650; `.run/shots/datum-*` (the web's: `(cd apps/web &&
    /// node scripts/e2e/shots.mjs datums)`):
    ///
    /// ```text
    /// cargo test -p kentos-desktop project::choices::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use crate::files_testing::{app_with_drawing, drive, find_text};
        use crate::project::{SettingsEvent, settings_message};
        use iced::Size;
        use kentos_ui::snapshot::{Input, Snapshot};

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let tr = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/geodesy/v1/ntv2/tr.gsb");
        for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            app.grids.library = Some(crate::grids::Library::at(crate::files_testing::scratch(
                "datum-resim",
            )));
            let _ = app.update(Message::Run("crs.set"));
            let task = app.update(settings_message(SettingsEvent::Grid(
                crate::grids::Event::Picked(Some(tr.clone())),
            )));
            drive(&mut app, task);
            let send = |app: &mut App, i: usize, e: Edit| {
                let _ = app.update(settings_message(SettingsEvent::Choice(i, e)));
            };
            send(&mut app, 0, Edit::Method(Method::Helmert));
            send(&mut app, 0, Edit::Name("ED50 → TUREF: Bölge 7".to_owned()));
            for (k, v) in [
                "-158.785", "-109.965", "", "1.4275", "-3.0873", "0.5505", "-5.1814",
            ]
            .into_iter()
            .enumerate()
            {
                send(&mut app, 0, Edit::Parameter(k, v.to_owned()));
            }
            send(&mut app, 0, Edit::Convention(Convention::CoordinateFrame));
            send(&mut app, 0, Edit::Accuracy("0.3".to_owned()));
            send(&mut app, 1, Edit::Method(Method::Grid));
            let id = app
                .grids
                .entries
                .first()
                .map(|e| e.id.clone())
                .unwrap_or_default();
            send(&mut app, 1, Edit::Grid(id));
            send(&mut app, 1, Edit::Accuracy("0.5".to_owned()));
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if let Some(list) = find_text(&mut snapshot, &app, "İkinci koordinat sistemi") {
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::Scroll(list.center(), -9.0),
                );
            }
            let file = out.join(format!("datum-donusumleri-{theme}-{w}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
