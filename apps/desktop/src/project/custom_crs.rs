//! Özel koordinat sistemi (docs/adr/0168 §1–§2, §6; the web's
//! `ui/settings/CustomCrsDialog.ts`): the project's own system or its second
//! system typed as a definition: its name, its kind (a transverse Mercator,
//! a geographic system, a local system bound to a base), the kind's values,
//! its datum (the registry's, or the project's: an ellipsoid and seven
//! parameters to WGS 84), a local system's base and plane. What is typed is
//! checked field by field (`kentos_project::definition_form`, the shared
//! cases'), and a definition the registry has is said. It opens over Proje
//! ayarları, which waits under it: Tamam puts the definition in its draft,
//! Kaydet there assigns it; the drawing is not transformed.

use std::fmt;

use iced::widget::{Column, column, container, row};
use iced::{Element, Fill};
use kentos_contracts::{Convention, CrsDefinition, CrsSystem, RegistryDatum};
use kentos_project::definition_form::{
    self, AFFINE, DatumPick, ELLIPSOIDS, Form, Kind, PARAMETERS, PlaneKind, Problems,
};
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Dialog, overlay};
use kentos_ui::{label, style};

use super::choices::{CAPTIONS, Rule, field};
use crate::app::Message;
use crate::exchange::words;

/// Whose definition the window edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The project's own system.
    Own,
    /// Its second system.
    Second,
}

/// A text field of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Name,
    LatitudeOfOrigin,
    CentralMeridian,
    ScaleFactor,
    FalseEasting,
    FalseNorthing,
    DatumName,
    SemiMajor,
    InverseFlattening,
    Accuracy,
    East,
    North,
    Rotation,
    Scale,
}

/// A change to the window, or its buttons.
#[derive(Debug, Clone)]
pub enum Event {
    Text(Field, String),
    Kind(Kind),
    Datum(DatumPick),
    /// One of the classic ellipsoids by its place; none: typed.
    Ellipsoid(Option<usize>),
    Linked(bool),
    /// One of the seven parameters by its place.
    Parameter(usize, String),
    Convention(Convention),
    Base(u32),
    Plane(PlaneKind),
    /// One of the affine's six coefficients by its place.
    Affine(usize, String),
    Done,
    Cancel,
}

/// What an event leaves: the window open, closed, or done with a definition.
#[derive(Debug)]
pub enum Outcome {
    Keep,
    Close,
    Done(Target, CrsDefinition),
}

/// The window as typed, and what is wrong.
#[derive(Debug, Clone)]
pub struct Editor {
    pub target: Target,
    form: Form,
    problems: Problems,
    /// The registry's system the definition is (“EPSG:5254 … ile aynı”).
    note: Option<String>,
    /// A definition from a file whose base is itself a definition: the
    /// window cannot show that base, so the definition stays as it is until
    /// something is typed.
    kept: Option<CrsDefinition>,
}

impl Editor {
    /// The window on a definition, or on a new one (a TM on TUREF to fill in).
    pub fn new(target: Target, existing: Option<&CrsDefinition>) -> Self {
        let kept = existing
            .filter(|d| matches!(&d.system, CrsSystem::Local(l) if l.base.definition.is_some()))
            .cloned();
        let mut editor = Self {
            target,
            form: existing.map(definition_form::form_of).unwrap_or_default(),
            problems: Problems::new(),
            note: None,
            kept,
        };
        // A new window says nothing before anything is typed; a definition is checked as it is.
        if existing.is_some() && editor.kept.is_none() {
            editor.check();
        }
        editor
    }

    fn check(&mut self) {
        match definition_form::build(&self.form) {
            Ok((_, note)) => {
                self.problems.clear();
                self.note = note;
            }
            Err(p) => {
                self.problems = p;
                self.note = None;
            }
        }
    }

    /// Tamam waits while something is to be put right.
    pub fn ready(&self) -> bool {
        self.problems.is_empty()
    }

