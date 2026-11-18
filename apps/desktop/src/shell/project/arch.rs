//! Proje ayarları › Mimari (docs/adr/0339 §3, §6): the project's levels, a row each (Ad, Kot, Kat yüksekliği; up,
//! down, delete; Kat ekle), and the detail the walls are drawn at (Ölçeğe göre, Şematik, Uygulama, Ayrıntılı). What
//! does not hold is said under the rows and Kaydet waits; Kaydet writes them through `cad.settings.set`'s `arch`,
//! which keeps a level a wall names. A level's height changed moves the levels above by as much (docs/adr/8040 §1): the
//! storeys sit on each other, and the walls under the slab of the one above follow.

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
    /// Its last height that read as a number: what a new one moves the levels above by (typed away and again, the
    /// move is from it).
    pub stacked: Option<f64>,
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
    (None, "Ölçeğe göre"),
    (Some(WallDetail::Schematic), "Şematik"),
    (Some(WallDetail::Standard), "Uygulama"),
    (Some(WallDetail::Detailed), "Ayrıntılı"),
];

/// Çizgi kalınlıkları (docs/adr/8013): the line groups by the scale, or every object its own pen (written as nothing).
const LINES: [(Option<kentos_commands::LineGroups>, &str); 2] = [
    (Some(kentos_commands::LineGroups::ByScale), "Ölçeğe göre"),
    (None, "Nesnenin kendi kalınlığı"),
];

/// İmar durumu as typed (docs/adr/8030): the parcel's area, m², the plan's emsal (KAKS) and TAKS, each with where it is
/// read from (docs/adr/8140); ±0.00's absolute elevation, m (the site plan's, docs/adr/8032); the municipality's
/// regulation chosen (docs/adr/8034); whether the building has a sprinkler system (Yangın güvenliği, docs/adr/8035).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoningTexts {
    pub parcel: String,
    pub far: String,
    pub coverage: String,
    /// Where the emsal and the TAKS are read from (docs/adr/8140): the zoning certificate, or the plan decision taken.
    pub far_source: String,
    pub coverage_source: String,
    pub datum: String,
    pub local: Option<kentos_commands::LocalRules>,
    pub sprinklers: bool,
    /// Plan notları (docs/adr/8084), a row each.
    pub plan_notes: Vec<super::plan_notes::Note>,
    /// Bölge: the parcel's area's use in the plan (docs/adr/8085).
    pub zone: Option<kentos_commands::Zone>,
    /// Yapı nizamı: the building's layout the plan sets (docs/adr/8141).
    pub layout: Option<kentos_commands::BuildingLayout>,
    /// Taşıyıcı sistem: the building's structural system and its traditional elements (docs/adr/8094).
    pub structure: Option<kentos_commands::BuildingStructure>,
    /// Yangın güvenliği: the building's fire protection class and its basis as typed (docs/adr/8068, 8124).
    pub fire_class: String,
    pub fire_basis: String,
}

/// Taşıyıcı sistem's choices (docs/adr/8094): not said, or a system.
fn structure_choices() -> Vec<(Option<kentos_commands::StructuralSystem>, &'static str)> {
    std::iter::once((None, "Yazılmamış"))
        .chain(kentos_commands::StructuralSystem::ALL.into_iter().map(|s| (Some(s), s.label())))
        .collect()
}

/// Bölge's choices (docs/adr/8085): not said, or the plan's use.
fn zone_choices() -> Vec<(Option<kentos_commands::Zone>, &'static str)> {
    std::iter::once((None, "Yazılmamış (konut projesinde konut alanı sayılır)"))
        .chain(kentos_commands::Zone::ALL.into_iter().map(|z| (Some(z), z.label())))
        .collect()
}

/// Yapı nizamı's choices (docs/adr/8141): not said, or the plan's layout.
fn layout_choices() -> Vec<(Option<kentos_commands::BuildingLayout>, &'static str)> {
    std::iter::once((None, "Yazılmamış (plan belirlemiyorsa ayrık uygulanır)"))
        .chain(kentos_commands::BuildingLayout::ALL.into_iter().map(|l| {
            (
                Some(l),
                match l {
                    kentos_commands::BuildingLayout::Twin => "İkiz (blok sayılır)",
                    other => other.label(),
                },
            )
        }))
        .collect()
}

/// Yerel imar yönetmeliği's choices (docs/adr/8034): none, or a municipality's.
const LOCAL_RULES: [(Option<kentos_commands::LocalRules>, &str); 2] = [
    (None, "Yok (yalnız Planlı Alanlar İmar Yönetmeliği)"),
    (
        Some(kentos_commands::LocalRules::Sivas),
        "Sivas Belediyesi İmar Yönetmeliği (10.11.2022)",
    ),
];

/// One of İmar durumu's fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoningField {
    Parcel,
    Far,
    Coverage,
    /// Where the emsal, the TAKS are read from (docs/adr/8140).
    FarSource,
    CoverageSource,
    Datum,
    /// The building's fire protection class, its basis (docs/adr/8124).
    FireClass,
    FireBasis,
}

