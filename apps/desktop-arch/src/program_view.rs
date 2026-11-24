//! Bina programı's look (docs/adr/0773, 0784, 8031, 8072 §1; DESIGN.md §7.9 Liste ve ayrıntı): a one-line lead, the
//! parts as a segmented choice, and in Binalar, Bölümler and Hedefler the list on the left and the chosen item's form on
//! the right, every box on one label column, its own actions beside it and its removal a red-outlined button. The
//! settings of a part stand above its list on the same label column: Ortak alan payı and Arsa payı paydası over
//! Bölümler, Hedeflerin sürümü over Hedefler. Durum is the check's answer in the kit's tables. The body has one height
//! whichever part is shown; the footer keeps the window's own: Denetle, Vazgeç and Kaydet.
//!
//! ```text
//! ┌ Bina programı ───────────────────────────────────────────────────────────────────────┐
//! │ Binaları, bölümleri ve mahal hedeflerini tutar; Denetle çizimi bunlara göre ölçer.     │
//! │ [Binalar | Bölümler | Hedefler | Durum]                                                │
//! │ Ortak alan payı  [Net alana göre ▾]  Arsa payı paydası [Parsel alanı] m²               │
//! │ ┌ No  Ad          Tür       ┐│ 1 Daire 1                               [Bölümü sil]   │
//! │ │ 1   Daire 1     Bağımsız  ││ No              [1    ]                                │
//! │ │ 2   Daire 2     Bağımsız  ││ Adı             [Daire 1               ]               │
//! │ │ O   Ortak alan  Ortak     ││ Türü            [Bağımsız bölüm ▾]                     │
//! │ ├───────────────────────────┤│ Bina            [A Blok ▾]                             │
//! │ │ [Bölüm ekle]              ││ Mahalleri ─────────────────────────────────            │
//! │ └───────────────────────────┘│                 [Seçilenleri ver]                      │
//! │ [Denetle] 7 bulgu…                                              [Vazgeç] [Kaydet]     │
//! └───────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length};
use kentos_commands::{ProgramBuilding, ProgramChecked, ShareRule, UnitKind};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{Column as Col, Row, Table};
use kentos_ui::widget::{Choice, Dialog as Frame, Form, ListDetails, Pairs, Segmented, Select, text_field};
use crate::arch::ArchState;

use super::{
    ADD_BRIEF, ADD_BUILDING, ADD_UNIT, ADD_WING, CANCEL, CHECK, Event, GIVE, SAVE, SHOW_ON_PLAN, STOP, TITLE, Tab,
    UnitRow, Window, fixed2, msg,
};
use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};

/// The window's lead: what it is for, in one sentence.
const LEAD: &str = "Binaları, bölümleri ve mahal hedeflerini tutar; Denetle çizimi bunlara göre ölçer ve sonucu Durum'da \
                    gösterir.";
/// The body's height (12 px text), the same in every part.
const BODY: f32 = 420.0;
/// A part's settings row above its list.
const SETTINGS: f32 = 44.0;
/// The label column of the forms (the kit's own).
const LABELS: f32 = 110.0;
/// The list's width.
const LIST: f32 = 270.0;

/// A removal: the red-outlined button (DESIGN.md §7.9).
fn remove<'a>(words: &'a str, on: Message) -> Element<'a, Message> {
    button(label::body(words))
        .on_press(on)
        .padding(kentos_ui::theme::metrics::action())
        .style(style::button::danger_outline)
        .into()
}

/// The chosen item's head: its name and its removal on the right.
fn head<'a>(name: String, removal: Element<'a, Message>) -> Element<'a, Message> {
    row![label::strong(name), space::horizontal().width(Fill), removal].spacing(8).align_y(Center).into()
}

/// An empty list's details: what to do, step by step (DESIGN.md §7.5).
fn steps<'a>(title: &'a str, lead: &'a str, list: &[&'a str]) -> Element<'a, Message> {
    let mut out = Column::new().spacing(10).push(label::strong(title)).push(label::caption(lead));
    for (k, s) in list.iter().enumerate() {
        out = out.push(
            row![
                container(label::strong((k + 1).to_string())).width(scaled(18.0)),
                container(label::body(*s)).width(Fill)
            ]
            .spacing(6),
        );
    }
    out.into()
}

/// A level's name in the project; its id when the project does not have it.
fn level_name(app: &App, id: &str) -> String {
    app.document
        .as_ref()
        .and_then(|d| d.model.settings().arch.as_ref())
        .and_then(|a| a.levels.iter().find(|l| l.id == id).map(|l| l.name.clone()))
        .unwrap_or_else(|| id.to_owned())
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait ProgramViewApp {
    fn program_levels(&self) -> Vec<(String, String)>;
    fn program_view(&self) -> Element<'_, Message>;
    fn buildings_list<'a>(&self, w: &'a Window) -> Element<'a, Message>;
    fn level_checks<'a>(&self, chosen: &[String], on: impl Fn(String) -> Message + 'a) -> Element<'a, Message>;
    fn building_details<'a>(&'a self, w: &'a Window) -> Element<'a, Message>;
    fn unit_details<'a>(&'a self, w: &'a Window) -> Element<'a, Message>;
    fn status_view<'a>(&'a self, w: &'a Window) -> Element<'a, Message>;
    fn spaces_table<'a>(&'a self, c: &'a ProgramChecked) -> Element<'a, Message>;
}