    pub fn edit(&mut self, e: Event) -> Outcome {
        let f = &mut self.form;
        match e {
            Event::Text(at, t) => {
                *match at {
                    Field::Name => &mut f.name,
                    Field::LatitudeOfOrigin => &mut f.latitude_of_origin,
                    Field::CentralMeridian => &mut f.central_meridian,
                    Field::ScaleFactor => &mut f.scale_factor,
                    Field::FalseEasting => &mut f.false_easting,
                    Field::FalseNorthing => &mut f.false_northing,
                    Field::DatumName => &mut f.datum_name,
                    Field::SemiMajor => &mut f.semi_major,
                    Field::InverseFlattening => &mut f.inverse_flattening,
                    Field::Accuracy => &mut f.accuracy,
                    Field::East => &mut f.east,
                    Field::North => &mut f.north,
                    Field::Rotation => &mut f.rotation,
                    Field::Scale => &mut f.scale,
                } = t;
            }
            Event::Kind(k) => f.kind = k,
            Event::Datum(d) => f.datum = d,
            Event::Ellipsoid(i) => {
                // The typed values start from the ellipsoid chosen before.
                if i.is_none()
                    && let Some(&(_, a, rf)) = f.ellipsoid.and_then(|k| ELLIPSOIDS.get(k))
                {
                    f.semi_major = format!("{a}");
                    f.inverse_flattening = format!("{rf}");
                }
                f.ellipsoid = i;
            }
            Event::Linked(on) => f.linked = on,
            Event::Parameter(k, t) => {
                if let Some(p) = f.parameters.get_mut(k) {
                    *p = t;
                }
            }
            Event::Convention(c) => f.convention = c,
            Event::Base(srid) => f.base = srid.to_string(),
            Event::Plane(p) => f.plane = p,
            Event::Affine(k, t) => {
                if let Some(c) = f.affine.get_mut(k) {
                    *c = t;
                }
            }
            Event::Cancel => return Outcome::Close,
            Event::Done => {
                if let Some(d) = &self.kept {
                    return Outcome::Done(self.target, d.clone());
                }
                return match definition_form::build(&self.form) {
                    Ok((d, _)) => Outcome::Done(self.target, d),
                    Err(p) => {
                        self.problems = p;
                        Outcome::Keep
                    }
                };
            }
        }
        self.kept = None;
        self.check();
        Outcome::Keep
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KindName(Kind);

impl fmt::Display for KindName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Kind::Tm => "TM izdüşümü",
            Kind::Geographic => "Coğrafi",
            Kind::Local => "Yerel (taban sisteme bağlı)",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DatumName(DatumPick);

impl fmt::Display for DatumName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            DatumPick::Registry(RegistryDatum::Turef) => "TUREF",
            DatumPick::Registry(RegistryDatum::Ed50) => "ED50",
            DatumPick::Registry(RegistryDatum::Wgs84) => "WGS 84",
            DatumPick::Custom => "Projenin datumu",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlaneName(PlaneKind);

impl fmt::Display for PlaneName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            PlaneKind::Similarity => "Benzerlik",
            PlaneKind::Affine => "Afin",
        })
    }
}

/// A text field of the form with its caption and what is wrong in it.
fn text<'a>(
    editor: &'a Editor,
    on: impl Fn(Event) -> Message + 'a,
    at: Field,
    caption: &'a str,
    key: &str,
    width: f32,
) -> Element<'a, Message> {
    let f = &editor.form;
    let value = match at {
        Field::Name => &f.name,
        Field::LatitudeOfOrigin => &f.latitude_of_origin,
        Field::CentralMeridian => &f.central_meridian,
        Field::ScaleFactor => &f.scale_factor,
        Field::FalseEasting => &f.false_easting,
        Field::FalseNorthing => &f.false_northing,
        Field::DatumName => &f.datum_name,
        Field::SemiMajor => &f.semi_major,
        Field::InverseFlattening => &f.inverse_flattening,
        Field::Accuracy => &f.accuracy,
        Field::East => &f.east,
        Field::North => &f.north,
        Field::Rotation => &f.rotation,
        Field::Scale => &f.scale,
    };
    field(
        caption,
        value,
        width,
        editor.problems.get(key).copied(),
        move |t| on(Event::Text(at, t)),
    )
}