impl ZoningTexts {
    /// The project's zoning as the form shows it.
    pub fn of(arch: Option<&ArchSettings>) -> Self {
        let z = arch.and_then(|a| a.zoning.as_ref());
        let fire = || arch.and_then(|a| a.performance.as_ref()).and_then(|p| p.fire_protection_class.as_ref());
        let t = |v: Option<f64>| {
            v.map(kentos_processing::text::js_number)
                .unwrap_or_default()
        };
        Self {
            parcel: t(z.map(|z| z.parcel_area)),
            far: t(z.and_then(|z| z.far)),
            coverage: t(z.and_then(|z| z.coverage)),
            far_source: z.and_then(|z| z.far_source.clone()).unwrap_or_default(),
            coverage_source: z.and_then(|z| z.coverage_source.clone()).unwrap_or_default(),
            datum: t(arch.and_then(|a| a.site.as_ref()).and_then(|s| s.datum)),
            local: arch.and_then(|a| a.local_rules),
            sprinklers: arch.is_some_and(|a| a.sprinklers),
            plan_notes: super::plan_notes::rows(arch.map_or(&[][..], |a| a.plan_notes.as_slice())),
            zone: arch.and_then(|a| a.zone),
            layout: arch.and_then(|a| a.layout),
            structure: arch.and_then(|a| a.structure),
            fire_class: fire().map(|d| d.value.clone()).unwrap_or_default(),
            fire_basis: fire().map(|d| d.basis.field()).unwrap_or_default(),
        }
    }

    pub fn edit(&mut self, field: ZoningField, text: String) {
        match field {
            ZoningField::Parcel => self.parcel = text,
            ZoningField::Far => self.far = text,
            ZoningField::Coverage => self.coverage = text,
            ZoningField::FarSource => self.far_source = text,
            ZoningField::CoverageSource => self.coverage_source = text,
            ZoningField::Datum => self.datum = text,
            ZoningField::FireClass => self.fire_class = text,
            ZoningField::FireBasis => self.fire_basis = text,
        }
    }

    /// The zoning of these values with the sources as typed, their ends trimmed; an empty one is none (docs/adr/8140).
    fn sourced(&self, parcel_area: f64, far: Option<f64>, coverage: Option<f64>) -> kentos_commands::Zoning {
        let source = |s: &str| Some(s.trim().to_owned()).filter(|s| !s.is_empty());
        kentos_commands::Zoning {
            parcel_area,
            far,
            coverage,
            far_source: source(&self.far_source),
            coverage_source: source(&self.coverage_source),
        }
    }

    /// A zoning plan decision's values taken (docs/adr/8140): its emsal and TAKS into İmar durumu with their source, the
    /// plan's building height and storey count into Plan notları, each into its measure's row (a new row when none gives
    /// it). Nothing is saved: Kaydet writes them, Vazgeç leaves the project as it was.
    pub fn take(&mut self, t: &crate::zoning_badge::PlanTake) {
        let text = kentos_processing::text::js_number;
        if let Some(v) = t.far {
            self.far = text(v);
            self.far_source.clone_from(&t.source);
        }
        if let Some(v) = t.coverage {
            self.coverage = text(v);
            self.coverage_source.clone_from(&t.source);
        }
        if let Some(l) = t.layout {
            self.layout = Some(l);
        }
        if let Some(z) = t.zone {
            self.zone = Some(z);
        }
        for (key, v) in [
            (kentos_commands::PlanNoteKey::BuildingHeight, t.height),
            (kentos_commands::PlanNoteKey::Storeys, t.storeys),
        ] {
            let Some(v) = v else { continue };
            match self.plan_notes.iter_mut().find(|n| n.key == key) {
                Some(n) => {
                    n.value = text(v);
                    n.source.clone_from(&t.source);
                }
                None => self.plan_notes.push(super::plan_notes::Note { key, value: text(v), source: t.source.clone() }),
            }
        }
    }

    /// Whether the form holds a decision's values with its source, as [`ZoningTexts::take`] writes them.
    pub fn holds(&self, t: &crate::zoning_badge::PlanTake) -> bool {
        let same = |typed: &str, v: f64| number(typed) == Ok(Some(v));
        let note = |key, v: Option<f64>| {
            v.is_none_or(|v| {
                self.plan_notes.iter().any(|n| n.key == key && same(&n.value, v) && n.source.trim() == t.source)
            })
        };
        t.far.is_none_or(|v| same(&self.far, v) && self.far_source.trim() == t.source)
            && t.coverage.is_none_or(|v| same(&self.coverage, v) && self.coverage_source.trim() == t.source)
            && t.layout.is_none_or(|l| self.layout == Some(l))
            && t.zone.is_none_or(|z| self.zone == Some(z))
            && note(kentos_commands::PlanNoteKey::BuildingHeight, t.height)
            && note(kentos_commands::PlanNoteKey::Storeys, t.storeys)
    }

