//! Özel koordinat sistemi (docs/adr/0168 §1–§2, §6; the web's
//! `ui/settings/CustomCrsDialog.ts`): the project's own system or its second
//! system typed as a definition: its name, its kind (a transverse Mercator,
//! a geographic system, a local system bound to a base), the kind's values,
//! its datum (the registry's, or the project's: an ellipsoid and seven
//! parameters to WGS 84), a local system's base and plane. What is typed is
//! checked field by field (`kentos_project::definition_form`, the shared
//! cases'), and a definition the registry has is said, with Kayıttakini
//! seç. A WKT or PROJ text pasted, or a `.prj` file, fills the fields
//! (docs/adr/0168 §5); the definition is copied as WKT or PROJ; a Deneme
//! noktası shows where a point typed in it is in WGS 84 and in the project's
//! other system, with the project's datum choices. A local system's plane is
//! found from points known in both systems (Ortak noktalardan hesapla:
//! Vektör oturtma's solution, its residuals and m0). It opens over Proje
//! ayarları, which waits under it: Tamam puts the definition in its draft,
//! Kaydet there assigns it; the drawing is not transformed.

use std::fmt;
use std::path::PathBuf;

use iced::widget::{Column, column, container, row, text_editor};
use iced::{Element, Fill, Length, Task};
use kentos_contracts::{Convention, CrsDefinition, CrsSystem, RegistryDatum};
use kentos_geometry_core::crs as core;
use kentos_project::definition_form::{
    self, AFFINE, DatumPick, ELLIPSOIDS, FitRow, Form, Kind, PARAMETERS, PlaneFit, PlaneKind,
    Problems,
};
use kentos_project::systems::definition_system;
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Dialog, overlay};
use kentos_ui::{label, style};

use super::choices::{CAPTIONS, Rule, field};
use crate::app::Message;
use crate::calc::convert::{ConvertFormat, convert_point, error_text};
use crate::calc::grid::{self, Col, Mark, Owner, Table};
use crate::calc::traverse::mm_text;
use crate::exchange::words;

/// The largest `.prj` file read (a definition is a few hundred bytes).
const PRJ_LIMIT: u64 = 1 << 20;

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
    /// WKT ya da PROJ'dan al: the box edited, read, or a `.prj` file asked for and picked.
    Paste(text_editor::Action),
    Read,
    PickFile,
    Picked(Option<PathBuf>),
    /// The definition as text, to the clipboard.
    Copy(Text),
    /// Kayıttakini seç: the registry's system the definition is.
    Registry,
    /// Deneme noktası's two values.
    Trial(usize, String),
    /// Ortak noktalardan hesapla's table: a cell typed, a paste (the field's
    /// own, then the clipboard's lines), Enter, a row added or removed; and
    /// Düzleme yaz.
    Point(usize, usize, String),
    PointPaste(usize, usize, String),
    PointPasted(usize, usize, Option<String>),
    PointSubmit(usize, usize),
    PointAdd,
    PointRemove(usize),
    FitWrite,
    Done,
    Cancel,
}

/// The texts a definition is copied as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    Wkt,
    Proj,
}

/// What an event leaves: the window open, closed, or done with a definition;
/// a file to ask for, a text for the clipboard, the registry's system chosen.
#[derive(Debug)]
pub enum Outcome {
    Keep,
    Close,
    Done(Target, CrsDefinition),
    PickFile,
    Copy(String, &'static str),
    Registry(Target, u32),
    /// The keyboard moved, or the clipboard read, in the common points' table.
    Focus(Task<Message>),
}

/// Where a trial point is compared: the project's other system (named),
/// its datum choices and how it writes points.
#[derive(Debug)]
pub struct Context {
    pub reference: Option<(String, core::System)>,
    pub choices: Vec<core::Choice>,
    pub format: ConvertFormat,
}

/// The window as typed, and what is wrong.
#[derive(Debug)]
pub struct Editor {
    pub target: Target,
    form: Form,
    problems: Problems,
    /// The registry's system the definition is (“EPSG:5254 … ile aynı”).
    note: Option<String>,
    same: Option<u32>,
    /// A definition from a file whose base is itself a definition: the
    /// window cannot show that base, so the definition stays as it is until
    /// something is typed.
    kept: Option<CrsDefinition>,
    /// The text pasted, and what reading it said: the name read or why not.
    paste: text_editor::Content,
    read: Option<Result<String, String>>,
    /// A grid the registry has on the text's own datum (said after a read).
    grid: Option<String>,
    trial: [String; 2],
    context: Context,
    /// Ortak noktalardan hesapla's rows and their solution.
    points: Points,
}

/// The common points' columns: Kullan, this system's east and north, the
/// base's, the residuals (mm); named as the project's type names its axes.
const POINT_COLUMNS_YX: [Col; 8] = [
    point_col("Kullan", None, false),
    point_col("Yerel Y", None, true),
    point_col("Yerel X", None, true),
    point_col("Taban Y", None, true),
    point_col("Taban X", None, true),
    point_col("vY", Some("mm"), true),
    point_col("vX", Some("mm"), true),
    point_col("v", Some("mm"), true),
];
const POINT_COLUMNS_XY: [Col; 8] = [
    point_col("Kullan", None, false),
    point_col("Yerel X", None, true),
    point_col("Yerel Y", None, true),
    point_col("Taban X", None, true),
    point_col("Taban Y", None, true),
    point_col("vX", Some("mm"), true),
    point_col("vY", Some("mm"), true),
    point_col("v", Some("mm"), true),
];
const USE: usize = 0;
const RESIDUAL: usize = 5;

const fn point_col(label: &'static str, unit: Option<&'static str>, numeric: bool) -> Col {
    Col {
        label,
        unit,
        numeric,
    }
}

/// Ortak noktalardan hesapla's table (the Hesap windows' `grid`): the rows
/// as typed with their residuals, the solution, the rows left out.
#[derive(Debug)]
pub struct Points {
    rows: Vec<[String; 8]>,
    fit: Option<Result<PlaneFit, String>>,
    skipped: Vec<usize>,
}

impl Default for Points {
    fn default() -> Self {
        Self {
            rows: vec![Default::default(); 4],
            fit: None,
            skipped: Vec::new(),
        }
    }
}

impl Points {
    /// The rows as the shared rules read them.
    fn typed(&self) -> Vec<FitRow> {
        self.rows
            .iter()
            .map(|r| {
                [
                    r[1].clone(),
                    r[2].clone(),
                    r[3].clone(),
                    r[4].clone(),
                    r[USE].clone(),
                ]
            })
            .collect()
    }