/// The window over Proje ayarları.
pub fn view<'a>(
    editor: &'a Editor,
    on: impl Fn(Event) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let f = &editor.form;
    let mut body = Column::new().spacing(16).width(Fill);
    body = body.push(label::caption(match editor.target {
        Target::Own => {
            "Kayıtta olmayan bir sistemi projenin sistemi olarak tanımlayın: tanım projeyle saklanır ve paylaşılır. Tamam tanımı Proje ayarları'na yazar, Kaydet onu atar; koordinatlar dönüştürülmez."
        }
        Target::Second => {
            "Kayıtta olmayan bir sistemi ikinci sistem olarak tanımlayın: değerleri durum çubuğunda ve Koordinat oku'da projeninkilerin yanında gösterilir. Tamam tanımı Proje ayarları'na yazar, Kaydet onu atar; çizim dönüştürülmez."
        }
    }));
    if editor.kept.is_some() {
        body = body.push(Banner::info(
            "Bu tanımın tabanı da bir tanım (dosyadan geldi); bu pencere onu gösteremez. Bir alanı değiştirmezseniz tanım olduğu gibi kalır.",
        ));
    }
    let kinds = Segmented::new(
        [
            KindName(Kind::Tm),
            KindName(Kind::Geographic),
            KindName(Kind::Local),
        ],
        KindName(f.kind),
        {
            let on = on.clone();
            move |k: KindName| on(Event::Kind(k.0))
        },
    );
    body = body.push(
        row![
            text(editor, on.clone(), Field::Name, "Ad", "name", 280.0),
            column![label::caption("Tür"), kinds].spacing(4)
        ]
        .spacing(16),
    );
    match f.kind {
        Kind::Tm => {
            let value = |at, caption, key, width| text(editor, on.clone(), at, caption, key, width);
            body = body
                .push(section("İzdüşüm"))
                .push(
                    row![
                        value(
                            Field::CentralMeridian,
                            "Orta meridyen (°)",
                            "centralMeridian",
                            130.0
                        ),
                        value(Field::ScaleFactor, "Ölçek", "scaleFactor", 110.0),
                        value(
                            Field::LatitudeOfOrigin,
                            "Başlangıç enlemi (°)",
                            "latitudeOfOrigin",
                            130.0
                        ),
                    ]
                    .spacing(12),
                )
                .push(
                    row![
                        value(
                            Field::FalseEasting,
                            "Sağa öteleme (m)",
                            "falseEasting",
                            150.0
                        ),
                        value(
                            Field::FalseNorthing,
                            "Yukarı öteleme (m)",
                            "falseNorthing",
                            150.0
                        ),
                    ]
                    .spacing(12),
                )
                .push(label::caption(
                    "Boş ölçek 1, boş başlangıç enlemi 0'dır. Orta meridyen ve ötelemeler yazılmalı.",
                ))
                .push(datum(editor, on.clone()));
        }
        Kind::Geographic => body = body.push(datum(editor, on.clone())),
        Kind::Local => body = body.push(local(editor, on.clone())),
    }
    if let Some(note) = &editor.note {
        body = body.push(Banner::info(note.clone()));
    }
    overlay::blocking(
        Dialog::new("Özel koordinat sistemi")
            .scroll(body)
            .action(words::secondary("Vazgeç", Some(on(Event::Cancel))))
            .action(words::primary(
                "Tamam",
                editor.ready().then(|| on(Event::Done)),
            ))
            .width(760.0)
            .max_height(760.0),
    )
}

/// A part's heading in the window's body.
fn section<'a>(title: &'a str) -> Element<'a, Message> {
    iced::widget::text(title)
        .font(typography::ui_strong())
        .size(typography::body())
        .into()
}