    /// The building's fire protection class typed (docs/adr/8124), its basis one line: none when the class is empty.
    pub fn performance(&self) -> Result<Option<kentos_commands::BuildingPerformance>, String> {
        let value = self.fire_class.trim();
        if value.is_empty() {
            if !self.fire_basis.trim().is_empty() {
                return Err("Dayanak belgesi binanın yangın koruma sınıfınındır: önce sınıfı yazın.".into());
            }
            return Ok(None);
        }
        let basis = kentos_commands::DeclaredBasis::read(&self.fire_basis)?;
        let class = kentos_commands::Declared { value: value.to_owned(), basis };
        if let Some(why) = class.problem("binanın yangın koruma sınıfı") {
            return Err(why);
        }
        Ok(Some(kentos_commands::BuildingPerformance { fire_protection_class: Some(class) }))
    }

    /// The zoning typed: none when all three are empty; the parcel's area when any is given.
    pub fn read(&self) -> Result<Option<kentos_commands::Zoning>, String> {
        let parcel =
            number(&self.parcel).map_err(|()| "Parsel alanı bir sayı olmalı.".to_owned())?;
        let far = number(&self.far).map_err(|()| "Emsal bir sayı olmalı.".to_owned())?;
        let coverage = number(&self.coverage).map_err(|()| "TAKS bir sayı olmalı.".to_owned())?;
        match parcel {
            // Nothing typed but a source: the source's own words (docs/adr/8140).
            None if far.is_none() && coverage.is_none() => match self.sourced(0.0, None, None).sources_problem() {
                Some((_, why)) => Err(why),
                None => Ok(None),
            },
            None => Err("Emsal ya da TAKS için parsel alanı da verilmeli.".into()),
            Some(parcel_area) => Ok(Some(self.sourced(parcel_area, far, coverage))),
        }
    }
}