    /// The rows solved again, the residuals into their cells (mm).
    fn solve(&mut self, kind: PlaneKind) {
        for r in &mut self.rows {
            for cell in &mut r[RESIDUAL..] {
                cell.clear();
            }
        }
        let typed = self.typed();
        self.skipped = definition_form::unread_rows(&typed);
        let any = typed.len() > self.skipped.len()
            && typed
                .iter()
                .any(|r| r[..4].iter().any(|v| !v.trim().is_empty()));
        self.fit = any.then(|| definition_form::fit_plane(&typed, kind));
        if let Some(Ok(fit)) = &self.fit {
            for (&r, [vx, vy, v]) in fit.rows.iter().zip(&fit.residuals) {
                let row = &mut self.rows[r];
                row[RESIDUAL] = kentos_interaction::fixed(vx * 1000.0, 1);
                row[RESIDUAL + 1] = kentos_interaction::fixed(vy * 1000.0, 1);
                row[RESIDUAL + 2] = kentos_interaction::fixed(v * 1000.0, 1);
            }
        }
    }

    /// The used pair with the largest residual: its row.
    fn worst(&self) -> Option<usize> {
        let fit = self.fit.as_ref()?.as_ref().ok()?;
        fit.m0?;
        fit.rows
            .iter()
            .zip(&fit.residuals)
            .filter(|(r, _)| self.rows.get(**r).is_some_and(|row| row[USE] != "0"))
            .max_by(|a, b| a.1[2].total_cmp(&b.1[2]))
            .map(|(r, _)| *r)
    }
}

impl Table for Points {
    fn columns(&self) -> usize {
        8
    }

    fn rows(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, row: usize, col: usize) -> &str {
        self.rows.get(row).map_or("", |r| r[col].as_str())
    }

    fn set(&mut self, row: usize, col: usize, text: String) {
        if let Some(r) = self.rows.get_mut(row) {
            r[col] = text;
        }
    }

    fn readonly(&self, _row: usize, col: usize) -> bool {
        col >= RESIDUAL
    }

    fn check(&self, col: usize) -> bool {
        col == USE
    }

    fn mark(&self, row: usize) -> Option<Mark> {
        if self.rows.get(row).is_some_and(|r| r[USE] == "0") {
            return Some(Mark::Off);
        }
        (self.worst() == Some(row)).then_some(Mark::Worst)
    }

    fn insert_after(&mut self, row: usize) {
        let at = (row + 1).min(self.rows.len());
        self.rows.insert(at, Default::default());
    }

    fn can_remove(&self, _row: usize) -> bool {
        self.rows.len() > 1
    }

    fn remove(&mut self, row: usize) {
        if self.rows.len() > 1 && row < self.rows.len() {
            self.rows.remove(row);
        }
    }
}

/// The common points' table as Proje ayarları' window owns it: its cells
/// and messages.
#[derive(Clone, Copy, Debug)]
struct CommonPoints;

fn point_message(e: Event) -> Message {
    crate::project::settings_message(crate::project::SettingsEvent::Custom(e))
}

impl Owner for CommonPoints {
    fn cell_id(self, row: usize, col: usize) -> iced::widget::Id {
        iced::widget::Id::from(format!("ozel-crs-nokta-{row}-{col}"))
    }
    fn cell(self, row: usize, col: usize, text: String) -> Message {
        point_message(Event::Point(row, col, text))
    }
    fn paste(self, row: usize, col: usize, text: String) -> Message {
        point_message(Event::PointPaste(row, col, text))
    }
    fn submit(self, row: usize, col: usize) -> Message {
        point_message(Event::PointSubmit(row, col))
    }
    fn remove_row(self, row: usize) -> Message {
        point_message(Event::PointRemove(row))
    }
    fn add_row(self) -> Message {
        point_message(Event::PointAdd)
    }
    fn add_label(self) -> &'static str {
        "Nokta ekle"
    }
}