impl ProgramViewApp for App {
    /// The project's levels: id and name, in its order.
    fn program_levels(&self) -> Vec<(String, String)>{
        self.document
            .as_ref()
            .and_then(|d| d.model.settings().arch.as_ref())
            .map(|a| a.levels.iter().map(|l| (l.id.clone(), l.name.clone())).collect())
            .unwrap_or_default()
    }

    fn program_view(&self) -> Element<'_, Message>{
        let Some(w) = &self.modules.arch().program_window else {
            return space::horizontal().into();
        };
        let tabs = Segmented::new([Tab::Buildings, Tab::Units, Tab::Briefs, Tab::Status], w.tab, |t| {
            msg(Event::Tab(t))
        });
        let body: Element<'_, Message> = match w.tab {
            Tab::Buildings => ListDetails::new(self.buildings_list(w), self.building_details(w))
                .list_foot(words::secondary(ADD_BUILDING, Some(msg(Event::AddBuilding))))
                .list_width(LIST)
                .height(BODY)
                .into(),
            Tab::Units => column![
                units_settings(w),
                ListDetails::new(units_list(w), self.unit_details(w))
                    .list_foot(words::secondary(ADD_UNIT, Some(msg(Event::AddUnit))))
                    .list_width(LIST)
                    .height(BODY - SETTINGS)
            ]
            .spacing(0)
            .into(),
            Tab::Briefs => column![
                briefs_settings(w),
                ListDetails::new(briefs_list(w), brief_details(w))
                    .list_foot(words::secondary(ADD_BRIEF, Some(msg(Event::AddBrief))))
                    .list_width(LIST)
                    .height(BODY - SETTINGS)
            ]
            .spacing(0)
            .into(),
            Tab::Status => container(scrollable(container(self.status_view(w)).padding(iced::Padding::ZERO.right(14))))
                .height(scaled(BODY))
                .into(),
        };
        let mut foot = row![].spacing(8).align_y(Center);
        match &w.running {
            Some(r) => {
                let share = (r.done * 100).checked_div(r.total).unwrap_or(0);
                foot = foot
                    .push(words::secondary(STOP, Some(msg(Event::Stop))))
                    .push(words::text_line(Kind::Info, format!("Denetleniyor… %{share}")));
            }
            None => {
                foot = foot.push(words::secondary(CHECK, Some(msg(Event::Check))));
                if let Some((kind, said)) = &w.said {
                    foot = foot.push(container(words::text_line(*kind, said.clone())).width(Length::Shrink));
                }
            }
        }
        let foot = foot
            .push(space::horizontal().width(Fill))
            .push(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .push(words::primary(SAVE, Some(msg(Event::Save))));
        // The body scrolls when the window is short (DESIGN.md §7.9): the head and the footer stay in view.
        let window = Frame::new(TITLE)
            .push(label::caption(LEAD))
            .push(tabs)
            .scroll(body)
            .footer(foot)
            .width(1000.0)
            .modal(msg(Event::Close));
        if !w.closing {
            return window;
        }
        use crate::unsaved_close::Answer;
        let question = kentos_ui::widget::unsaved(
            TITLE,
            msg(Event::Answer(Answer::Discard)),
            msg(Event::Answer(Answer::Stay)),
            Some(msg(Event::Answer(Answer::Save))),
        );
        iced::widget::stack![window, question].into()
    }

    // ── Binalar ───────────────────────────────────────────────────────────────────────────────────────────────────

    /// The buildings: each one's name, code and how many levels it spans.
    fn buildings_list<'a>(&self, w: &'a Window) -> Element<'a, Message>{
        let mut t = Table::new([
            Col::new("Bina").width(Fill),
            Col::new("Kod").width(scaled(44.0)),
            Col::new("Kat").width(scaled(36.0)).align_right(),
        ])
        .empty("Programda bina yok.");
        for (i, b) in w.buildings.iter().enumerate() {
            let name = if b.name.trim().is_empty() { "Adsız bina".to_owned() } else { b.name.clone() };
            let levels = if b.levels.is_empty() { "—".to_owned() } else { b.levels.len().to_string() };
            t = t.push(
                Row::new([
                    label::body(name).into(),
                    label::body(b.code.clone().unwrap_or_else(|| "—".into())).into(),
                    label::body(levels).into(),
                ])
                .selected(i == w.building)
                .on_press(msg(Event::ChooseBuilding(i))),
            );
        }
        t.into()
    }

    /// The levels as boxes in rows of three, each toggling through `on`.
    fn level_checks<'a>(&self, chosen: &[String], on: impl Fn(String) -> Message + 'a) -> Element<'a, Message>{
        let levels = self.program_levels();
        if levels.is_empty() {
            return label::caption("Projede kat yok: Proje ayarları › Mimari'de ekleyin.").into();
        }
        let mut out = Column::new().spacing(6);
        for part in levels.chunks(3) {
            let mut r = row![].spacing(16).align_y(Center);
            for (id, name) in part {
                r = r.push(container(words::check(chosen.contains(id), name.clone(), Some(on(id.clone())))).width(scaled(150.0)));
            }
            out = out.push(r);
        }
        out.into()
    }

    /// The chosen building: its name, code, use, ±0.00 and levels; the shared levels it spans and the building each
    /// counts in; its units; its wings in a table, the chosen wing's name and levels under it.
    fn building_details<'a>(&'a self, w: &'a Window) -> Element<'a, Message>{
        let Some(b) = w.buildings.get(w.building) else {
            return steps(
                "Programda bina yok",
                "Tek binalı projede bina gerekmez; birden çok blokta her blok bir binadır.",
                &[
                    "Soldaki Bina ekle'ye basın.",
                    "Binaya ad, kod (EK-2: A1, B2) ve katlarını verin.",
                    "Bölümler'de her bölümü binasına bağlayın.",
                ],
            );
        };
        let i = w.building;
        let users = w.users_of(&b.id);
        let mut form = Form::new()
            .label_width(scaled(LABELS))
            .field(
                "Adı",
                text_field("A Blok", &b.name).on_input(move |t| msg(Event::BuildingName(i, t))).width(scaled(260.0)),
            )
            .field(
                "Bina kodu",
                text_field("—", b.code.as_deref().unwrap_or(""))
                    .on_input(move |t| msg(Event::BuildingCode(i, t)))
                    .width(scaled(80.0)),
            )
            .help("EK-2: iki büyük harf ya da rakam, her binada ayrı; tek binalı projede 00.")
            .field(
                "Kullanım türü",
                text_field("—", b.usage.as_deref().unwrap_or(""))
                    .on_input(move |t| msg(Event::BuildingUsage(i, t)))
                    .width(scaled(260.0)),
            )
            .field(
                "±0,00 farkı (m)",
                text_field("0", w.zeros.get(i).map_or("", String::as_str))
                    .on_input(move |t| msg(Event::BuildingZero(i, t)))
                    .width(scaled(100.0)),
            )
            .help("Projenin ±0,00'ına göre; boşsa projeninki.")
            .field("Bölge", {
                // Its own Bölge (docs/adr/8144): the project's unless the block stands on a decision of another use.
                let choices: Vec<(Option<kentos_commands::Zone>, &'static str)> = std::iter::once((None, "Projeninki"))
                    .chain(kentos_commands::Zone::ALL.into_iter().map(|z| (Some(z), z.label())))
                    .collect();
                let at = choices.iter().position(|(z, _)| *z == b.zone);
                let pick = Select::new(choices.iter().map(|(_, l)| Choice::new(*l)), at, move |k| {
                    let zone = std::iter::once(None).chain(kentos_commands::Zone::ALL.into_iter().map(Some)).nth(k);
                    msg(Event::BuildingZone(i, zone.flatten()))
                })
                .searchable(false);
                container(pick).width(scaled(330.0))
            })
            .help("Bloklar planın farklı kullanımlı kararlarında duruyorsa: binanın katları kendi bölgesiyle okunur \
                   (Planlı Alanlar İmar Yönetmeliği 28/1). Projeninki: Proje ayarları › Mimari › İmar durumu.")
            .field("Taşıyıcı sistem", {
                // Its own structural system (docs/adr/8145): the project's unless the block is built otherwise.
                let choices: Vec<(Option<kentos_commands::StructuralSystem>, &'static str)> =
                    std::iter::once((None, "Projeninki"))
                        .chain(kentos_commands::StructuralSystem::ALL.into_iter().map(|x| (Some(x), x.label())))
                        .collect();
                let at = choices.iter().position(|(x, _)| *x == b.structure.map(|x| x.system));
                let pick = Select::new(choices.iter().map(|(_, l)| Choice::new(*l)), at, move |k| {
                    let system =
                        std::iter::once(None).chain(kentos_commands::StructuralSystem::ALL.into_iter().map(Some)).nth(k);
                    msg(Event::BuildingStructure(i, system.flatten()))
                })
                .searchable(false);
                let mut col = Column::new().spacing(6).push(container(pick).width(scaled(330.0)));
                if let Some(x) = b.structure.filter(|x| x.system == kentos_commands::StructuralSystem::CutStoneMasonry) {
                    col = col.push(
                        kentos_ui::widget::Switch::new(x.traditional, move |on| msg(Event::BuildingTraditional(i, on)))
                            .label("Geleneksel mimari öğeler barındırıyor"),
                    );
                }
                col
            })
            .help("Blok öbürlerinden başka türlü yapılıyorsa: geleneksel kesme taş yığma bloğun katları 28/1-d, taşlık, \
                   dış sofa ve eyvanı 22/1-m, kalın taş duvarları 5/8 ve 20/8-j ile kendi kat sayısıyla okunur. \
                   Projeninki: Proje ayarları › Mimari › Taşıyıcı sistem.")
            .field("Katları", self.level_checks(&b.levels, move |l| msg(Event::BuildingLevel(i, l))))
            .help("Boşsa her katta olabilir.")
            .field(
                "Bölümleri",
                label::body(if users.is_empty() { "Yok".to_owned() } else { users.join(", ") }),
            );
        // Its own plan notes (docs/adr/8163): the plan's values for the area of the parcel it stands in.
        let notes = w.notes.get(i).map_or(&[][..], Vec::as_slice);
        form = form
            .section("Plan notları")
            .wide(crate::project::plan_notes::table(
                notes,
                move |k, e| msg(Event::BuildingPlanNote(i, k, e)),
                msg(Event::AddBuildingPlanNote(i)),
            ))
            .wide(label::caption(
                "Bina kademe hattının bir yanında duruyorsa (Planlı Alanlar İmar Yönetmeliği 4/1-mm) planın o alana \
                 verdiği değerler: yazılan konu Proje ayarları › Mimari › Plan notları'ndaki aynı konunun yerine geçer, \
                 yazılmayanlar projeninkidir.",
            ));
        // The levels it shares with other buildings (docs/adr/8072 §1): the building each counts in.
        let mut shared_head = false;
        for (id, name) in self.program_levels() {
            if !b.levels.contains(&id) || w.buildings.iter().filter(|x| x.levels.contains(&id)).count() < 2 {
                continue;
            }
            let owners: Vec<&ProgramBuilding> = w.buildings.iter().filter(|x| x.levels.contains(&id)).collect();
            let chosen = w.shared.iter().find(|sh| sh.level == id).map(|sh| sh.building.clone());
            let at = owners.iter().position(|x| Some(&x.id) == chosen.as_ref());
            let ids: Vec<String> = owners.iter().map(|x| x.id.clone()).collect();
            let level = id.clone();
            let pick = Select::new(owners.iter().map(|x| Choice::new(x.name.clone())), at, move |k| {
                msg(Event::SharedBuilding(level.clone(), ids.get(k).cloned().unwrap_or_default()))
            })
            .searchable(false);
            if !shared_head {
                form = form.section("Ortak katlar");
                shared_head = true;
            }
            form = form
                .field(name, container(pick).width(scaled(220.0)))
                .help("Alanı bir kez, seçilen binada sayılır.")
                .warning(at.is_none().then_some("Seçilmedi: alanı hangi binada sayılacağı belli değil."));
        }
        // Its wings: a table, the chosen one's form under it.
        let mut wings = Table::new([
            Col::new("Kanat").width(Fill),
            Col::new("Katları").width(scaled(200.0)),
            Col::new("Bölümleri").width(scaled(110.0)),
        ])
        .empty("Kanadı yok.");
        for (k, x) in b.wings.iter().enumerate() {
            let levels: Vec<String> = x.levels.iter().map(|l| level_name(self, l)).collect();
            let wing_users = w.users_of(&x.id);
            wings = wings.push(
                Row::new([
                    label::body(x.name.clone()).into(),
                    label::body(if levels.is_empty() { "Hepsi".to_owned() } else { levels.join(", ") }).into(),
                    label::body(if wing_users.is_empty() { "—".to_owned() } else { wing_users.join(", ") }).into(),
                ])
                .selected(k == w.wing)
                .on_press(msg(Event::ChooseWing(k))),
            );
        }
        form = form
            .section("Kanatlar")
            .wide(wings)
            .row(words::secondary(ADD_WING, Some(msg(Event::AddWing(i)))));
        if let Some(x) = b.wings.get(w.wing) {
            let k = w.wing;
            form = form
                .field(
                    "Kanadın adı",
                    text_field("Doğu kanadı", &x.name)
                        .on_input(move |t| msg(Event::WingName(i, k, t)))
                        .width(scaled(260.0)),
                )
                .field("Kanadın katları", self.level_checks(&x.levels, move |l| msg(Event::WingLevel(i, k, l))))
                .help("Binanın katlarından seçilir; boşsa binanın bütün katları.")
                .row(remove("Kanadı sil", msg(Event::RemoveWing(i, k))));
        }
        let name = if b.name.trim().is_empty() { "Adsız bina".to_owned() } else { b.name.clone() };
        scrollable(
            column![head(name, remove("Binayı sil", msg(Event::RemoveBuilding(i)))), form]
                .spacing(12)
                .padding(iced::Padding::ZERO.right(14)),
        )
        .into()
    }

    // ── Bölümler ──────────────────────────────────────────────────────────────────────────────────────────────────

    /// The chosen unit: what it is, where it is, its kat irtifakı values, and, after Denetle, what was measured of it.
    fn unit_details<'a>(&'a self, w: &'a Window) -> Element<'a, Message>{
        let Some(r) = w.units.get(w.unit) else {
            return steps(
                "Programda bölüm yok",
                "Bölüm, mahallerin bir araya geldiği bağımsız bölüm, eklenti ya da ortak alandır.",
                &[
                    "Soldaki Bölüm ekle'ye basın; numara kendiliğinden verilir.",
                    "Adını, türünü ve binasını seçin.",
                    "Çizimde mahallerini seçip Seçilenleri ver'e basın.",
                ],
            );
        };
        let i = w.unit;
        let kinds = UnitKind::ALL;
        let kind = Select::new(
            kinds.iter().map(|k| Choice::new(k.label())),
            kinds.iter().position(|k| *k == r.unit.kind),
            move |k| msg(Event::UnitKind(i, kinds[k])),
        )
        .searchable(false);
        let mut form = Form::new()
            .label_width(scaled(LABELS))
            .field("No", text_field("1", &r.unit.number).on_input(move |t| msg(Event::UnitNumber(i, t))).width(scaled(100.0)))
            .field("Adı", text_field("Daire 1", &r.name).on_input(move |t| msg(Event::UnitName(i, t))).width(scaled(260.0)))
            .field("Türü", container(kind).width(scaled(200.0)))
            .field(
                "Kayıtlı alan (m²)",
                text_field("Tapu, ruhsat", &r.area).on_input(move |t| msg(Event::UnitArea(i, t))).width(scaled(120.0)),
            )
            .help("Tapu ya da ruhsattaki alan; yalnız karşılaştırılır, hesapla yazılmaz.");
        // Its building and wing.
        let ids: Vec<Option<String>> = std::iter::once(None).chain(w.buildings.iter().map(|b| Some(b.id.clone()))).collect();
        let names: Vec<String> = std::iter::once("—".to_owned()).chain(w.buildings.iter().map(|b| b.name.clone())).collect();
        let at = ids.iter().position(|x| *x == r.unit.building);
        let building = Select::new(names.into_iter().map(Choice::new), at, move |k| {
            msg(Event::UnitBuilding(i, ids.get(k).cloned().flatten()))
        })
        .searchable(false);
        form = form.field("Bina", container(building).width(scaled(200.0)));
        let wings: Vec<(String, String)> = r
            .unit
            .building
            .as_deref()
            .and_then(|b| w.buildings.iter().find(|x| x.id == b))
            .map(|b| b.wings.iter().map(|x| (x.id.clone(), x.name.clone())).collect())
            .unwrap_or_default();
        if !wings.is_empty() {
            let wing_ids: Vec<Option<String>> = std::iter::once(None).chain(wings.iter().map(|x| Some(x.0.clone()))).collect();
            let wing_names: Vec<String> = std::iter::once("—".to_owned()).chain(wings.iter().map(|x| x.1.clone())).collect();
            let wat = wing_ids.iter().position(|x| *x == r.unit.wing);
            let wing = Select::new(wing_names.into_iter().map(Choice::new), wat, move |k| {
                msg(Event::UnitWing(i, wing_ids.get(k).cloned().flatten()))
            })
            .searchable(false);
            form = form.field("Kanat", container(wing).width(scaled(200.0)));
        }
        // Kat irtifakı (docs/adr/8031): a unit's and an annex's nature; an annex's unit; a unit's value factor.
        if matches!(r.unit.kind, UnitKind::Unit | UnitKind::Annex) {
            form = form
                .section("Kat irtifakı")
                .field(
                    "Niteliği",
                    text_field("—", &r.nature)
                        .on_input(move |t| msg(Event::UnitNature(i, t)))
                        .width(scaled(200.0)),
                )
                .help("Daire, dükkân, iş bürosu, depo …");
        }
        if r.unit.kind == UnitKind::Annex {
            let units: Vec<&UnitRow> = w.units.iter().filter(|x| x.unit.kind == UnitKind::Unit).collect();
            let ids: Vec<String> = units.iter().map(|x| x.unit.id.clone()).collect();
            let names: Vec<String> = units
                .iter()
                .map(|x| if x.name.trim().is_empty() { x.unit.number.clone() } else { format!("{} {}", x.unit.number, x.name.trim()) })
                .collect();
            let at = ids.iter().position(|x| Some(x) == r.unit.annex_of.as_ref());
            let of = Select::new(names.into_iter().map(Choice::new), at, move |k| msg(Event::UnitAnnexOf(i, ids.get(k).cloned())))
                .searchable(false);
            form = form.field("Ait olduğu bölüm", container(of).width(scaled(200.0)));
        }
        if r.unit.kind == UnitKind::Unit {
            form = form
                .field("Değer katsayısı", text_field("1", &r.factor).on_input(move |t| msg(Event::UnitFactor(i, t))).width(scaled(100.0)))
                .help("Arsa payında; boşsa 1.");
            if w.share_rule == Some(ShareRule::Weight) {
                form = form.field(
                    "Pay ağırlığı",
                    text_field("120", &r.weight).on_input(move |t| msg(Event::UnitWeight(i, t))).width(scaled(100.0)),
                );
            }
        }
        // Its rooms: given from the drawing's selection; measured after Denetle.
        form = form
            .section("Mahalleri")
            .row(row![
                words::secondary(GIVE, Some(msg(Event::Give(i)))),
                label::caption("Çizimde seçili mahalleri bu bölüme verir; tek geri alma adımı."),
            ]
            .spacing(10)
            .align_y(Center));
        if let Some(u) = w.checked.as_deref().and_then(|c| c.units.iter().find(|u| u.id == r.unit.id)) {
            let levels: Vec<String> = u.levels.iter().map(|l| level_name(self, l)).collect();
            let methods: Vec<&str> = u.methods.iter().map(|m| m.label()).collect();
            form = form
                .field("Mahal sayısı", label::body(u.spaces.to_string()))
                .field("Odalar", label::body(format!("{} m²", fixed2(u.rooms))))
                .field("Dış alan", label::body(format!("{} m²", fixed2(u.outdoor))))
                .field("Şaft", label::body(format!("{} m²", fixed2(u.voids))));
            if !levels.is_empty() {
                form = form.field("Katları", label::body(levels.join(", ")));
            }
            if !methods.is_empty() {
                form = form.field("Ölçme yöntemi", label::body(methods.join(", ")));
            }
            if let (Some(a), Some(d)) = (u.recorded_area, u.recorded_difference) {
                form = form.field("Kayıtlı / fark", label::body(format!("{} m² / {} m²", fixed2(a), fixed2(d))));
            }
            if let Some(sh) = &u.share {
                form = form.field("Pay / brüt", label::body(format!("{} m² / {} m²", sh.share, sh.gross)));
            }
        }
        let name = match r.name.trim() {
            "" => r.unit.number.clone(),
            n => format!("{} {n}", r.unit.number),
        };
        scrollable(
            column![head(name, remove("Bölümü sil", msg(Event::RemoveUnit(i)))), form]
                .spacing(12)
                .padding(iced::Padding::ZERO.right(14)),
        )
        .into()
    }

    // ── Durum ─────────────────────────────────────────────────────────────────────────────────────────────────────

    /// The check's answer: the totals, the shares, every room measured, the findings.
    fn status_view<'a>(&'a self, w: &'a Window) -> Element<'a, Message>{
        let Some(c) = w.checked.as_deref() else {
            return steps(
                "Henüz denetlenmedi",
                "Denetle programı çizimle karşılaştırır: mahallerin alanını, enini, geçişlerini ve bölümlerin payını ölçer.",
                &["Alttaki Denetle'ye basın.", "Bitince sonuç burada görünür; bulgular tasarım hedefidir, yasal sınır değil."],
            );
        };
        let t = &c.totals;
        let totals = Pairs::new()
            .push(label::caption("Bağımsız bölümler"), label::body(format!("{} m² (dış {} m²)", fixed2(t.units), fixed2(t.unit_outdoor))))
            .push(label::caption("Ortak alanlar"), label::body(format!("{} m²", fixed2(t.common))))
            .push(label::caption("Dış alanlar"), label::body(format!("{} m²", fixed2(t.outdoor))))
            .push(label::caption("Şaftlar"), label::body(format!("{} m²", fixed2(t.voids))))
            .push(label::caption("Bölümsüz"), label::body(format!("{} m²", fixed2(t.unassigned))))
            .push(label::caption("Hepsi"), label::body(format!("{} m²; her mahal bir kez sayılır", fixed2(t.all))));
        let mut out = Column::new().spacing(12).push(label::strong("Alanlar")).push(totals);
        if let Some(sh) = &c.shares {
            out = out.push(shares_view(c, sh));
        }
        out = out.push(label::strong("Mahaller")).push(self.spaces_table(c));
        if !c.findings.is_empty() {
            let mut found = Column::new().spacing(4).push(label::strong(format!("Bulgular ({})", c.findings.len())));
            let unsaved = w.unsaved();
            if unsaved {
                found = found.push(label::caption("Bulgulara planda gitmek için programı kaydedip yeniden denetleyin."));
            }
            for (index, finding) in c.findings.iter().enumerate() {
                let action = (!unsaved && w.running.is_none() && !finding.uids.is_empty())
                    .then_some(msg(Event::ShowFinding(index)));
                found = found.push(row![
                    container(words::text_line(Kind::Warn, finding.message.clone())).width(Fill),
                    words::secondary(SHOW_ON_PLAN, action),
                ].align_y(Center).spacing(8));
            }
            out = out.push(found);
        }
        out.push(label::caption(format!("Kural: {}", c.rule))).into()
    }

    /// Every room measured: its number, name, unit, level, method, area against its brief, width, passage, doors.
    fn spaces_table<'a>(&'a self, c: &'a ProgramChecked) -> Element<'a, Message>{
        let unit_no = |id: &Option<String>| -> String {
            id.as_deref()
                .map(|u| c.units.iter().find(|x| x.id == u).map_or_else(|| format!("{u}?"), |x| x.number.clone()))
                .unwrap_or_else(|| "—".into())
        };
        let mut t = Table::new([
            Col::new("No").width(scaled(48.0)),
            Col::new("Ad").width(Fill),
            Col::new("Bölüm").width(scaled(52.0)),
            Col::new("Kat").width(scaled(80.0)),
            Col::new("Yöntem").width(scaled(110.0)),
            Col::new("Alan m²").width(scaled(64.0)).align_right(),
            Col::new("Hedef m²").width(scaled(96.0)).align_right(),
            Col::new("Fark m²").width(scaled(64.0)).align_right(),
            Col::new("Net en").width(scaled(56.0)).align_right(),
            Col::new("Geçiş").width(scaled(52.0)).align_right(),
            Col::new("Kapı").width(scaled(40.0)).align_right(),
        ])
        .height(scaled(220.0))
        .empty("Çizimde mahal yok.");
        for s in &c.spaces {
            let target = match (s.target_min, s.target_max) {
                (Some(a), Some(b)) => format!("{}–{}", fixed2(a), fixed2(b)),
                (Some(a), None) => format!("≥ {}", fixed2(a)),
                (None, Some(b)) => format!("≤ {}", fixed2(b)),
                (None, None) => "—".into(),
            };
            let diff = s.target_difference.map_or_else(|| "—".into(), |d| if d > 0.0 { format!("+{}", fixed2(d)) } else { fixed2(d) });
            let method = if s.void { "Şaft (boşluk)".to_owned() } else { s.method.label().to_owned() };
            let level = s.level.as_deref().map_or_else(|| "—".to_owned(), |l| level_name(self, l));
            t = t.push(Row::new([
                label::body(s.number.clone().unwrap_or_default()).into(),
                label::body(s.name.clone().or(s.usage.clone()).unwrap_or_default()).into(),
                label::body(unit_no(&s.unit)).into(),
                label::body(level).into(),
                label::body(method).into(),
                label::body(fixed2(s.area)).into(),
                label::body(target).into(),
                label::body(diff).into(),
                label::body(fixed2(s.width)).into(),
                label::body(s.passage.map_or_else(|| "—".into(), fixed2)).into(),
                label::body(s.doors.to_string()).into(),
            ]));
        }
        t.into()
    }
}

// ── Bölümler and Hedefler: their lists and settings ─────────────────────────────────────────────────────────────

/// Bölümler's settings on one line: the rule the common areas are shared by and the land shares' whole.
fn units_settings(w: &Window) -> Element<'_, Message> {
    let rules: [Option<ShareRule>; 4] = [None, Some(ShareRule::Equal), Some(ShareRule::Net), Some(ShareRule::Weight)];
    let rule = Select::new(
        rules.iter().map(|r| Choice::new(r.map_or("Paylaştırma", ShareRule::label))),
        rules.iter().position(|r| *r == w.share_rule),
        move |k| msg(Event::Rule(rules[k])),
    )
    .searchable(false);
    let land = text_field("Parsel alanı", &w.land_total).on_input(|t| msg(Event::LandTotal(t))).width(scaled(120.0));
    container(
        Form::new().label_width(scaled(LABELS)).field(
            "Ortak alan payı",
            row![
                container(rule).width(scaled(200.0)),
                space::horizontal().width(scaled(16.0)),
                label::body("Arsa payı paydası"),
                land,
                label::caption("m²"),
            ]
            .spacing(8)
            .align_y(Center),
        ),
    )
    .height(scaled(SETTINGS))
    .into()
}