/// İmar durumu (docs/adr/8030): the fields, the emsal's and the TAKS's source under each (docs/adr/8140) and, when the
/// drawing has a zoning, its ratios: each line's words, value and whether it passes its limit; the zoning plan's
/// decisions on the parcel, each that gives a value with its Değerlerini al.
#[allow(clippy::too_many_arguments)]
pub(super) fn zoning_view<'a>(
    t: &'a ZoningTexts,
    on: impl Fn(ZoningField, String) -> Message + Copy + 'a,
    on_local: impl Fn(Option<kentos_commands::LocalRules>) -> Message + 'a,
    on_zone: impl Fn(Option<kentos_commands::Zone>) -> Message + 'a,
    on_layout: impl Fn(Option<kentos_commands::BuildingLayout>) -> Message + 'a,
    summary: Vec<(String, String, bool)>,
    plan: Vec<(String, bool)>,
    takes: Vec<(usize, crate::zoning_badge::PlanTake)>,
    on_take: impl Fn(crate::zoning_badge::PlanTake) -> Message + 'a,
) -> Element<'a, Message> {
    let wrong = |s: &str| number(s).is_err();
    let line = |name: &'static str, holder: &'static str, value: &'a str, f: ZoningField| {
        row![
            container(label::body(name)).width(typography::scaled(140.0)),
            container(field(holder, value, wrong(value), move |v| on(f, v)))
                .width(typography::scaled(120.0)),
        ]
        .spacing(8)
        .align_y(Center)
    };
    // A source said without its value, too long or broken over lines (docs/adr/8140): an empty one is none.
    let source_wrong = |source: &str, value: &str| {
        let s = source.trim();
        !s.is_empty()
            && (value.trim().is_empty()
                || s.chars().count() > kentos_commands::MAX_ZONING_SOURCE
                || s.chars().any(char::is_control))
    };
    let source = |name: &'static str, value: &'a str, of: &'a str, f: ZoningField| {
        row![
            container(label::body(name)).width(typography::scaled(140.0)),
            container(field("İmar durum belgesi 12.3.2025/1234", value, source_wrong(value, of), move |v| on(f, v)))
                .width(Fill),
        ]
        .spacing(8)
        .align_y(Center)
    };
    let mut body = Column::new()
        .spacing(6)
        .push(line("Parsel alanı (m²)", "450", &t.parcel, ZoningField::Parcel))
        .push(line("Emsal (KAKS)", "1,50", &t.far, ZoningField::Far))
        .push(source("Emsalin kaynağı", &t.far_source, &t.far, ZoningField::FarSource))
        .push(line("TAKS", "0,40", &t.coverage, ZoningField::Coverage))
        .push(source("TAKS'ın kaynağı", &t.coverage_source, &t.coverage, ZoningField::CoverageSource))
        .push(line("±0,00 kotu (m)", "1285,40", &t.datum, ZoningField::Datum))
        .push(
            row![
                container(label::body("Yerel imar yönetmeliği")).width(typography::scaled(140.0)),
                container(
                    Select::new(
                        LOCAL_RULES.iter().map(|(_, l)| Choice::new(*l)),
                        LOCAL_RULES.iter().position(|(r, _)| *r == t.local),
                        move |i| on_local(LOCAL_RULES.get(i).and_then(|(r, _)| *r)),
                    )
                    .searchable(false)
                )
                .width(typography::scaled(330.0)),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .push({
            let zones = zone_choices();
            let at = zones.iter().position(|(z, _)| *z == t.zone);
            row![
                container(label::body("Bölge")).width(typography::scaled(140.0)),
                container(
                    Select::new(zones.iter().map(|(_, l)| Choice::new(*l)), at, move |i| {
                        on_zone(zone_choices().get(i).and_then(|(z, _)| *z))
                    })
                    .searchable(false)
                )
                .width(typography::scaled(330.0)),
            ]
            .spacing(8)
            .align_y(Center)
        })
        .push({
            let layouts = layout_choices();
            let at = layouts.iter().position(|(l, _)| *l == t.layout);
            row![
                container(label::body("Yapı nizamı")).width(typography::scaled(140.0)),
                container(
                    Select::new(layouts.iter().map(|(_, l)| Choice::new(*l)), at, move |i| {
                        on_layout(layout_choices().get(i).and_then(|(l, _)| *l))
                    })
                    .searchable(false)
                )
                .width(typography::scaled(330.0)),
            ]
            .spacing(8)
            .align_y(Center)
        })
        .push(label::caption(
            "İmar durumundan. Emsal, emsale dahil mahallerin brüt alanının (katlar alanı) parsel alanına oranıdır; \
             emsal harici mahaller (otopark, sığınak, kazan dairesi, balkon …) parselin emsal alanının %30'unu \
             geçerse fazlası katlar alanına eklenir (Planlı Alanlar İmar Yönetmeliği 5/8, 22). TAKS, zemin katın \
             brüt alanının parsel alanına oranıdır. Kaynak, değerin okunduğu yerdir (imar durum belgesi ya da alındığı plan \
             kararı); Denetim'in bulgusu onu anar. ±0,00 kotu, projenin ±0,00'ının röperden okunan mutlak kotudur. \
             Parsel ve bahçe mesafeleri Mimari › Proje ▾ › Parsel ve bahçe mesafeleri ile verilir. Yerel imar \
             yönetmeliği seçilince Denetim › Mevzuat belediyenin değerlerini de okur: sıkı olan bağlar, ölçünün \
             belediyeye uyup Planlı Alanlar İmar Yönetmeliği'ne uymadığı yer çelişki olarak söylenir (idare karar \
             verir). Bölge, parselin planda ayrıldığı kullanımdır: Denetim › Mevzuat kat yüksekliklerini ona göre okur \
             (Planlı Alanlar İmar Yönetmeliği 28/1). Yapı nizamı planın ayrık, bitişik, blok ya da ikiz nizamıdır (ikiz \
             blok sayılır, 4/1-pppp): Denetim › Mevzuat TAKS verilmeyen parselin %40 ya da %60 sınırını (5/6), tavan \
             kaplamasını ve çatıyı (Binaların Yangından Korunması Hakkında Yönetmelik 26/4, 28/3) ona göre okur; plan \
             nizam belirlemiyorsa ayrık nizam uygulanır (8/5). Planın notunun değiştirdiği değerler aşağıda, Plan \
             notları'na yazılır. KentOS bir onay vermez.",
        ));
    for (words, value, over) in summary {
        body = body.push(
            row![
                container(label::body(words)).width(typography::scaled(220.0)),
                label::strong(value).style(move |theme: &iced::Theme| iced::widget::text::Style {
                    color: over.then(|| kentos_ui::theme::Tokens::of(theme).danger),
                }),
            ]
            .spacing(8),
        );
    }
    // The zoning plan's decisions on the parcel (docs/adr/8072 §5): with their source; a decision's values go into the
    // fields only by its Değerlerini al, and are saved only by Kaydet (docs/adr/8140).
    if !plan.is_empty() {
        body = body.push(label::body("Parselin plan kararları").font(typography::ui_strong()));
        for (i, (words, warn)) in plan.into_iter().enumerate() {
            let line = label::body(words);
            body = body.push(if warn { line.style(kentos_ui::style::text::warning) } else { line });
            for (_, take) in takes.iter().filter(|(after, _)| *after == i) {
                let held = t.holds(take);
                let take_button = button(
                    row![icon(crate::icons::from_web(Some("import"))).size(13.0), label::body("Değerlerini al")]
                        .spacing(5)
                        .align_y(Center),
                )
                .on_press_maybe((!held).then(|| on_take(take.clone())))
                .padding([4, 10])
                .style(style::button::secondary);
                body = body.push(
                    row![take_button, label::caption(take.words(held)).width(Fill)].spacing(8).align_y(Center),
                );
            }
        }
        body = body.push(label::caption(
            "Belediyenin CBS'inden gelen imar planı katmanlarından, parselle kesişimiyle okunur. Bir kesişim tek başına \
             hukuki karar değildir: değerleri imar durum belgesiyle karşılaştırın. Değerlerini al, kararın sayıyla \
             yazılmış emsalini ve TAKS'ını kaynağıyla (plan, onay günü, karar), yapı nizamını ve kullanımını (Bölge) \
             yukarıya, bina yüksekliğini (Hmax) ve kat adedini Plan notları'na yazar; Kaydet onaylar, Vazgeç bırakır. \
             Planı belli olmayan kararın değeri alınmaz; zemin katında ticaret yapılabilen konut alanının planda kodu \
             yoktur, elle seçilir.",
        ));
    }
    super::group::<Message>("İmar durumu", body)
}

/// Yangın güvenliği (docs/adr/8035, 8124): whether the building has a sprinkler system, the fire regulation's escape
/// distances read by it; the building's fire protection class and its basis, which the model carries.
pub(super) fn fire_view<'a>(
    t: &'a ZoningTexts,
    on: impl Fn(bool) -> Message + 'a,
    on_text: impl Fn(ZoningField, String) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let line = |name: &'static str, holder: &'static str, value: &'a str, wrong: bool, f: ZoningField| {
        row![
            container(label::body(name)).width(typography::scaled(140.0)),
            container(field(holder, value, wrong, move |v| on_text(f, v))).width(typography::scaled(330.0)),
        ]
        .spacing(8)
        .align_y(Center)
    };
    let basis_wrong = kentos_commands::DeclaredBasis::read(&t.fire_basis).is_err();
    let body = Column::new()
        .spacing(6)
        .push(kentos_ui::widget::Switch::new(t.sprinklers, on).label("Yağmurlama (sprinkler) sistemi var"))
        .push(line("Yangın koruma sınıfı", "Yok", &t.fire_class, false, ZoningField::FireClass))
        .push(line(
            "Dayanak belgesi",
            "Müellif beyanı",
            &t.fire_basis,
            basis_wrong,
            ZoningField::FireBasis,
        ))
        .push(label::caption(
            "Denetim › Mevzuat kaçış uzaklıklarını Binaların Yangından Korunması Hakkında Yönetmelik'e göre ölçer: \
             en uzak nokta duvardan 40 cm içeride alınır (32/6), sınırlar Ek-5/B'den ve konutta 48/2'den gelir \
             (yağmurlamalı binada daha uzun). Korunumlu merdiven, yangın kapısı ve güvenlik holü çizimden okunmaz. \
             Yangın koruma sınıfı binanın modelinde yazılır; müellifin beyanıdır, dayanağı bir belgeyse adını ve gününü \
             yazın: Rapor 41 (2026-02-28).",
        ));
    super::group::<Message>("Yangın güvenliği", body)
}