/// The datum: the registry's three, or the project's own: its name, its
/// ellipsoid and, when it is bound to WGS 84, its seven parameters.
fn datum<'a>(
    editor: &'a Editor,
    on: impl Fn(Event) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let f = &editor.form;
    let problem = |key: &str| editor.problems.get(key).copied();
    let datums = Segmented::new(
        [
            DatumName(DatumPick::Registry(RegistryDatum::Turef)),
            DatumName(DatumPick::Registry(RegistryDatum::Ed50)),
            DatumName(DatumPick::Registry(RegistryDatum::Wgs84)),
            DatumName(DatumPick::Custom),
        ],
        DatumName(f.datum),
        {
            let on = on.clone();
            move |d: DatumName| on(Event::Datum(d.0))
        },
    );
    let c = column![section("Datum"), datums].spacing(10);
    if f.datum != DatumPick::Custom {
        return c
            .push(label::caption(
                "Kayıttaki datum: öbür datumlara EPSG'nin yollarıyla ya da projenin Datum dönüşümleri'yle varılır.",
            ))
            .into();
    }
    let names = ELLIPSOIDS
        .iter()
        .map(|(name, a, rf)| Choice::new(*name).detail(format!("a {a}, 1/f {rf}")))
        .chain(std::iter::once(Choice::new("Değerleri yazılan")));
    let pick = Select::new(names, Some(f.ellipsoid.unwrap_or(ELLIPSOIDS.len())), {
        let on = on.clone();
        move |i| on(Event::Ellipsoid((i < ELLIPSOIDS.len()).then_some(i)))
    })
    .searchable(false);
    let mut ellipsoid = row![
        text(
            editor,
            on.clone(),
            Field::DatumName,
            "Datumun adı",
            "datumName",
            220.0
        ),
        column![label::caption("Elipsoid"), container(pick).width(220)].spacing(4),
    ]
    .spacing(16);
    if f.ellipsoid.is_none() {
        ellipsoid = ellipsoid
            .push(text(
                editor,
                on.clone(),
                Field::SemiMajor,
                "a (m)",
                "semiMajor",
                130.0,
            ))
            .push(text(
                editor,
                on.clone(),
                Field::InverseFlattening,
                "1/f",
                "inverseFlattening",
                120.0,
            ));
    }
    let c = c.push(ellipsoid).push(words::check(
        f.linked,
        "WGS 84'e yedi parametreyle bağlı",
        Some(on(Event::Linked(!f.linked))),
    ));
    if !f.linked {
        return c
            .push(label::caption(
                "Bağsız datum kendi içinde kalır: başka datumdaki sistemlere değer verilmez.",
            ))
            .into();
    }
    let parameter = |k: usize| {
        let on = on.clone();
        field(
            CAPTIONS[k],
            &f.parameters[k],
            if k == 6 { 140.0 } else { 110.0 },
            problem(PARAMETERS[k]),
            move |t| on(Event::Parameter(k, t)),
        )
    };
    let rule = Segmented::new(
        [
            Rule(Convention::PositionVector),
            Rule(Convention::CoordinateFrame),
        ],
        Rule(f.convention),
        {
            let on = on.clone();
            move |r: Rule| on(Event::Convention(r.0))
        },
    );
    c.push(row![parameter(0), parameter(1), parameter(2)].spacing(12))
        .push(row![parameter(3), parameter(4), parameter(5), parameter(6)].spacing(12))
        .push(
            row![
                column![label::caption("Dönüklüklerin kuralı"), rule].spacing(4),
                text(
                    editor,
                    on.clone(),
                    Field::Accuracy,
                    "Doğruluk (m)",
                    "accuracy",
                    110.0
                ),
            ]
            .spacing(16),
        )
        .push(label::caption(
            "Boş dönüklük ve ölçek farkı 0'dır (üç parametre). Öbür datumlara WGS 84 üstünden varılır; doğruluk boşsa bilinmiyor sayılır.",
        ))
        .into()
}