impl Editor {
    /// The window on a definition, or on a new one (a TM on TUREF to fill in).
    pub fn new(target: Target, existing: Option<&CrsDefinition>, context: Context) -> Self {
        let kept = existing
            .filter(|d| matches!(&d.system, CrsSystem::Local(l) if l.base.definition.is_some()))
            .cloned();
        let mut editor = Self {
            target,
            form: existing.map(definition_form::form_of).unwrap_or_default(),
            problems: Problems::new(),
            note: None,
            same: None,
            kept,
            paste: text_editor::Content::new(),
            read: None,
            grid: None,
            trial: Default::default(),
            context,
            points: Points::default(),
        };
        // A new window says nothing before anything is typed; a definition is checked as it is.
        if existing.is_some() && editor.kept.is_none() {
            editor.check();
        }
        editor
    }

    fn check(&mut self) {
        match definition_form::build(&self.form) {
            Ok((d, note)) => {
                self.problems.clear();
                self.note = note;
                self.same = definition_form::same_srid(&d);
            }
            Err(p) => {
                self.problems = p;
                self.note = None;
                self.same = None;
            }
        }
    }

    /// The definition as it stands: the one kept, or the form's when it builds.
    fn definition(&self) -> Option<CrsDefinition> {
        match &self.kept {
            Some(d) => Some(d.clone()),
            None => definition_form::build(&self.form).ok().map(|(d, _)| d),
        }
    }