/// Taşıyıcı sistem (docs/adr/8094): the building's structural system and, of a cut stone masonry one, whether it holds
/// traditional architectural elements.
pub(super) fn structure_view<'a>(
    t: &'a ZoningTexts,
    on_system: impl Fn(Option<kentos_commands::StructuralSystem>) -> Message + 'a,
    on_traditional: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let choices = structure_choices();
    let at = choices.iter().position(|(s, _)| *s == t.structure.map(|x| x.system));
    let mut body = Column::new().spacing(6).push(
        row![
            container(label::body("Sistem")).width(typography::scaled(140.0)),
            container(
                Select::new(choices.iter().map(|(_, l)| Choice::new(*l)), at, move |i| {
                    on_system(structure_choices().get(i).and_then(|(s, _)| *s))
                })
                .searchable(false)
            )
            .width(typography::scaled(330.0)),
        ]
        .spacing(8)
        .align_y(Center),
    );
    // Traditional elements are read of a cut stone masonry building only (PAİY 20/8-j, 22/1-m, 28/1-d).
    if let Some(x) = t.structure.filter(|x| x.system == kentos_commands::StructuralSystem::CutStoneMasonry) {
        body = body.push(
            kentos_ui::widget::Switch::new(x.traditional, on_traditional)
                .label("Geleneksel mimari öğeler barındırıyor"),
        );
    }
    body = body.push(label::caption(
        "Statik projenin taşıyıcı sistemidir. Geleneksel mimari öğeler barındıran, zemin dahil 3 katı geçmeyen kesme taş \
         yığma binada Denetim › Mevzuat konut bölgesinde kat yüksekliğini 4,50 m'yle okur (Planlı Alanlar İmar \
         Yönetmeliği 28/1-d); Emsal ve TAKS tablosu Taşlık, Dış sofa ve Eyvan adlı mahalleri emsal harici sayar (22/1-m), \
         koşullarını Denetim söyler. KentOS bir onay vermez.",
    ));
    super::group::<Message>("Taşıyıcı sistem", body)
}

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
                stacked: l.height,
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
        stacked: Some(3.0),
    }
}