/// A local system: its base (a projected system of the registry) and its
/// plane, this system's coordinates to the base's.
fn local<'a>(
    editor: &'a Editor,
    on: impl Fn(Event) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let f = &editor.form;
    let problem = |key: &str| editor.problems.get(key).copied();
    let bases: Vec<&'static crate::crs::System> = crate::crs::systems()
        .iter()
        .filter(|s| s.kind == "projected")
        .collect();
    let srids: Vec<u32> = bases.iter().map(|s| s.srid).collect();
    let selected = bases
        .iter()
        .position(|s| s.srid.to_string() == f.base.trim());
    let pick = Select::new(
        bases.iter().map(|s| {
            Choice::new(s.name.clone())
                .detail(format!("EPSG:{}", s.srid))
                .shown(crate::crs::title(s))
        }),
        selected,
        {
            let on = on.clone();
            move |i| on(Event::Base(srids.get(i).copied().unwrap_or(0)))
        },
    )
    .placeholder("Taban sistemi seçin…");
    let mut base = column![label::caption("Taban sistem"), container(pick).width(320)].spacing(4);
    if let Some(p) = problem("base") {
        base = base.push(label::caption(p).style(style::text::danger));
    }
    let planes = Segmented::new(
        [
            PlaneName(PlaneKind::Similarity),
            PlaneName(PlaneKind::Affine),
        ],
        PlaneName(f.plane),
        {
            let on = on.clone();
            move |p: PlaneName| on(Event::Plane(p.0))
        },
    );
    let c = column![
        section("Taban ve düzlem"),
        row![
            base,
            column![label::caption("Düzlem dönüşümü"), planes].spacing(4)
        ]
        .spacing(16),
    ]
    .spacing(10);
    match f.plane {
        PlaneKind::Similarity => {
            let value = |at, caption, key, width| text(editor, on.clone(), at, caption, key, width);
            c.push(
                row![
                    value(Field::East, "Sağa öteleme (m)", "east", 150.0),
                    value(Field::North, "Yukarı öteleme (m)", "north", 150.0),
                    value(Field::Rotation, "Dönüklük (°)", "rotation", 110.0),
                    value(Field::Scale, "Ölçek", "scale", 110.0),
                ]
                .spacing(12),
            )
            .push(label::caption(
                "Bu sistemin noktası saat yönünün tersine döndürülür, ölçeklenir, sonra ötelenir: tabandaki yeri çıkar. Boş dönüklük 0, boş ölçek 1'dir.",
            ))
            .into()
        }
        PlaneKind::Affine => {
            let coefficient = |k: usize| {
                let on = on.clone();
                field(
                    AFFINE[k],
                    &f.affine[k],
                    150.0,
                    problem(AFFINE[k]),
                    move |t| on(Event::Affine(k, t)),
                )
            };
            c.push(row![coefficient(0), coefficient(1), coefficient(2)].spacing(12))
                .push(row![coefficient(3), coefficient(4), coefficient(5)].spacing(12))
                .push(label::caption(
                    "Tabanda sağa = a·sağa + b·yukarı + c, tabanda yukarı = d·sağa + e·yukarı + f; sağa ve yukarı bu sistemin koordinatlarıdır.",
                ))
                .into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(editor: &mut Editor, at: Field, t: &str) {
        assert!(matches!(
            editor.edit(Event::Text(at, t.to_owned())),
            Outcome::Keep
        ));
    }

    /// A new window says nothing until something is typed; then each field
    /// says what is wrong, and Tamam gives the definition the shared rules
    /// build (definition_form.rs).
    #[test]
    fn a_new_definition_is_typed_checked_and_given() {
        let mut editor = Editor::new(Target::Own, None);
        assert!(editor.ready(), "nothing said before typing");
        typed(&mut editor, Field::Name, "Şantiye");
        assert_eq!(
            editor.problems.get("centralMeridian"),
            Some(&definition_form::NUMBER)
        );
        assert!(!editor.ready());
        assert!(matches!(editor.edit(Event::Done), Outcome::Keep));
        typed(&mut editor, Field::CentralMeridian, "30");
        typed(&mut editor, Field::FalseEasting, "500000");
        typed(&mut editor, Field::FalseNorthing, "0");
        typed(&mut editor, Field::ScaleFactor, "1");
        assert!(editor.ready());
        // TM30 on TUREF with every value the registry's: the window says so.
        assert_eq!(
            editor.note.as_deref(),
            Some("EPSG:5254 (TUREF / TM30) ile aynı; kayıttakini seçin.")
        );
        typed(&mut editor, Field::FalseNorthing, "-4000000");
        assert_eq!(editor.note, None);
        let Outcome::Done(Target::Own, d) = editor.edit(Event::Done) else {
            panic!("Tamam gives the definition");
        };
        assert_eq!(d.name, "Şantiye");
        assert!(matches!(&d.system, CrsSystem::Tm(t) if t.false_northing == -4_000_000.0));
        assert!(d.problem().is_none());
    }

    /// The datum and the ellipsoid: typed values start from the ellipsoid
    /// chosen before; a datum without its link keeps no parameters.
    #[test]
    fn the_datum_and_its_ellipsoid_are_chosen_or_typed() {
        let mut editor = Editor::new(Target::Second, None);
        typed(&mut editor, Field::Name, "Bessel TM");
        typed(&mut editor, Field::CentralMeridian, "27");
        typed(&mut editor, Field::FalseEasting, "500000");
        typed(&mut editor, Field::FalseNorthing, "0");
        editor.edit(Event::Datum(DatumPick::Custom));
        assert_eq!(
            editor.problems.get("datumName"),
            Some(&definition_form::DATUM_NAME)
        );
        typed(&mut editor, Field::DatumName, "Bessel datumu");
        editor.edit(Event::Ellipsoid(Some(3)));
        editor.edit(Event::Ellipsoid(None));
        assert_eq!(editor.form.semi_major, "6377397.155");
        assert_eq!(editor.form.inverse_flattening, "299.1528128");
        for (k, v) in ["674.374", "15.056", "405.346"].into_iter().enumerate() {
            editor.edit(Event::Parameter(k, v.to_owned()));
        }
        editor.edit(Event::Linked(false));
        let Outcome::Done(Target::Second, d) = editor.edit(Event::Done) else {
            panic!("Tamam gives the definition");
        };
        let CrsSystem::Tm(t) = &d.system else {
            panic!("a TM");
        };
        let datum = t.custom_datum.as_deref().expect("the project's datum");
        assert_eq!(datum.ellipsoid.semi_major, 6_377_397.155);
        assert!(datum.to_wgs84.is_none(), "a datum without its link");
    }

    /// A definition opens as it is; one whose base is a definition (from a
    /// file) stays as it is unless something is typed, and Vazgeç gives
    /// nothing.
    #[test]
    fn a_definition_opens_as_it_is() {
        let file: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/crs/v1/definition-form.json"
        ))
        .expect("the cases read");
        let d: CrsDefinition = file["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find_map(|c| serde_json::from_value(c["definition"].clone()).ok())
            .expect("a definition");
        let mut editor = Editor::new(Target::Own, Some(&d));
        assert!(editor.ready());
        assert!(matches!(editor.edit(Event::Done), Outcome::Done(Target::Own, got) if got == d));
        assert!(matches!(editor.edit(Event::Cancel), Outcome::Close));

        let based = CrsDefinition {
            name: "İç içe".to_owned(),
            system: CrsSystem::Local(kentos_contracts::LocalDefinition {
                base: kentos_contracts::CrsBase {
                    srid: None,
                    definition: Some(Box::new(d.clone())),
                },
                plane: kentos_contracts::CrsPlane::Similarity {
                    east: 1.0,
                    north: 2.0,
                    rotation: 0.0,
                    scale: 1.0,
                },
            }),
        };
        let mut editor = Editor::new(Target::Second, Some(&based));
        assert!(editor.ready(), "kept, nothing said");
        assert!(
            matches!(editor.edit(Event::Done), Outcome::Done(Target::Second, got) if got == based)
        );
        editor.edit(Event::Text(Field::Name, "Başka".to_owned()));
        // Typed: the window's own base (none chosen) is asked for.
        assert_eq!(editor.problems.get("base"), Some(&definition_form::BASE));
    }

    /// The editor over Proje ayarları, if it is open.
    fn open_editor(app: &crate::app::App) -> Option<&Editor> {
        match &app.project {
            Some(super::super::Window::Settings(s)) => s.custom.as_ref(),
            _ => None,
        }
    }

    /// Clicks the text field under a caption and types into it.
    fn type_into(
        snapshot: &mut kentos_ui::snapshot::Snapshot,
        app: &mut crate::app::App,
        caption: &str,
        typed: &str,
    ) {
        use crate::app::App;
        use kentos_ui::snapshot::Input;
        // The window is drawn over the app: its text is the last of the same words.
        let r = crate::files_testing::find_texts(snapshot, app, caption)
            .last()
            .copied()
            .unwrap_or_else(|| panic!("the field {caption}"));
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let at = iced::Point::new(r.x + 12.0, r.y + r.height + 16.0);
        snapshot.input(app, App::view, &mut update, Input::Click(at));
        snapshot.input(app, App::view, &mut update, Input::Type(typed.to_owned()));
    }

    /// Clicks a text where it is drawn.
    fn click(
        snapshot: &mut kentos_ui::snapshot::Snapshot,
        app: &mut crate::app::App,
        caption: &str,
    ) {
        use crate::app::App;
        use kentos_ui::snapshot::Input;
        let r = crate::files_testing::find_texts(snapshot, app, caption)
            .last()
            .copied()
            .unwrap_or_else(|| panic!("the text {caption}"));
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.input(app, App::view, &mut update, Input::Click(r.center()));
    }

    /// With the mouse and the keyboard (docs/adr/0168 §6): Özel sistem…
    /// under the list opens the window over Proje ayarları; what is typed is
    /// checked, Tamam puts the definition in the draft (its card says so,
    /// with Düzenle), Esc over Düzenle's window closes only it, and Kaydet
    /// assigns the definition without transforming the drawing.
    #[test]
    fn a_definition_is_made_with_the_mouse_and_kaydet_assigns_it() {
        use crate::app::App;
        use iced::Size;

        let mut app = crate::files_testing::app_with_drawing();
        let before = app.document.as_ref().expect("a drawing").entity_count();
        let _ = app.update(Message::Run("crs.set"));
        let mut snapshot = crate::files_testing::offscreen(Size::new(1440.0, 900.0));
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        click(&mut snapshot, &mut app, "Özel sistem…");
        assert!(
            open_editor(&app).is_some(),
            "the window over Proje ayarları"
        );
        type_into(&mut snapshot, &mut app, "Ad", "Şantiye");
        // The first field typed says what the others still want.
        assert!(!open_editor(&app).expect("open").ready());
        type_into(&mut snapshot, &mut app, "Orta meridyen (°)", "30");
        type_into(&mut snapshot, &mut app, "Sağa öteleme (m)", "500000");
        type_into(&mut snapshot, &mut app, "Yukarı öteleme (m)", "-4000000");
        let editor = open_editor(&app).expect("open");
        assert!(editor.ready(), "{:?}", editor.problems);
        click(&mut snapshot, &mut app, "Tamam");
        assert!(open_editor(&app).is_none());
        // The draft's system is the definition, shown on the card with Düzenle.
        snapshot.settle(&mut app, App::view, &mut update);
        assert!(crate::files_testing::find_text(&mut snapshot, &app, "Şantiye").is_some());
        click(&mut snapshot, &mut app, "Düzenle");
        assert!(open_editor(&app).is_some());
        // Esc as the app's keys come (its subscription; wizard/tests.rs).
        let _ = app.update(Message::Key(crate::keys::KeyPress {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            physical: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            modifiers: iced::keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }));
        assert!(open_editor(&app).is_none(), "Esc closes the window over it");
        assert!(app.project.is_some(), "Proje ayarları stays");
        click(&mut snapshot, &mut app, "Kaydet");
        assert!(app.project.is_none());
        let doc = app.document.as_ref().expect("a drawing");
        let settings = doc.settings();
        assert_eq!(settings.srid, crate::crs::LOCAL_SRID);
        let d = settings.custom_crs.as_ref().expect("the definition");
        assert_eq!(d.name, "Şantiye");
        assert!(
            matches!(&d.system, CrsSystem::Tm(t) if t.central_meridian == 30.0 && t.false_northing == -4_000_000.0)
        );
        assert_eq!(doc.entity_count(), before, "nothing transformed");
        assert!(app.log.said(
            kentos_interaction::Level::Success,
            "Proje koordinat sistemi Şantiye (özel sistem) olarak atandı. Koordinat değerleri değiştirilmedi."
        ));
    }

    /// A second definition from the second system's list: Özel sistem… under
    /// it, then the row of the definition, chosen again after another system;
    /// Kaydet keeps only the one chosen.
    #[test]
    fn a_second_definition_is_chosen_from_its_list() {
        use crate::project::{SettingsEvent, settings_message};

        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(Message::Run("crs.set"));
        let send = |app: &mut crate::app::App, e: SettingsEvent| {
            let _ = app.update(settings_message(e));
        };
        send(&mut app, SettingsEvent::NewDefinition(Target::Second));
        let custom = |app: &mut crate::app::App, e: Event| {
            let _ = app.update(settings_message(SettingsEvent::Custom(e)));
        };
        custom(&mut app, Event::Text(Field::Name, "Belediye".to_owned()));
        custom(&mut app, Event::Kind(Kind::Local));
        custom(&mut app, Event::Base(5254));
        custom(&mut app, Event::Text(Field::East, "412000".to_owned()));
        custom(&mut app, Event::Text(Field::North, "4521000".to_owned()));
        custom(&mut app, Event::Text(Field::Rotation, "0,25".to_owned()));
        custom(&mut app, Event::Done);
        // Another system takes its place in the draft; its row brings it back.
        send(&mut app, SettingsEvent::Second(Some(2320)));
        send(&mut app, SettingsEvent::SecondDefined);
        send(&mut app, SettingsEvent::Save);
        let settings = app.document.as_ref().expect("a drawing").settings();
        assert_eq!(settings.second_srid, None);
        let d = settings
            .second_custom_crs
            .as_ref()
            .expect("the second definition");
        assert_eq!(d.name, "Belediye");
        assert!(matches!(&d.system, CrsSystem::Local(l) if l.base.srid == Some(5254)));
        assert!(app.log.said(
            kentos_interaction::Level::Success,
            "İkinci koordinat sistemi: Belediye (özel sistem). Çizim dönüştürülmedi."
        ));
    }

    /// Pictures for the owner: the window new and filled (a TM on the
    /// project's datum, a local system with its affine), what is wrong said
    /// under its field, and Proje ayarları with the definition chosen (its
    /// card, Düzenle, its row) and the second system's list with Özel
    /// sistem…; light at 1440×900, dark at 1100×650 (.run/shots/ozel-crs-*).
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use crate::app::App;
        use crate::project::{SettingsEvent, settings_message};
        use iced::Size;
        use kentos_ui::snapshot::{Input, Snapshot};

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let shots: [(&str, &[Event]); 4] = [
            ("yeni", &[]),
            (
                "hata",
                &[
                    Event::Text(Field::Name, "Şantiye".to_owned()),
                    Event::Text(Field::CentralMeridian, "300".to_owned()),
                    Event::Text(Field::ScaleFactor, "0".to_owned()),
                ],
            ),
            (
                "tm",
                &[
                    Event::Text(Field::Name, "Bessel TM27".to_owned()),
                    Event::Text(Field::CentralMeridian, "27".to_owned()),
                    Event::Text(Field::ScaleFactor, "1".to_owned()),
                    Event::Text(Field::FalseEasting, "500000".to_owned()),
                    Event::Text(Field::FalseNorthing, "0".to_owned()),
                    Event::Datum(DatumPick::Custom),
                    Event::Text(Field::DatumName, "Bessel datumu".to_owned()),
                    Event::Ellipsoid(Some(3)),
                    Event::Parameter(0, "674.374".to_owned()),
                    Event::Parameter(1, "15.056".to_owned()),
                    Event::Parameter(2, "405.346".to_owned()),
                    Event::Text(Field::Accuracy, "1".to_owned()),
                ],
            ),
            (
                "yerel",
                &[
                    Event::Text(Field::Name, "Belediye yerel".to_owned()),
                    Event::Kind(Kind::Local),
                    Event::Base(5254),
                    Event::Plane(PlaneKind::Affine),
                    Event::Affine(0, "1.0000215".to_owned()),
                    Event::Affine(1, "-0.0003871".to_owned()),
                    Event::Affine(2, "412000".to_owned()),
                    Event::Affine(3, "0.0003871".to_owned()),
                    Event::Affine(4, "1.0000215".to_owned()),
                    Event::Affine(5, "4521000".to_owned()),
                ],
            ),
        ];
        for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
            let picture = |app: &mut App, name: &str, click: Option<&str>| {
                app.follow.flash = None;
                let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(app, App::view, &mut update);
                if let Some(caption) = click
                    && let Some(r) = crate::files_testing::find_texts(&mut snapshot, app, caption)
                        .last()
                        .copied()
                {
                    snapshot.input(app, App::view, &mut update, Input::Click(r.center()));
                }
                let file = out.join(format!("ozel-crs-{name}-{w}x{h}-{theme}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            };
            let fresh = || {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                let _ = app.update(Message::Run("crs.set"));
                app
            };
            for (name, events) in &shots {
                let mut app = fresh();
                let _ = app.update(settings_message(SettingsEvent::NewDefinition(Target::Own)));
                for e in events.iter() {
                    let _ = app.update(settings_message(SettingsEvent::Custom(e.clone())));
                }
                picture(&mut app, name, None);
            }
            // Proje ayarları with the definition chosen, and the second system's list open.
            let mut app = fresh();
            let _ = app.update(settings_message(SettingsEvent::NewDefinition(Target::Own)));
            for e in shots[2].1 {
                let _ = app.update(settings_message(SettingsEvent::Custom(e.clone())));
            }
            let _ = app.update(settings_message(SettingsEvent::Custom(Event::Done)));
            picture(&mut app, "ayarlar", None);
            let _ = app.update(settings_message(SettingsEvent::NewDefinition(
                Target::Second,
            )));
            for e in shots[3].1 {
                let _ = app.update(settings_message(SettingsEvent::Custom(e.clone())));
            }
            let _ = app.update(settings_message(SettingsEvent::Custom(Event::Done)));
            picture(&mut app, "ikinci", Some("Belediye yerel (özel sistem)"));
        }
    }
}