    /// A text read into the fields, or why not (the shared cases').
    fn take_text(&mut self, text: &str) {
        match definition_form::read(text) {
            Ok(read) => {
                let local_on_definition = matches!(
                    &read.definition.system,
                    CrsSystem::Local(l) if l.base.definition.is_some()
                );
                self.form = definition_form::form_of(&read.definition);
                self.read = Some(Ok(format!(
                    "“{}” okundu; alanlar metinden dolduruldu.",
                    read.definition.name
                )));
                self.grid = read.note;
                self.kept = local_on_definition.then_some(read.definition);
                self.problems.clear();
                self.note = None;
                self.same = None;
                if self.kept.is_none() {
                    self.check();
                }
            }
            Err(why) => self.read = Some(Err(why)),
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
            Event::Plane(p) => {
                f.plane = p;
                self.points.solve(p);
            }
            Event::Affine(k, t) => {
                if let Some(c) = f.affine.get_mut(k) {
                    *c = t;
                }
            }
            Event::Paste(action) => {
                self.paste.perform(action);
                return Outcome::Keep;
            }
            Event::Read => {
                let text = self.paste.text();
                self.take_text(&text);
                return Outcome::Keep;
            }
            Event::PickFile => return Outcome::PickFile,
            Event::Picked(None) => return Outcome::Keep,
            Event::Picked(Some(path)) => {
                match std::fs::metadata(&path) {
                    Ok(m) if m.len() > PRJ_LIMIT => {
                        self.read = Some(Err(
                            "Dosya bir tanım için çok büyük (en çok 1 MiB); .prj dosyasını seçin."
                                .to_owned(),
                        ));
                    }
                    _ => match std::fs::read_to_string(&path) {
                        Ok(text) => {
                            self.paste = text_editor::Content::with_text(text.trim());
                            self.take_text(&text);
                        }
                        Err(e) => {
                            self.read = Some(Err(format!(
                                "Dosya okunamadı ({e}); metni kutuya yapıştırmayı deneyin."
                            )));
                        }
                    },
                }
                return Outcome::Keep;
            }
            Event::Copy(what) => {
                let text = self.definition().and_then(|d| match what {
                    Text::Wkt => definition_form::wkt(&d),
                    Text::Proj => definition_form::proj(&d),
                });
                return match (text, what) {
                    (Some(t), Text::Wkt) => Outcome::Copy(t, "WKT"),
                    (Some(t), Text::Proj) => Outcome::Copy(t, "PROJ dizesi"),
                    (None, _) => Outcome::Keep,
                };
            }
            Event::Registry => {
                return match self.same {
                    Some(srid) => Outcome::Registry(self.target, srid),
                    None => Outcome::Keep,
                };
            }
            Event::Trial(i, t) => {
                if let Some(v) = self.trial.get_mut(i) {
                    *v = t;
                }
                return Outcome::Keep;
            }
            Event::Point(row, col, t) => {
                self.points.set(row, col, t);
                self.points.solve(self.form.plane);
                return Outcome::Keep;
            }
            Event::PointPaste(row, col, contents) => {
                // The field's own paste stands until the clipboard says it held a table.
                self.points.set(row, col, contents);
                self.points.solve(self.form.plane);
                return Outcome::Focus(
                    iced::clipboard::read()
                        .map(move |t| point_message(Event::PointPasted(row, col, t))),
                );
            }
            Event::PointPasted(row, col, raw) => {
                let at = raw.and_then(|raw| grid::paste(&mut self.points, row, col, &raw));
                self.points.solve(self.form.plane);
                return match at {
                    Some(at) => Outcome::Focus(iced::widget::operation::focus(
                        CommonPoints.cell_id(at, col),
                    )),
                    None => Outcome::Keep,
                };
            }
            Event::PointSubmit(row, col) => {
                let task = grid::submit(&mut self.points, CommonPoints, row, col);
                return Outcome::Focus(task);
            }
            Event::PointAdd => return Outcome::Focus(grid::add(&mut self.points, CommonPoints)),
            Event::PointRemove(row) => {
                self.points.remove(row);
                self.points.solve(self.form.plane);
                return Outcome::Keep;
            }
            Event::FitWrite => {
                let Some(Ok(fit)) = &self.points.fit else {
                    return Outcome::Keep;
                };
                let plane = fit.plane.clone();
                definition_form::plane_texts(f, &plane);
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
        self.grid = None;
        self.check();
        Outcome::Keep
    }

    /// Deneme noktası: the point in WGS 84 and in the project's other system
    /// (a line each: the system and the values, how sure), or why not.
    fn trial_lines(&self) -> Result<Vec<(String, String, String)>, String> {
        if self.trial.iter().all(|t| t.trim().is_empty()) {
            return Ok(Vec::new());
        }
        let from = self
            .definition()
            .as_ref()
            .and_then(definition_system)
            .ok_or_else(|| "Önce tanımı tamamlayın; nokta onunla dönüştürülür.".to_owned())?;
        let c = &self.context;
        let wgs84 = core::System::Geographic {
            datum: core::Datum::Wgs84,
        };
        let mut targets = vec![("WGS 84".to_owned(), wgs84)];
        targets.extend(c.reference.iter().cloned());
        let mut lines = Vec::new();
        for (name, to) in targets {
            let [a, b] = &self.trial;
            match convert_point(&from, &to, &c.choices, a, b, &c.format) {
                Ok(p) => {
                    let values = p
                        .values
                        .iter()
                        .map(|(k, v)| format!("{k} {v}"))
                        .collect::<Vec<_>>()
                        .join("   ");
                    lines.push((name, values, p.accuracy));
                }
                Err(e) => lines.push((name, error_text(e, &from, &c.format), String::new())),
            }
        }
        Ok(lines)
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
    body = body.push(paste(editor, on.clone()));
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
        let banner = Banner::info(note.clone());
        body = body.push(match editor.same {
            Some(_) => banner.action("Kayıttakini seç", on(Event::Registry)),
            None => banner,
        });
    }
    if let Some(grid) = &editor.grid {
        body = body.push(Banner::info(grid.clone()));
    }
    body = body.push(trial(editor, on.clone()));
    let whole = editor.definition();
    let local = whole
        .as_ref()
        .is_some_and(|d| matches!(d.system, CrsSystem::Local(_)));
    let copy = |caption: &'a str, what: Text, can: bool| {
        words::secondary(caption, can.then(|| on(Event::Copy(what))))
    };
    let proj = copy("PROJ olarak kopyala", Text::Proj, whole.is_some() && !local);
    // PROJ cannot write a system derived from another (docs/adr/0168 §5): said where the button waits.
    let proj = if local {
        kentos_ui::widget::tip(
            proj,
            kentos_ui::widget::Tip::new(
                "Yerel sistem PROJ dizesiyle yazılamaz; WKT olarak kopyalayın.",
            ),
            iced::widget::tooltip::Position::Top,
        )
    } else {
        proj
    };
    overlay::blocking(
        Dialog::new("Özel koordinat sistemi")
            .scroll(body)
            .aside(copy("WKT olarak kopyala", Text::Wkt, whole.is_some()))
            .aside(proj)
            .action(words::secondary("Vazgeç", Some(on(Event::Cancel))))
            .action(words::primary(
                "Tamam",
                editor.ready().then(|| on(Event::Done)),
            ))
            // As wide as Proje ayarları: the common points' national coordinates fit their columns.
            .width(900.0)
            .max_height(760.0),
    )
}

/// WKT ya da PROJ'dan al: the box, Al and `.prj dosyası…`, and what reading said.
fn paste<'a>(
    editor: &'a Editor,
    on: impl Fn(Event) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let typed = !editor.paste.text().trim().is_empty();
    let box_ = text_editor(&editor.paste)
        .placeholder("WKT (PROJCS[…], GEOGCS[…], PROJCRS[…] …) ya da +proj=… ile başlayan PROJ dizesi yapıştırın")
        .on_action({
            let on = on.clone();
            move |a| on(Event::Paste(a))
        })
        .height(Length::Fixed(typography::from_default(64.0)))
        // WKT has no spaces to break at: a long line breaks between letters.
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
        .size(typography::body())
        .padding([5, 8])
        .style(style::field::text_area);
    let mut c = column![
        section("WKT ya da PROJ'dan al"),
        box_,
        row![
            words::secondary("Al", typed.then(|| on(Event::Read))),
            words::secondary(".prj dosyası…", Some(on(Event::PickFile))),
        ]
        .spacing(8),
    ]
    .spacing(8);
    match &editor.read {
        Some(Ok(said)) => c = c.push(label::caption(said.clone())),
        Some(Err(why)) => c = c.push(label::caption(why.clone()).style(style::text::danger)),
        None => {}
    }
    c.into()
}