/// The units: each one's number, name and kind.
fn units_list(w: &Window) -> Element<'_, Message> {
    let mut t = Table::new([
        Col::new("No").width(scaled(48.0)),
        Col::new("Ad").width(Fill),
        Col::new("Tür").width(scaled(86.0)),
    ])
    .empty("Programda bölüm yok.");
    for (i, r) in w.units.iter().enumerate() {
        let kind = match r.unit.kind {
            UnitKind::Unit => "Bağımsız",
            UnitKind::Annex => "Eklenti",
            UnitKind::Common => "Ortak",
            UnitKind::Outdoor => "Dış alan",
        };
        t = t.push(
            Row::new([
                label::body(r.unit.number.clone()).into(),
                label::body(if r.name.trim().is_empty() { "—".to_owned() } else { r.name.clone() }).into(),
                label::body(kind).into(),
            ])
            .selected(i == w.unit)
            .on_press(msg(Event::ChooseUnit(i))),
        );
    }
    t.into()
}

/// Hedefler's setting: the briefs' version, and what they are.
fn briefs_settings(w: &Window) -> Element<'_, Message> {
    container(
        Form::new().label_width(scaled(LABELS)).field(
            "Hedeflerin sürümü",
            row![
                text_field("Ofis konut hedefleri 2026-1", &w.version)
                    .on_input(|t| msg(Event::Version(t)))
                    .width(scaled(260.0)),
                label::caption("Tasarımcının kendi hedefleri; yönetmelik ya da yasal sınır değildir."),
            ]
            .spacing(10)
            .align_y(Center),
        ),
    )
    .height(scaled(SETTINGS))
    .into()
}