pub fn edit(rows: &mut Vec<Row>, i: usize, e: Edit) {
    let n = rows.len();
    let Some(r) = rows.get_mut(i) else { return };
    match e {
        Edit::Name(t) => r.name = t,
        Edit::Elevation(t) => r.elevation = t,
        Edit::Height(t) => {
            r.height = t;
            stack(rows, i);
        }
        Edit::Prefix(t) => r.prefix = t,
        Edit::Up if i > 0 => rows.swap(i, i - 1),
        Edit::Down if i + 1 < n => rows.swap(i, i + 1),
        Edit::Up | Edit::Down => {}
        Edit::Delete => {
            rows.remove(i);
        }
    }
}

/// Row `i`'s height now read as a number moves the levels at or above its top by the change (docs/adr/8040 §1): a
/// storey's height and the elevations above it stay one stack, as Türkiye's sections give them (±0.00, +3.00, +6.00).
/// A level whose elevation does not read stays; a height that does not read moves nothing until it does.
fn stack(rows: &mut [Row], i: usize) {
    let Some(r) = rows.get(i) else { return };
    let (Ok(Some(new)), Some(old)) = (number(&r.height), r.stacked) else {
        return;
    };
    let Ok(Some(foot)) = number(&r.elevation) else {
        rows[i].stacked = Some(new);
        return;
    };
    let delta = new - old;
    rows[i].stacked = Some(new);
    if delta.abs() < 1e-9 {
        return;
    }
    let top = foot + old;
    for (j, above) in rows.iter_mut().enumerate() {
        if j == i {
            continue;
        }
        if let Ok(Some(e)) = number(&above.elevation)
            && e >= top - 1e-6
        {
            // To the millimetre: 3.0 + 0.2 + 0.04 reads 3.24, not 3.2400000000000002.
            above.elevation = shown(Some(((e + delta) * 1000.0).round() / 1000.0));
        }
    }
}

/// The rows and the detail read as the settings, or what does not hold.
pub fn read(
    rows: &[Row],
    detail: Option<WallDetail>,
    space_method: Option<kentos_commands::SpaceMethod>,
    cut: &str,
    lines: Option<kentos_commands::LineGroups>,
    zoning: &ZoningTexts,
    choices: Option<&ArchSettings>,
) -> Result<ArchSettings, String> {
    let cut_height = number(cut).map_err(|()| "Kesme yüksekliği bir sayı olmalı.".to_owned())?;
    // ±0.00's elevation (docs/adr/8032): the site plan's, the rest of it kept as the project has it.
    let datum = number(&zoning.datum).map_err(|()| "±0,00 kotu bir sayı olmalı.".to_owned())?;
    // The municipality's regulation the audit reads (docs/adr/8034) and the sprinkler system (docs/adr/8035).
    let local_rules = zoning.local;
    let sprinklers = zoning.sprinklers;
    // The parcel's plan notes (docs/adr/8084) and its area's use (docs/adr/8085).
    let plan_notes = super::plan_notes::read(&zoning.plan_notes)?;
    let zone = zoning.zone;
    // The building's layout (docs/adr/8141).
    let layout = zoning.layout;
    let performance = zoning.performance()?;
    // The building's structural system (docs/adr/8094).
    let structure = zoning.structure;
    let site = {
        let mut s = choices.and_then(|a| a.site.clone()).unwrap_or_default();
        s.datum = datum;
        (s != kentos_commands::SitePlan::default()).then_some(s)
    };
    let zoning = zoning.read()?;
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
        // The plan's cut height (docs/adr/8012); empty: 1.10.
        cut_height,
        // How its lines are weighted (docs/adr/8013); none: every object its own pen.
        line_groups: lines,
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
        // So are the building materials (docs/adr/0785).
        materials: choices.map_or_else(Vec::new, |a| a.materials.clone()),
        // So are the design options (docs/adr/0788).
        options: choices.map_or_else(Vec::new, |a| a.options.clone()),
        // And their combinations (docs/adr/0808).
        option_combos: choices.map_or_else(Vec::new, |a| a.option_combos.clone()),
        // The parcel's zoning (docs/adr/8030).
        zoning,
        // The site plan (docs/adr/8032): its parcel and edges are Parsel's, ±0.00 this form's.
        site,
        local_rules,
        sprinklers,
        plan_notes,
        zone,
        layout,
        structure,
        // The building's fire protection class (docs/adr/8068, 8124): this form's Yangın güvenliği.
        performance,
        // The form's answers (docs/adr/8077) are their own window's: kept as the project has them.
        form_answers: choices.map_or_else(Vec::new, |a| a.form_answers.clone()),
        // So is the delivery record (docs/adr/8099).
        deliveries: choices.map_or_else(Vec::new, |a| a.deliveries.clone()),
        underlays: choices.map_or_else(Vec::new, |a| a.underlays.clone()),
        element_types: choices.map_or_else(Vec::new, |a| a.element_types.clone()),
    };
    match arch.problem() {
        Some((_, why)) => Err(why),
        None => Ok(arch),
    }
}