/// Deneme noktası: a point typed in this system, where it is in WGS 84 and
/// in the project's other system, how sure.
fn trial<'a>(
    editor: &'a Editor,
    on: impl Fn(Event) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    // Named by the kind chosen, not by a whole definition: the names stay as the fields are typed.
    let format = &editor.context.format;
    let names = match editor.form.kind {
        Kind::Geographic => ["Enlem", "Boylam"],
        _ => [format.east, format.north],
    };
    let input = |i: usize| {
        let on = on.clone();
        field(names[i], &editor.trial[i], 160.0, None, move |t| {
            on(Event::Trial(i, t))
        })
    };
    let reference = editor
        .context
        .reference
        .as_ref()
        .map_or("WGS 84'teki yeri".to_owned(), |(name, _)| {
            format!("WGS 84'teki ve {name} sistemindeki yeri")
        });
    let mut c = column![
        section("Deneme noktası"),
        label::caption(format!(
            "Bu sistemde bir nokta yazın: {reference}, projenin datum dönüşümleriyle gösterilir."
        )),
        row![input(0), input(1)].spacing(12),
    ]
    .spacing(8);
    match editor.trial_lines() {
        Ok(lines) => {
            for (name, values, accuracy) in lines {
                c = c.push(
                    row![
                        container(label::caption(name)).width(180),
                        column![label::mono(values), label::caption(accuracy)].spacing(2)
                    ]
                    .spacing(8),
                );
            }
        }
        Err(why) => c = c.push(label::caption(why)),
    }
    c.into()
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
    let c: Column<'a, Message> = column![
        section("Taban ve düzlem"),
        row![
            base,
            column![label::caption("Düzlem dönüşümü"), planes].spacing(4)
        ]
        .spacing(16),
    ]
    .spacing(10);
    let plane: Element<'a, Message> = match f.plane {
        PlaneKind::Similarity => {
            let value = |at, caption, key, width| text(editor, on.clone(), at, caption, key, width);
            c.push(
                row![
                    value(Field::East, "Sağa öteleme (m)", "east", 150.0),
                    value(Field::North, "Yukarı öteleme (m)", "north", 150.0),
                    value(Field::Rotation, "Dönüklük (°)", "rotation", 150.0),
                    value(Field::Scale, "Ölçek", "scale", 150.0),
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
    };
    column![plane, common_points(editor, on)].spacing(16).into()
}

/// Ortak noktalardan hesapla (docs/adr/0168 §1, §6): points known in this
/// system and in the base, the plane through them by least squares
/// (Vektör oturtma's solution), each one's residual and m0; Düzleme yaz
/// puts the plane in the fields above.
fn common_points<'a>(
    editor: &'a Editor,
    on: impl Fn(Event) -> Message + Clone + 'a,
) -> Element<'a, Message> {
    let p = &editor.points;
    let columns: &'static [Col] = if editor.context.format.east == "X" {
        &POINT_COLUMNS_XY
    } else {
        &POINT_COLUMNS_YX
    };
    let table = grid::view_with(
        CommonPoints,
        columns,
        p,
        |_, _| String::new(),
        0,
        |_| Vec::new(),
    );
    let kind = match editor.form.plane {
        PlaneKind::Similarity => "Benzerlik",
        PlaneKind::Affine => "Afin",
    };
    let mut c = column![
        section("Ortak noktalardan hesapla"),
        label::caption(format!(
            "Hem bu sistemde hem tabanda koordinatı bilinen noktalar: {kind} düzlemi en küçük karelerle, Vektör oturtma'nın çözümüyle bulunur. Satırları elektronik tablodan yapıştırabilirsiniz."
        )),
        table,
    ]
    .spacing(8);
    let used = p.fit.as_ref().and_then(|f| f.as_ref().ok()).map_or(0, |f| {
        f.rows.iter().filter(|r| p.rows[**r][USE] != "0").count()
    });
    match &p.fit {
        Some(Ok(fit)) => {
            let need = if editor.form.plane == PlaneKind::Similarity {
                2
            } else {
                3
            };
            c = c.push(label::caption(match fit.m0 {
                None => format!(
                    "{used} nokta tam geçer; m0 için en az bir fazla nokta gerekir (serbestlik 0)."
                ),
                Some(m0) => format!(
                    "m0 = ±{} ({used} nokta, serbestlik {}).",
                    mm_text(m0),
                    2 * used.saturating_sub(need)
                ),
            }));
        }
        Some(Err(why)) => c = c.push(label::caption(why.clone()).style(style::text::danger)),
        None => {}
    }
    if let Some(note) = definition_form::skipped_text(&p.skipped) {
        c = c.push(label::caption(note).style(style::text::danger));
    }
    let ready = matches!(p.fit, Some(Ok(_)));
    c.push(row![words::secondary(
        "Düzleme yaz",
        ready.then(|| on(Event::FitWrite))
    )])
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trial point compared with TUREF / TM30, no datum choices, as a CBS project writes points.
    fn context() -> Context {
        Context {
            reference: kentos_project::crs::system(5254)
                .and_then(|s| Some(("TUREF / TM30".to_owned(), s.transform_system()?))),
            choices: Vec::new(),
            format: ConvertFormat {
                east: "Y",
                north: "X",
                decimals: 3,
                notation: kentos_interaction::second::Notation::Dms,
            },
        }
    }

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
        let mut editor = Editor::new(Target::Own, None, context());
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
        let mut editor = Editor::new(Target::Second, None, context());
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
        let mut editor = Editor::new(Target::Own, Some(&d), context());
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
        let mut editor = Editor::new(Target::Second, Some(&based), context());
        assert!(editor.ready(), "kept, nothing said");
        assert!(
            matches!(editor.edit(Event::Done), Outcome::Done(Target::Second, got) if got == based)
        );
        editor.edit(Event::Text(Field::Name, "Başka".to_owned()));
        // Typed: the window's own base (none chosen) is asked for.
        assert_eq!(editor.problems.get("base"), Some(&definition_form::BASE));
    }

    fn paste(editor: &mut Editor, text: &str) {
        let paste = text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            text.to_owned(),
        )));
        assert!(matches!(editor.edit(Event::Paste(paste)), Outcome::Keep));
    }

    /// WKT ya da PROJ'dan al (docs/adr/0168 §5): a text pasted fills the
    /// fields; one the registry has offers Kayıttakini seç; a text that is
    /// not read says why and leaves the fields; a `.prj` file is read too.
    #[test]
    fn a_text_or_a_prj_file_fills_the_fields() {
        let mut editor = Editor::new(Target::Own, None, context());
        paste(
            &mut editor,
            "PROJCS[\"TUREF_TM36\",GEOGCS[\"GCS_TUREF\",DATUM[\"D_Turkish_National_Reference_Frame\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"False_Easting\",500000.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",36.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Latitude_Of_Origin\",0.0],UNIT[\"Meter\",1.0]]",
        );
        assert!(matches!(editor.edit(Event::Read), Outcome::Keep));
        assert_eq!(editor.form.name, "TUREF TM36");
        assert_eq!(editor.form.central_meridian, "36");
        assert_eq!(editor.same, Some(5256));
        assert_eq!(
            editor.note.as_deref(),
            Some("EPSG:5256 (TUREF / TM36) ile aynı; kayıttakini seçin.")
        );
        assert!(matches!(
            editor.edit(Event::Registry),
            Outcome::Registry(Target::Own, 5256)
        ));

        // Not read: why, the fields as they were.
        let mut editor = Editor::new(Target::Own, None, context());
        paste(&mut editor, "+proj=lcc +lat_1=36 +lat_2=42 +units=m");
        editor.edit(Event::Read);
        assert_eq!(
            editor.read,
            Some(Err(
                definition_form::READ_UNSUPPORTED.replace("{detail}", "+proj=lcc")
            ))
        );
        assert_eq!(editor.form, Form::default());

        // A .prj file: the box shows it, the fields are filled.
        let dir = crate::files_testing::scratch("ozel-crs-prj");
        let file = dir.join("santiye.prj");
        std::fs::write(
            &file,
            "+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 +ellps=bessel +towgs84=598.1,73.7,418.2,0.202,0.045,-2.455,6.7 +units=m +no_defs",
        )
        .expect("a file");
        let mut editor = Editor::new(Target::Second, None, context());
        editor.edit(Event::Picked(Some(file)));
        assert!(editor.paste.text().starts_with("+proj=tmerc"));
        assert_eq!(editor.form.datum, DatumPick::Custom);
        assert_eq!(editor.form.ellipsoid, Some(3), "Bessel 1841");
        assert!(editor.ready(), "{:?}", editor.problems);
    }

    /// The definition copied as WKT and PROJ (the core's texts), and a trial
    /// point shown in WGS 84 and in the project's other system.
    #[test]
    fn a_definition_is_copied_and_tried() {
        let mut editor = Editor::new(Target::Second, None, context());
        for (at, v) in [
            (Field::Name, "Kaydırılmış TM30"),
            (Field::CentralMeridian, "30"),
            (Field::FalseEasting, "400000"),
            (Field::FalseNorthing, "0"),
        ] {
            typed(&mut editor, at, v);
        }
        let d = editor.definition().expect("whole");
        let Outcome::Copy(wkt, "WKT") = editor.edit(Event::Copy(Text::Wkt)) else {
            panic!("WKT to the clipboard");
        };
        assert_eq!(Some(wkt), definition_form::wkt(&d));
        let Outcome::Copy(proj, "PROJ dizesi") = editor.edit(Event::Copy(Text::Proj)) else {
            panic!("PROJ to the clipboard");
        };
        assert!(proj.starts_with("+proj=tmerc +lat_0=0 +lon_0=30 +k=1 +x_0=400000"));
        // TM30 with its false easting 100 km less: a point is TM30's 100 km further east.
        editor.edit(Event::Trial(0, "400000".to_owned()));
        editor.edit(Event::Trial(1, "4400000".to_owned()));
        let lines = editor.trial_lines().expect("lines");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "WGS 84");
        assert!(lines[0].1.starts_with("Enlem 39°"), "{}", lines[0].1);
        assert_eq!(lines[1].0, "TUREF / TM30");
        assert_eq!(lines[1].1, "Y 500000.000   X 4400000.000");
        assert_eq!(lines[1].2, "kesin, yalnız projeksiyon");
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

    /// The shared case “benzerlik: beş nokta, gürültülü”'s rows.
    fn five_points() -> Vec<[String; 5]> {
        let file: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/crs/v1/definition-fit.json"
        ))
        .expect("the cases read");
        let case = file["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|c| c["name"] == "benzerlik: beş nokta, gürültülü")
            .expect("the case")
            .clone();
        case["typed"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| std::array::from_fn(|i| r[i].as_str().expect("a text").to_owned()))
            .collect()
    }

    /// Ortak noktalardan hesapla (docs/adr/0168 §1, §6): points typed and
    /// pasted from a spreadsheet give the plane live, with residuals and m0;
    /// a point left out reads faded, the worst residual is marked; Düzleme
    /// yaz puts the plane in the fields, which build the definition.
    #[test]
    fn common_points_give_the_local_plane() {
        let mut editor = Editor::new(Target::Own, None, context());
        typed(&mut editor, Field::Name, "Belediye yerel");
        editor.edit(Event::Kind(Kind::Local));
        editor.edit(Event::Base(5254));
        let rows = five_points();
        // The first row typed cell by cell, the others pasted as a spreadsheet's lines.
        for (col, v) in rows[0][..4].iter().enumerate() {
            editor.edit(Event::Point(0, col + 1, v.clone()));
        }
        let lines: Vec<String> = rows[1..].iter().map(|r| r[..4].join("\t")).collect();
        editor.edit(Event::PointPasted(1, 1, Some(lines.join("\n"))));
        assert_eq!(editor.points.rows.len(), 5, "a row added for the fifth");
        let Some(Ok(fit)) = &editor.points.fit else {
            panic!("a plane: {:?}", editor.points.fit);
        };
        let m0 = fit.m0.expect("m0");
        assert!((m0 - 0.003_004_520_808_648_213_6).abs() < 1e-9, "{m0}");
        assert!(
            !editor.points.rows[0][RESIDUAL + 2].is_empty(),
            "residuals in mm"
        );
        let worst = editor.points.worst().expect("the worst residual");
        assert_eq!(editor.points.mark(worst), Some(Mark::Worst));
        // A point left out: faded, and the solution without it.
        editor.edit(Event::Point(2, USE, "0".to_owned()));
        assert_eq!(editor.points.mark(2), Some(Mark::Off));
        let Some(Ok(fit)) = &editor.points.fit else {
            panic!("a plane");
        };
        assert!((fit.m0.expect("m0") - 0.002_727_334_231_884_259_7).abs() < 1e-9);
        assert!(matches!(editor.edit(Event::FitWrite), Outcome::Keep));
        assert_eq!(editor.form.plane, PlaneKind::Similarity);
        assert!(
            editor.form.scale.starts_with("1.0000072"),
            "{}",
            editor.form.scale
        );
        assert!(editor.ready(), "{:?}", editor.problems);
        let d = editor.definition().expect("whole");
        let CrsSystem::Local(l) = &d.system else {
            panic!("a local system");
        };
        assert_eq!(l.base.srid, Some(5254));
        // An affine needs three: the plane changes kind and is solved again.
        editor.edit(Event::Plane(PlaneKind::Affine));
        assert!(matches!(editor.points.fit, Some(Ok(_))));
        editor.edit(Event::FitWrite);
        assert!(editor.ready(), "{:?}", editor.problems);
    }

    /// Through Proje ayarları: a WKT read offers the registry's system, and
    /// Kayıttakini seç puts it in the draft in the window's place; a copy
    /// says so in the log.
    #[test]
    fn kayittakini_sec_and_a_copy_go_through_proje_ayarlari() {
        use crate::project::{SettingsEvent, settings_message};
        let mut app = crate::files_testing::app_with_drawing();
        let _ = app.update(Message::Run("crs.set"));
        let send = |app: &mut crate::app::App, e: Event| {
            let _ = app.update(settings_message(SettingsEvent::Custom(e)));
        };
        let _ = app.update(settings_message(SettingsEvent::NewDefinition(Target::Own)));
        let paste = text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            "+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 +ellps=GRS80 +units=m +no_defs"
                .to_owned(),
        )));
        send(&mut app, Event::Paste(paste));
        send(&mut app, Event::Read);
        send(&mut app, Event::Copy(Text::Proj));
        assert!(app.log.said(
            kentos_interaction::Level::Success,
            "PROJ dizesi panoya kopyalandı."
        ));
        // GRS80 without TUREF's name is a datum of the text's own: no registry system, a shared grid said.
        let editor = open_editor(&app).expect("open");
        assert_eq!(editor.same, None);
        assert_eq!(
            editor.grid.as_deref(),
            Some("Izgarası EPSG:5255 (TUREF / TM33) ile aynı; datumu metnin kendi datumu.")
        );
        send(
            &mut app,
            Event::Datum(DatumPick::Registry(RegistryDatum::Turef)),
        );
        assert_eq!(open_editor(&app).expect("open").same, Some(5255));
        send(&mut app, Event::Registry);
        assert!(open_editor(&app).is_none());
        let Some(super::super::Window::Settings(s)) = &app.project else {
            panic!("Proje ayarları stays");
        };
        assert_eq!(s.draft_srid(), (5255, false));
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
        let paste = |t: &str| {
            Event::Paste(text_editor::Action::Edit(text_editor::Edit::Paste(
                std::sync::Arc::new(t.to_owned()),
            )))
        };
        let tm36 = "PROJCS[\"TUREF_TM36\",GEOGCS[\"GCS_TUREF\",DATUM[\"D_Turkish_National_Reference_Frame\",SPHEROID[\"GRS_1980\",6378137.0,298.257222101]],PRIMEM[\"Greenwich\",0.0],UNIT[\"Degree\",0.0174532925199433]],PROJECTION[\"Transverse_Mercator\"],PARAMETER[\"False_Easting\",500000.0],PARAMETER[\"False_Northing\",0.0],PARAMETER[\"Central_Meridian\",36.0],PARAMETER[\"Scale_Factor\",1.0],PARAMETER[\"Latitude_Of_Origin\",0.0],UNIT[\"Meter\",1.0]]";
        // Each picture: its name, what is done in the window, whether its body is scrolled to the end.
        let shots: Vec<(&str, Vec<Event>, bool)> = vec![
            ("yeni", vec![], false),
            (
                "hata",
                vec![
                    Event::Text(Field::Name, "Şantiye".to_owned()),
                    Event::Text(Field::CentralMeridian, "300".to_owned()),
                    Event::Text(Field::ScaleFactor, "0".to_owned()),
                ],
                false,
            ),
            (
                "tm",
                vec![
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
                true,
            ),
            (
                "yerel",
                vec![
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
                false,
            ),
            ("metin", vec![paste(tm36), Event::Read], false),
            (
                "metin-hata",
                vec![
                    paste("+proj=lcc +lat_1=36 +lat_2=42 +lon_0=33 +units=m"),
                    Event::Read,
                ],
                false,
            ),
            (
                "ortak",
                {
                    let mut events = vec![
                        Event::Text(Field::Name, "Belediye yerel".to_owned()),
                        Event::Kind(Kind::Local),
                        Event::Base(5254),
                    ];
                    let rows = five_points();
                    let lines: Vec<String> = rows.iter().map(|r| r[..4].join("\t")).collect();
                    events.push(Event::PointPasted(0, 1, Some(lines.join("\n"))));
                    events.push(Event::Point(2, USE, "0".to_owned()));
                    events.push(Event::FitWrite);
                    events
                },
                true,
            ),
            (
                "deneme",
                vec![
                    Event::Text(Field::Name, "Kaydırılmış TM36".to_owned()),
                    Event::Text(Field::CentralMeridian, "36".to_owned()),
                    Event::Text(Field::FalseEasting, "400000".to_owned()),
                    Event::Text(Field::FalseNorthing, "0".to_owned()),
                    Event::Trial(0, "412345.678".to_owned()),
                    Event::Trial(1, "4421234.567".to_owned()),
                ],
                true,
            ),
        ];
        for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
            let picture = |app: &mut App, name: &str, click: Option<&str>, end: bool| {
                app.follow.flash = None;
                let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(app, App::view, &mut update);
                if end
                    && let Some(r) = crate::files_testing::find_texts(
                        &mut snapshot,
                        app,
                        "Özel koordinat sistemi",
                    )
                    .last()
                    .copied()
                {
                    let body = iced::Point::new(r.x + 200.0, r.y + 200.0);
                    snapshot.input(app, App::view, &mut update, Input::Scroll(body, -40.0));
                }
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
            for (name, events, end) in &shots {
                let mut app = fresh();
                let _ = app.update(settings_message(SettingsEvent::NewDefinition(Target::Own)));
                for e in events.iter() {
                    let _ = app.update(settings_message(SettingsEvent::Custom(e.clone())));
                }
                picture(&mut app, name, None, *end);
            }
            // Proje ayarları with the definition chosen, and the second system's list open.
            let mut app = fresh();
            let _ = app.update(settings_message(SettingsEvent::NewDefinition(Target::Own)));
            for e in &shots[2].1 {
                let _ = app.update(settings_message(SettingsEvent::Custom(e.clone())));
            }
            let _ = app.update(settings_message(SettingsEvent::Custom(Event::Done)));
            picture(&mut app, "ayarlar", None, false);
            let _ = app.update(settings_message(SettingsEvent::NewDefinition(
                Target::Second,
            )));
            for e in &shots[3].1 {
                let _ = app.update(settings_message(SettingsEvent::Custom(e.clone())));
            }
            let _ = app.update(settings_message(SettingsEvent::Custom(Event::Done)));
            picture(
                &mut app,
                "ikinci",
                Some("Belediye yerel (özel sistem)"),
                false,
            );
        }
    }
}