/// A brief's area as the list says it: “12–15”, “≥ 6”, “≤ 8”, “—”.
fn area_words(min: &str, max: &str) -> String {
    match (min.trim(), max.trim()) {
        ("", "") => "—".into(),
        (a, "") => format!("≥ {a}"),
        ("", b) => format!("≤ {b}"),
        (a, b) => format!("{a}–{b}"),
    }
}

/// The briefs: each one's use and area.
fn briefs_list(w: &Window) -> Element<'_, Message> {
    let mut t = Table::new([Col::new("Kullanım").width(Fill), Col::new("Alan m²").width(scaled(80.0)).align_right()])
        .empty("Hedef yok.");
    for (i, r) in w.briefs.iter().enumerate() {
        let usage = if r.brief.usage.trim().is_empty() { "—".to_owned() } else { r.brief.usage.clone() };
        t = t.push(
            Row::new([label::body(usage).into(), label::body(area_words(&r.texts[0], &r.texts[1])).into()])
                .selected(i == w.brief)
                .on_press(msg(Event::ChooseBrief(i))),
        );
    }
    t.into()
}

/// The chosen brief: its use, area, width, passage, neighbours and the uses reached through it, whether it is outdoor.
fn brief_details(w: &Window) -> Element<'_, Message> {
    let Some(r) = w.briefs.get(w.brief) else {
        return steps(
            "Hedef yok",
            "Hedef, bir kullanımdaki mahallerin istenen alanı, eni, geçişi ve komşularıdır; Denetle bunlara göre ölçer.",
            &["Soldaki Hedef ekle'ye basın.", "Kullanımı (Yatak odası, Salon …) ve istenen değerleri yazın."],
        );
    };
    let i = w.brief;
    let text = |k: usize, place: &'static str, width: f32| {
        text_field(place, &r.texts[k]).on_input(move |t| msg(Event::BriefText(i, k, t))).width(scaled(width))
    };
    let form = Form::new()
        .label_width(scaled(LABELS + 10.0))
        .field(
            "Kullanım",
            text_field("Yatak odası", &r.brief.usage).on_input(move |t| msg(Event::BriefUsage(i, t))).width(scaled(260.0)),
        )
        .help("Bu kullanımdaki ya da bu adı taşıyan mahallere uygulanır.")
        .field("Alan (m²)", row![text(0, "en az", 90.0), label::body("–"), text(1, "en çok", 90.0)].spacing(8).align_y(Center))
        .field("Net en (m)", text(2, "—", 90.0))
        .help("En dar yerindeki genişlik: içine sığan en büyük dairenin çapı.")
        .field("Net geçiş (m)", text(3, "—", 90.0))
        .help("Koridor ve holde kapıların arasından geçebilen genişlik.")
        .field("Komşu kullanımlar", text(4, "—", 260.0))
        .help("Virgülle; bu mahalin bitişiğinde istenenler.")
        .field("Geçilen kullanımlar", text(5, "—", 260.0))
        .help("Virgülle; kapı ya da açık geçişle ulaşılması istenenler.")
        .row(words::check(r.brief.outdoor, "Dış mahal (balkon, teras)", Some(msg(Event::BriefOutdoor(i)))));
    let name = if r.brief.usage.trim().is_empty() { "Adsız hedef".to_owned() } else { r.brief.usage.clone() };
    scrollable(
        column![head(name, remove("Hedefi sil", msg(Event::RemoveBrief(i)))), form]
            .spacing(12)
            .padding(iced::Padding::ZERO.right(14)),
    )
    .into()
}