pub(super) fn small(glyph: &'static str, enabled: bool, on: Message) -> Element<'static, Message> {
    button(container(icon(crate::icons::from_web(Some(glyph))).size(14.0)).center_x(Length::Fill))
        .on_press_maybe(enabled.then_some(on))
        .padding([4, 0])
        .width(Length::Fixed(26.0))
        .style(style::button::ghost)
        .into()
}

pub(super) fn field<'a>(
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

/// The section: each of its fields with the message it sends (the other sections' views take theirs the same way).
#[allow(clippy::too_many_arguments)]
pub(super) fn view<'a>(
    rows: &'a [Row],
    detail: Option<WallDetail>,
    on: impl Fn(usize, Edit) -> Message + Copy + 'a,
    on_add: Message,
    on_detail: impl Fn(Option<WallDetail>) -> Message + 'a,
    method: Option<kentos_commands::SpaceMethod>,
    on_method: impl Fn(Option<kentos_commands::SpaceMethod>) -> Message + 'a,
    cut: &'a str,
    on_cut: impl Fn(String) -> Message + 'a,
    lines: Option<kentos_commands::LineGroups>,
    on_lines: impl Fn(Option<kentos_commands::LineGroups>) -> Message + 'a,
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
                "Ölçeğe göre: modelde yaklaştıkça ayrıntı artar. 1:30 ve daha büyükte ayrıntılı, 1:75'e dek uygulama, daha küçükte şematik çizilir; 1:150'den küçük paftada kapı kanadıyla, merdiven korkuluksuz, mobilya dış çizgisiyle en sade hâlini alır. Pafta ve PDF kendi ölçeğini kullanır; taramaların çizim ölçeği değişmez.",
            )),
    );
    let (low, high) = kentos_commands::CUT_HEIGHT_RANGE;
    let cut_wrong = !matches!(number(cut), Ok(None))
        && !matches!(number(cut), Ok(Some(h)) if (low..=high).contains(&h));
    let cut_box = super::group::<Message>(
        "Planın kesme yüksekliği",
        Column::new()
            .spacing(6)
            .push(container(field("1.10", cut, cut_wrong, on_cut)).width(typography::scaled(200.0)))
            .push(label::caption(format!(
                "Kat döşemesinden metre; boş: 1,10. Bu yüksekliği geçen pencere, boşluk ve niş planda kesilir; bütünüyle \
                 üstünde kalan yüksek pencere duvarı kesmez, kesikli çizilir. İTÜ'nün teknik çizim notu 1,20–1,50 m \
                 der; yeni Mimari projesi 1,20 ile açılır. {}–{} m arası.",
                format!("{low:.2}").replace('.', ","),
                format!("{high:.2}").replace('.', ",")
            ))),
    );
    let line_choice = Select::new(
        LINES.iter().map(|(_, l)| Choice::new(*l)),
        LINES.iter().position(|(g, _)| *g == lines),
        move |i| on_lines(LINES.get(i).and_then(|(g, _)| *g)),
    )
    .searchable(false);
    let lines_box = super::group::<Message>(
        "Çizgi kalınlıkları",
        Column::new()
            .spacing(6)
            .push(container(line_choice).width(typography::scaled(200.0)))
            .push(label::caption(
                "Ölçeğe göre: TS 88-23 / ISO 128-23 çizgi grupları. Duvar 1:200'de 0,25, 1:100'de 0,35, 1:50'de 0,5, \
                 1:20'de 0,7 mm; kapı, pencere ve ince çizgiler yarısı. Nesnenin kalınlığı 1:50'deki genişliği sayılır; \
                 ölçü, yazı ve akslar kendi kalınlığında kalır. Yeni Mimari projesi böyle açılır.",
            )),
    );
    let read_now = read(
        rows,
        detail,
        method,
        cut,
        lines,
        &ZoningTexts::default(),
        None,
    );
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
        .push(cut_box)
        .push(lines_box)
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
            "spaceNumberPrefixes":{"zemin":"Z-"},
            // The fire protection class is this form's (docs/adr/8124), read back from its texts as the window starts
            // them; the form's answers are their own window's (docs/adr/8077).
            "performance":{"fireProtectionClass":{"value":"Konut, düşük tehlike","basis":{"kind":"author"}}},
            "formAnswers":[{"no":1,"yes":true,"note":"Sınır yayımlanmadı"}]
        }))
        .unwrap()
    }

    /// A storey's height changed moves the levels above by as much (docs/adr/8040 §1), typed away and again too; a
    /// level below stays.
    #[test]
    fn a_storey_s_height_moves_the_levels_above() {
        let a: ArchSettings = serde_json::from_value(serde_json::json!({"levels":[
            {"id":"bodrum","name":"Bodrum kat","elevation":-3.0,"height":3.0},
            {"id":"zemin","name":"Zemin kat","elevation":0.0,"height":3.0},
            {"id":"k1","name":"1. kat","elevation":3.0,"height":3.0},
            {"id":"k2","name":"2. kat","elevation":6.0,"height":3.0}
        ]}))
        .unwrap();
        let mut form = rows(Some(&a));
        let elevations =
            |form: &[Row]| -> Vec<String> { form.iter().map(|r| r.elevation.clone()).collect() };
        let before = elevations(&form);
        for t in ["3.", "3.2", "3.24"] {
            edit(&mut form, 1, Edit::Height(t.to_owned()));
        }
        let after = elevations(&form);
        assert_eq!(after[0], before[0], "the level below stays");
        assert_eq!(after[1], before[1], "its own elevation stays");
        assert_eq!(
            [number(&after[2]).unwrap(), number(&after[3]).unwrap()],
            [Some(3.24), Some(6.24)]
        );
        // Typed away and again: from the last height that read.
        edit(&mut form, 1, Edit::Height(String::new()));
        edit(&mut form, 1, Edit::Height("3".to_owned()));
        let back = elevations(&form);
        assert_eq!(
            [number(&back[2]).unwrap(), number(&back[3]).unwrap()],
            [Some(3.0), Some(6.0)]
        );
    }

    /// Planın kesme yüksekliği as typed (docs/adr/8012): a comma or a point, empty the default, outside 0.30–3.00 m or
    /// not a number Kaydet waits.
    #[test]
    fn the_cut_height_is_read_as_typed_and_held_to_its_range() {
        let a = project();
        let form = rows(Some(&a));
        assert_eq!(
            read(
                &form,
                None,
                None,
                "1,25",
                None,
                &ZoningTexts::default(),
                Some(&a)
            )
            .unwrap()
            .cut_height,
            Some(1.25)
        );
        assert_eq!(
            read(
                &form,
                None,
                None,
                " ",
                None,
                &ZoningTexts::default(),
                Some(&a)
            )
            .unwrap()
            .cut_height,
            None
        );
        for bad in ["5", "0.1", "bir"] {
            assert!(
                read(
                    &form,
                    None,
                    None,
                    bad,
                    None,
                    &ZoningTexts::default(),
                    Some(&a)
                )
                .is_err(),
                "{bad}"
            );
        }
    }

    /// Çizgi kalınlıkları (docs/adr/8013): Ölçeğe göre writes the line groups, Nesnenin kendi kalınlığı nothing (an old
    /// drawing stays as it was).
    #[test]
    fn the_line_groups_are_written_only_by_the_scale() {
        let a = project();
        let form = rows(Some(&a));
        let by_scale = Some(kentos_commands::LineGroups::ByScale);
        assert_eq!(
            read(
                &form,
                None,
                None,
                "",
                by_scale,
                &ZoningTexts::default(),
                Some(&a)
            )
            .unwrap()
            .line_groups,
            by_scale
        );
        assert_eq!(
            read(
                &form,
                None,
                None,
                "",
                None,
                &ZoningTexts::default(),
                Some(&a)
            )
            .unwrap()
            .line_groups,
            None
        );
        assert_eq!(LINES.map(|(g, _)| g), [by_scale, None]);
    }

    #[test]
    fn project_form_preserves_exact_choices_and_only_an_explicit_prefix_edit_changes_the_start() {
        let a = project();
        let mut form = rows(Some(&a));
        assert_eq!(
            read(
                &form,
                None,
                None,
                "",
                None,
                &ZoningTexts::of(Some(&a)),
                Some(&a)
            )
            .unwrap(),
            a
        );
        edit(&mut form, 0, Edit::Name("Giriş katı".into()));
        let before = read(
            &form,
            None,
            None,
            "",
            None,
            &ZoningTexts::of(Some(&a)),
            Some(&a),
        )
        .unwrap();
        assert_eq!(before.space_first_number("zemin").as_deref(), Some("Z-001"));
        assert_eq!(before.space_names, a.space_names);
        edit(&mut form, 0, Edit::Prefix(" G- ".into()));
        let changed = read(
            &form,
            None,
            None,
            "",
            None,
            &ZoningTexts::of(Some(&a)),
            Some(&a),
        )
        .unwrap();
        assert_eq!(
            changed.space_first_number("zemin").as_deref(),
            Some("G-001")
        );
        assert_eq!(changed.space_usages, a.space_usages);
        edit(&mut form, 0, Edit::Delete);
        let cleared = read(
            &form,
            None,
            None,
            "",
            None,
            &ZoningTexts::of(Some(&a)),
            Some(&a),
        )
        .unwrap();
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
        assert!(
            read(
                &rows(Some(&a)),
                None,
                None,
                "",
                None,
                &ZoningTexts::default(),
                Some(&a)
            )
            .is_err()
        );
        change(&mut a, ChoiceEdit::Value(2, "Çalışma".into()));
        change(&mut a, ChoiceEdit::Up(2));
        assert_eq!(a.space_names, ["Mutfak", "Çalışma", "mutfak"]);
        change(&mut a, ChoiceEdit::Value(1, " Mutfak ".into()));
        assert!(
            read(
                &rows(Some(&a)),
                None,
                None,
                "",
                None,
                &ZoningTexts::default(),
                Some(&a)
            )
            .is_err()
        );
        change(&mut a, ChoiceEdit::Delete(1));
        assert_eq!(a, project());
    }
}