/// Paylar (docs/adr/0784 §2): each independent unit's net, share and gross; each common area's pool and its shares, or
/// why it is not shared; the totals, which add up exactly.
fn shares_view<'a>(c: &'a ProgramChecked, sh: &'a kentos_commands::ProgramShares) -> Element<'a, Message> {
    let mut table = Table::new([
        Col::new("Bölüm").width(Fill),
        Col::new("Net m²").width(scaled(80.0)).align_right(),
        Col::new("Pay m²").width(scaled(80.0)).align_right(),
        Col::new("Brüt m²").width(scaled(80.0)).align_right(),
        Col::new("Kayıtlı m²").width(scaled(80.0)).align_right(),
    ]);
    for u in &c.units {
        let Some(p) = &u.share else { continue };
        let name = match &u.name {
            Some(n) => format!("{} {n}", u.number),
            None => u.number.clone(),
        };
        table = table.push(Row::new([
            label::body(name).into(),
            label::body(p.net.clone()).into(),
            label::body(p.share.clone()).into(),
            label::body(p.gross.clone()).into(),
            label::body(u.recorded_area.map_or_else(|| "—".into(), fixed2)).into(),
        ]));
    }
    table = table.push(Row::new([
        label::strong("Toplam").into(),
        label::strong(sh.net.clone()).into(),
        label::strong(sh.shared.clone()).into(),
        label::strong(sh.gross.clone()).into(),
        label::body("").into(),
    ]));
    let number = |id: &str| c.units.iter().find(|u| u.id == id).map_or_else(|| id.to_owned(), |u| u.number.clone());
    let mut pools = Column::new().spacing(4);
    for p in &sh.pools {
        let line = match &p.unshared {
            Some(why) => words::text_line(Kind::Warn, format!("{} ({} m²): {why}", p.number, p.area)),
            None => {
                let parts: Vec<String> = p.shares.iter().map(|s| format!("{} {}", number(&s.unit), s.share)).collect();
                words::text_line(Kind::Info, format!("{} ({} m²) → {}", p.number, p.area, parts.join(" · ")))
            }
        };
        pools = pools.push(line);
    }
    let mut out = column![label::strong(format!("Paylar: {}", sh.rule)), table, pools].spacing(8);
    if sh.unshared != "0.00" {
        out = out.push(label::caption(format!("Paylaştırılmayan ortak alan {} m².", sh.unshared)));
    }
    out.push(label::caption(
        "0,01 m² adım; artan adımlar en büyük kalanlara, eşitlikte programdaki sıraya. Paylar her ortak alanın alanına tam \
         eşittir; kayıtlı alan yalnız okunur, hesapla yazılmaz.",
    ))
    .into()
}
