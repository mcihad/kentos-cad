//! The template gallery, “Pafta şablonları” (docs/sheet/design.md §11, §11a,
//! §12, §13; the web's `ui/sheet/TemplateGallery.ts`, `galleryDetails.ts`,
//! `galleryPlan.ts`): the sections on the left, Sistem · Benim · Kurumum ·
//! Benimle paylaşılanlar, each saying why when it has nothing to give now,
//! and the library's line (synced, offline …); in the middle the cards, each
//! the template's own plan drawn small as Kullan would make it, on this
//! project's drawing (the host's [`MapPainter`](crate::MapPainter)), with
//! its badges, searched and filtered by paper and kind; on the right the
//! chosen template: its picture, badges, how it stands with the project, its
//! actions with their reasons, what it needs that the project lacks, and
//! what it is. The order is the core's for the project (`rank_templates`:
//! its type's templates first, then its mode's, then the common ones);
//! “Bütün kiplerin şablonları” shows the others too, badged with their mode.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use iced::widget::{button, canvas, column, container, responsive, row, scrollable, space, stack};
use iced::{Center, Element, Fill, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use kentos_contracts::{ProjectType, Workspace};
use kentos_sheet::cloud::{DeviceCloudState, TemplateRole};
use kentos_sheet::display::{self, DisplayList, Prim, RenderMode};
use kentos_sheet::model::{Orientation, Paper, SheetBook};
use kentos_sheet::ops::{AddAssets, AddMaster, AddSheet, Op};
use kentos_sheet::preflight::Finding;
use kentos_sheet::profile::{RankedTemplate, TemplateFit, rank_templates};
use kentos_sheet::template::{
    InstanceIds, InstanceOptions, PaperChoice, Template, instantiate, system_templates,
};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Choice, Confirm, Dialog, InputField, Select, Switch, Tip, overlay, tip};

use crate::designer::{Designer, Effect, Say};
use crate::library::{self, Badge, CardSource, LibraryRequest, TemplateAction};
use crate::message::Message;
use crate::paint::{self, Maps, Options, Xf};
use crate::painter::Painter;
use crate::pictures::{self, Raster};
use crate::publish_template::PublishDialog;
use crate::share_template::ShareDialog;
use crate::store::StoredTemplate;

/// The gallery's sections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Source {
    #[default]
    System,
    Mine,
    Organisation,
    Shared,
}

impl Source {
    pub const ALL: [Source; 4] = [
        Source::System,
        Source::Mine,
        Source::Organisation,
        Source::Shared,
    ];

    /// The section a card is listed in.
    pub fn of(s: CardSource) -> Source {
        match s {
            CardSource::System => Source::System,
            CardSource::Organisation => Source::Organisation,
            CardSource::Shared => Source::Shared,
            CardSource::Device | CardSource::Cloud => Source::Mine,
        }
    }

    fn glyph(self) -> Icon {
        match self {
            Source::System => Icon::Lock,
            Source::Mine => crate::icons::STYLES,
            Source::Organisation => Icon::Layers,
            Source::Shared => crate::icons::SHARE,
        }
    }
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Source::System => "Sistem",
            Source::Mine => "Benim",
            Source::Organisation => "Kurumum",
            Source::Shared => "Benimle paylaşılanlar",
        })
    }
}

/// Whether a section has its cards, and why not (the web's `ProviderState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ready,
    /// It cannot list now: the reason, said in the list.
    Unavailable(&'static str),
    /// It is announced and comes later.
    #[allow(dead_code)]
    Soon(&'static str),
}

#[derive(Clone, Debug, PartialEq)]
pub enum GalleryMessage {
    Open,
    Close,
    Source(Source),
    Query(String),
    Paper(Option<Paper>),
    Kind(Option<String>),
    AllModes(bool),
    Pick(String),
    /// The paper Kullan lays the template out on; none: its recommended one.
    UsePaper(Option<PaperChoice>),
    Use,
    Act(TemplateAction),
    /// The question over the gallery answered: go on (true) or not.
    Answer(bool),
}

/// A question the gallery asks before it acts.
#[derive(Clone, Debug, PartialEq)]
pub enum Asking {
    /// Sil: a template of this device, or one of the cloud library everywhere (an organisation's:
    /// from every member's list).
    Delete {
        id: String,
        name: String,
        cloud: bool,
        organisation: Option<String>,
    },
    /// Kullan, though the project lacks what the template needs.
    Needs {
        id: String,
        name: String,
        needs: Vec<String>,
    },
}

/// A card's picture: the template laid out as Kullan would make it, its pictures decoded.
#[derive(Debug)]
pub(crate) struct Thumb {
    /// What it was made from: the template's id, revision and date, and where its maps look.
    key: String,
    pub list: Option<DisplayList>,
    /// The list cut at its maps and pictures (`paint::plan`): a canvas each, in order, so a
    /// picture keeps its place (a renderer draws a canvas's pictures over its shapes).
    plan: paint::Plan,
    /// A cache a canvas: the layers and the maps and pictures between them, in order.
    caches: Vec<canvas::Cache>,
    /// The painter's revision the cache holds.
    painted: Cell<u64>,
    pictures: BTreeMap<String, Option<Rc<Raster>>>,
    /// What a sheet made from it would lack in this project (the preflight's `needs_…`).
    pub needs: Vec<Finding>,
}

#[derive(Debug, Default)]
pub struct Gallery {
    pub source: Source,
    pub query: String,
    pub paper: Option<Paper>,
    pub kind: Option<String>,
    pub all_modes: bool,
    pub selected: Option<String>,
    pub use_paper: Option<PaperChoice>,
    /// What the gallery says at its foot (why an action cannot run).
    pub note: Option<&'static str>,
    pub asking: Option<Asking>,
    /// Şablonu paylaş, over the gallery.
    pub share: Option<ShareDialog>,
    /// Kuruma yayımla, over the gallery.
    pub publish: Option<PublishDialog>,
    pub(crate) thumbs: BTreeMap<String, Thumb>,
}

/// A template of the gallery with what the card says of it.
pub(crate) struct Card<'a> {
    pub id: &'a str,
    pub template: &'a Template,
    pub source: CardSource,
    pub badges: Vec<Badge>,
    /// Another mode's badge (“CBS şablonu”), from the core.
    pub mode: Option<String>,
    pub fit: TemplateFit,
    /// Shown without “Bütün kiplerin şablonları”.
    pub matches: bool,
    /// Who shared it, for one shared with the account.
    pub shared_by: Option<&'a str>,
    /// The account's role in one of its cloud library.
    pub role: Option<TemplateRole>,
    /// A copy a conflict kept.
    pub conflict: bool,
    /// The organisation whose library it is in.
    pub org: Option<&'a str>,
}

/// Text in the warning tone (a badge that waits, why an action cannot run, a declination not known).
pub(crate) fn warning(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).warning),
    }
}

/// A card as the host and the tests read it: what the gallery lists now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardView {
    pub id: String,
    pub name: String,
    pub source: CardSource,
    pub badges: Vec<Badge>,
}

/// How a card stands with the project (the web's `FIT_LABEL`).
fn fit_label(f: TemplateFit) -> &'static str {
    match f {
        TemplateFit::ProjectType => "Projenin türüne uygun",
        TemplateFit::Workspace => "Çalışma moduna uygun",
        TemplateFit::Common => "Ortak: her çalışma modunda",
        TemplateFit::Other => "Başka bir çalışma modu için",
    }
}

pub(crate) fn paper_name(p: Paper) -> &'static str {
    match p {
        Paper::A0 => "A0",
        Paper::A1 => "A1",
        Paper::A2 => "A2",
        Paper::A3 => "A3",
        Paper::A4 => "A4",
        Paper::A5 => "A5",
        Paper::B0 => "B0",
        Paper::B1 => "B1",
        Paper::B2 => "B2",
        Paper::B3 => "B3",
        Paper::B4 => "B4",
        Paper::Custom => "Özel",
    }
}

/// “A3 yatay”: a paper choice as the gallery writes it.
pub(crate) fn paper_text(c: &PaperChoice) -> String {
    format!(
        "{} {}",
        paper_name(c.paper),
        if c.orientation == Orientation::Landscape {
            "yatay"
        } else {
            "dikey"
        }
    )
}

/// A paper's size in millimetres, in its orientation.
fn paper_mm(c: &PaperChoice) -> Option<(f64, f64)> {
    let s = kentos_sheet::paper::paper_size(c.paper, c.orientation)?;
    Some((f64::from(s.width) / 1000.0, f64::from(s.height) / 1000.0))
}

/// A category as the gallery writes it: “kadastro” → “Kadastro”, “imar” → “İmar”.
pub(crate) fn category_label(c: &str) -> String {
    let mut chars = c.chars();
    match chars.next() {
        Some('i') => format!("İ{}", chars.as_str()),
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn mode_label(w: Option<Workspace>) -> &'static str {
    match w {
        Some(Workspace::Cad) => "CAD",
        Some(Workspace::Gis) => "CBS",
        _ => "Hibrit",
    }
}

fn type_label(t: ProjectType) -> &'static str {
    match t {
        ProjectType::Cad => "Genel CAD",
        ProjectType::Gis => "CBS",
        ProjectType::LandReadjustment => "18 uygulaması",
        ProjectType::ZoningPlan => "İmar planı",
        ProjectType::Subdivision => "İfraz / tevhit",
        ProjectType::Road => "Yol",
        ProjectType::Architecture => "Mimari",
    }
}

/// A template's layouts (design §3.2a): each with the papers it is for (the web's `layoutsOf`).
fn layouts_of(t: &Template) -> String {
    let vs = &t.sheet.variants;
    if vs.is_empty() {
        return "Temel düzen: her kâğıtta kısıtlarıyla yerleşir.".to_owned();
    }
    vs.iter()
        .map(|v| {
            format!(
                "{} · {}",
                v.name,
                crate::text::lower(&condition_text(&v.when))
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A layout's condition (the web's `conditionText`): “Yatay; genişlik en çok 420 mm”.
fn condition_text(w: &kentos_sheet::model::VariantCondition) -> String {
    let mm = |um: i32| kentos_geometry_core::display::fixed(f64::from(um) / 1000.0, 0);
    let mut parts = vec![
        if w.orientation == Orientation::Landscape {
            "Yatay"
        } else {
            "Dikey"
        }
        .to_owned(),
    ];
    let mut range = |label: &str, a: Option<i32>, b: Option<i32>| match (a, b) {
        (Some(a), Some(b)) if a == b => parts.push(format!("{label} {} mm", mm(a))),
        (Some(a), Some(b)) => parts.push(format!("{label} {}–{} mm", mm(a), mm(b))),
        (Some(a), None) => parts.push(format!("{label} en az {} mm", mm(a))),
        (None, Some(b)) => parts.push(format!("{label} en çok {} mm", mm(b))),
        (None, None) => {}
    };
    range("genişlik", w.min_width, w.max_width);
    range("yükseklik", w.min_height, w.max_height);
    parts.join("; ")
}

/// Where a template is kept, and the account's place in it (the web's `cloudText`).
fn where_text(c: &Card<'_>) -> String {
    match c.source {
        CardSource::System => "Uygulamayla gelir; web ve masaüstünde aynı.".to_owned(),
        CardSource::Device => {
            "Yalnız bu cihazda. Buluta eşitleyince web ve masaüstünde aynı olur ve paylaşılabilir."
                .to_owned()
        }
        CardSource::Shared => format!(
            "{} · {}. Son indirilen sürümü bu cihazda; bağlantı yokken de kullanılır.",
            c.shared_by.map_or_else(
                || "Sizinle paylaşıldı".to_owned(),
                |by| format!("{by} paylaştı")
            ),
            if c.role == Some(TemplateRole::Editor) {
                "düzenleyebilirsiniz"
            } else {
                "görüntüleyebilirsiniz"
            }
        ),
        CardSource::Organisation => format!(
            "“{}” kurumunun şablonu, revizyon {}{}. Kurumun etkin üyeleri görür ve kullanır; yayımlayan ve kurum yöneticileri düzenler. Son sürümü bu cihazda; bağlantı yokken de kullanılır.",
            c.org.unwrap_or_default(),
            c.template.meta.revision,
            match (c.role, c.shared_by) {
                (Some(TemplateRole::Owner), _) => " · siz yayımladınız".to_owned(),
                (_, Some(by)) => format!(" · {by} yayımladı"),
                _ => String::new(),
            }
        ),
        CardSource::Cloud => format!(
            "Bulut hesabınızda (kişisel alan), revizyon {}{}. Son sürümü bu cihazda; bağlantı yokken de kullanılır.",
            c.template.meta.revision,
            if c.badges.contains(&Badge::Shared) {
                " · paylaşıldı"
            } else {
                ""
            }
        ),
    }
}

/// Where a paper stands in the series (A before B, then by number).
fn series(p: Paper) -> u32 {
    match p {
        Paper::A0 => 0,
        Paper::A1 => 1,
        Paper::A2 => 2,
        Paper::A3 => 3,
        Paper::A4 => 4,
        Paper::A5 => 5,
        Paper::B0 => 100,
        Paper::B1 => 101,
        Paper::B2 => 102,
        Paper::B3 => 103,
        Paper::B4 => 104,
        Paper::Custom => 1000,
    }
}

/// The ids of a preview's sheet, fixed (its maps' pictures keep their place from one opening to the next).
fn preview_ids(t: &Template) -> InstanceIds {
    InstanceIds {
        sheet: "onizleme".into(),
        master: t.master.as_ref().map(|_| "onizleme-ana".into()),
        items: (0..t.sheet.items.len()).map(|i| format!("o{i}")).collect(),
        master_items: t
            .master
            .iter()
            .flat_map(|m| (0..m.items.len()).map(|i| format!("oa{i}")))
            .collect(),
    }
}

impl Designer {
    /// A sheet made from a template as Kullan would make it (the drawing
    /// area's centre, the plot scale), in a book of its own: the card's
    /// picture and what its preflight finds that the project lacks.
    fn thumb_of(&self, t: &Template, key: String) -> Thumb {
        let options = InstanceOptions {
            center: self.ctx.center,
            scale: self.ctx.capabilities.plot_scale,
            ..InstanceOptions::default()
        };
        let mut thumb = Thumb {
            key,
            list: None,
            plan: paint::Plan::default(),
            caches: Vec::new(),
            painted: Cell::new(u64::MAX),
            pictures: BTreeMap::new(),
            needs: Vec::new(),
        };
        let Ok(inst) = instantiate(t, &preview_ids(t), &options) else {
            return thumb;
        };
        thumb.pictures = inst
            .asset_bytes
            .iter()
            .map(|a| {
                let raster = kentos_sheet::template::base64_decode(&a.data)
                    .and_then(|b| pictures::decode(&b))
                    .map(Rc::new);
                (a.meta.sha256.clone(), raster)
            })
            .collect();
        let book = SheetBook {
            assets: inst.assets,
            masters: inst.master.into_iter().collect(),
            sheets: vec![inst.sheet],
            ..SheetBook::default()
        };
        let inputs = self.inputs(RenderMode::Design);
        thumb.list = display::build(&book, "onizleme", &inputs)
            .ok()
            .map(|(l, _)| l);
        if let Some(list) = &thumb.list {
            thumb.plan = paint::plan(list);
            let canvases = thumb.plan.layers.len() + thumb.plan.maps.len();
            thumb.caches = (0..canvases).map(|_| canvas::Cache::new()).collect();
        }
        for f in kentos_sheet::preflight::preflight(&book, "onizleme", &inputs).unwrap_or_default()
        {
            if f.code.starts_with("needs_") && !thumb.needs.iter().any(|n| n.code == f.code) {
                thumb.needs.push(f);
            }
        }
        thumb
    }

    /// Every card the gallery has, in the core's order for the project: the
    /// system's, and the templates kept here that the viewer sees.
    fn every_card(&self) -> Vec<Card<'_>> {
        let busy = &self.library.busy;
        let mut cards: Vec<Option<Card<'_>>> = system_templates()
            .iter()
            .map(|t| Card {
                id: t.meta.id.as_str(),
                template: t,
                source: CardSource::System,
                badges: vec![Badge::System],
                mode: None,
                fit: TemplateFit::Common,
                matches: true,
                shared_by: None,
                role: None,
                conflict: false,
                org: None,
            })
            .map(Some)
            .collect();
        for r in library::visible(&self.user_templates, self.library.viewer.as_deref()) {
            cards.push(Some(Card {
                id: r.id.as_str(),
                template: &r.template,
                source: CardSource::of(r),
                badges: library::badges_of(r, busy),
                mode: None,
                fit: TemplateFit::Common,
                matches: true,
                shared_by: r.cloud.as_ref().and_then(|c| c.owner_name.as_deref()),
                role: r.cloud.as_ref().map(|c| c.role),
                conflict: r.cloud.as_ref().is_some_and(|c| c.conflict_of.is_some()),
                org: r
                    .cloud
                    .as_ref()
                    .and_then(|c| c.organization.as_ref())
                    .map(|o| o.name.as_str()),
            }));
        }
        let metas: Vec<_> = cards
            .iter()
            .flatten()
            .map(|c| {
                let mut m = c.template.meta.clone();
                m.id = c.id.to_owned();
                m
            })
            .collect();
        let ranked: Vec<RankedTemplate> =
            rank_templates(&metas, self.ctx.workspace, self.ctx.project_type);
        let mut out = Vec::with_capacity(cards.len());
        for r in ranked {
            let slot = cards
                .iter_mut()
                .find(|c| c.as_ref().is_some_and(|c| c.id == r.id));
            if let Some(mut c) = slot.and_then(Option::take) {
                c.fit = r.fit;
                c.matches = r.matches;
                c.mode = r.badge;
                out.push(c);
            }
        }
        out
    }

    /// The cards of the gallery's section for its query, in the core's order.
    pub(crate) fn cards(&self, g: &Gallery) -> Vec<Card<'_>> {
        let words: Vec<String> = crate::text::fold(&g.query)
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        self.every_card()
            .into_iter()
            .filter(|c| Source::of(c.source) == g.source)
            .filter(|c| g.all_modes || c.matches)
            .filter(|c| {
                g.paper
                    .is_none_or(|p| c.template.meta.papers.iter().any(|x| x.paper == p))
            })
            .filter(|c| {
                g.kind
                    .as_ref()
                    .is_none_or(|k| &c.template.meta.category == k)
            })
            .filter(|c| {
                if words.is_empty() {
                    return true;
                }
                let m = &c.template.meta;
                let hay = crate::text::fold(&format!(
                    "{} {} {} {}",
                    m.name,
                    m.description,
                    m.category,
                    m.tags.join(" ")
                ));
                words.iter().all(|w| hay.contains(w.as_str()))
            })
            .collect()
    }

    /// The cards the open gallery lists now, in its order (none while it is closed).
    pub fn gallery_cards(&self) -> Vec<CardView> {
        let Some(g) = &self.gallery else {
            return Vec::new();
        };
        self.cards(g)
            .into_iter()
            .map(|c| CardView {
                id: c.id.to_owned(),
                name: c.template.meta.name.clone(),
                source: c.source,
                badges: c.badges,
            })
            .collect()
    }

    /// What the host said last of the account's cloud library.
    pub fn library(&self) -> &crate::library::Library {
        &self.library
    }

    /// Whether the template gallery is open.
    pub fn gallery_open(&self) -> bool {
        self.gallery.is_some()
    }

    /// What the open gallery says at its foot now (why an action cannot run).
    pub fn gallery_note(&self) -> Option<&'static str> {
        self.gallery.as_ref().and_then(|g| g.note)
    }

    /// The question the open gallery asks now (Sil, Kullan with what the project lacks).
    pub fn gallery_asking(&self) -> Option<&Asking> {
        self.gallery.as_ref().and_then(|g| g.asking.as_ref())
    }

    /// Şablonu paylaş, while it is open.
    pub fn sharing(&self) -> Option<&ShareDialog> {
        self.gallery.as_ref().and_then(|g| g.share.as_ref())
    }

    /// The templates kept on this device, as the sheet mode read them last.
    pub fn device_templates(&self) -> &[StoredTemplate] {
        &self.user_templates
    }

    /// Whether a section has its cards now, and why not.
    fn section_state(&self, s: Source, every: &[Card<'_>]) -> State {
        let org_cards = || every.iter().any(|c| c.source == CardSource::Organisation);
        match s {
            Source::Organisation if self.library.viewer.is_none() && !org_cards() => {
                State::Unavailable(library::texts::ORGANISATION_SIGN_IN)
            }
            // Signed in, the library read, in no organisation (its copies would still be listed).
            Source::Organisation
                if self.library.account.is_some()
                    && self.library.organizations.is_empty()
                    && matches!(self.library.status, library::SyncStatus::Synced(_))
                    && !org_cards() =>
            {
                State::Unavailable(library::texts::NO_ORGANISATION)
            }
            Source::Shared
                if self.library.viewer.is_none()
                    && !every.iter().any(|c| c.source == CardSource::Shared) =>
            {
                State::Unavailable(library::texts::SHARED_SIGN_IN)
            }
            _ => State::Ready,
        }
    }

    /// The card the gallery shows on the right.
    pub(crate) fn picked_card(&self) -> Option<Card<'_>> {
        let g = self.gallery.as_ref()?;
        let id = g.selected.as_deref()?;
        self.every_card().into_iter().find(|c| c.id == id)
    }

    /// The template the gallery shows on the right.
    pub(crate) fn picked(&self) -> Option<&Template> {
        self.picked_card().map(|c| c.template)
    }

    /// The key of a card's picture: the template's revision, where Kullan would place its maps
    /// and what the project has (what the template needs that it lacks).
    fn thumb_key(&self, t: &Template) -> String {
        let m = &t.meta;
        let caps = &self.ctx.capabilities;
        format!(
            "{}|{}|{}|{:?}|{:?}|{}|{}",
            m.id,
            m.revision,
            m.updated,
            self.ctx.center.map(|c| (c.x.to_bits(), c.y.to_bits())),
            caps.plot_scale,
            caps.georeferenced,
            caps.attribute_layers
        )
    }

    /// The pictures of the cards shown now (and the chosen one's), worked out once each.
    pub(crate) fn ensure_thumbs(&mut self) {
        let Some(g) = &self.gallery else {
            return;
        };
        let picked = self.picked_card();
        let wanted: Vec<(String, String, Template)> = self
            .cards(g)
            .iter()
            .chain(picked.iter())
            .map(|c| {
                (
                    c.id.to_owned(),
                    self.thumb_key(c.template),
                    c.template.clone(),
                )
            })
            .filter(|(id, key, _)| g.thumbs.get(id).is_none_or(|t| &t.key != key))
            .collect();
        let made: Vec<(String, Thumb)> = wanted
            .into_iter()
            .map(|(id, key, t)| (id, self.thumb_of(&t, key)))
            .collect();
        if let Some(g) = &mut self.gallery {
            for (id, thumb) in made {
                g.thumbs.insert(id, thumb);
            }
        }
    }

    /// The templates kept here, read again (a sync changed them; one was saved or deleted here).
    pub fn library_changed(&mut self) -> Vec<Effect> {
        if let Some(store) = &self.store {
            let (records, refused) = store.records();
            self.user_templates = records;
            self.refused = refused;
        }
        let known =
            |id: &str| id.starts_with("sys:") || self.user_templates.iter().any(|r| r.id == id);
        let stale = self
            .gallery
            .as_ref()
            .and_then(|g| g.selected.as_deref())
            .is_some_and(|id| !known(id));
        if stale && let Some(g) = &mut self.gallery {
            g.selected = None;
        }
        self.ensure_thumbs();
        self.follow_share(&[])
    }

    /// The host's word on the cloud library, and its answers to the share window.
    pub fn library_event(&mut self, e: library::LibraryEvent) -> Vec<Effect> {
        use library::LibraryEvent as E;
        match e {
            E::State(l) => {
                let before = self.library.why();
                self.library = l;
                if self.gallery.is_some() && before != self.library.why() {
                    return self.follow_share(&[]);
                }
                Vec::new()
            }
            E::Changed => self.library_changed(),
            E::Created(pairs) => {
                if let Some(g) = &mut self.gallery
                    && let Some((_, new)) = pairs
                        .iter()
                        .find(|(old, _)| g.selected.as_deref() == Some(old.as_str()))
                {
                    g.selected = Some(new.clone());
                }
                // Read again first: the window follows its template to the id the cloud gave it.
                let mut out = self.library_changed();
                out.extend(self.follow_share(&pairs));
                out
            }
            E::NotUploaded(id) => {
                self.share_not_uploaded(&id);
                Vec::new()
            }
            E::Published { id, result } => self.publish_answer(&id, result),
            other => self.share_answer(other),
        }
    }

    pub(crate) fn gallery_update(&mut self, m: GalleryMessage) -> Vec<Effect> {
        let mut effects = Vec::new();
        match m {
            GalleryMessage::Open => {
                let preferred = self.profile.default_template.clone();
                self.gallery = Some(Gallery {
                    selected: Some(preferred),
                    ..Gallery::default()
                });
                // What changed on the device since (a sync while the gallery was closed).
                if let Some(store) = &self.store {
                    let (records, refused) = store.records();
                    self.user_templates = records;
                    self.refused = refused;
                }
            }
            GalleryMessage::Close => self.gallery = None,
            GalleryMessage::Source(s) => {
                if let Some(g) = &mut self.gallery {
                    g.source = s;
                    g.selected = None;
                    g.note = None;
                    g.paper = None;
                    g.kind = None;
                }
            }
            GalleryMessage::Query(q) => {
                if let Some(g) = &mut self.gallery {
                    g.query = q;
                }
            }
            GalleryMessage::Paper(p) => {
                if let Some(g) = &mut self.gallery {
                    g.paper = p;
                }
            }
            GalleryMessage::Kind(k) => {
                if let Some(g) = &mut self.gallery {
                    g.kind = k;
                }
            }
            GalleryMessage::AllModes(on) => {
                if let Some(g) = &mut self.gallery {
                    g.all_modes = on;
                }
            }
            GalleryMessage::Pick(id) => {
                if let Some(g) = &mut self.gallery {
                    g.selected = Some(id);
                    g.use_paper = None;
                    g.note = None;
                }
            }
            GalleryMessage::UsePaper(p) => {
                if let Some(g) = &mut self.gallery {
                    g.use_paper = p;
                }
            }
            GalleryMessage::Use => effects = self.act(TemplateAction::Use),
            GalleryMessage::Act(a) => effects = self.act(a),
            GalleryMessage::Answer(go) => {
                let asked = self.gallery.as_mut().and_then(|g| g.asking.take());
                if go {
                    effects = match asked {
                        Some(Asking::Delete { id, .. }) => self.delete_template(&id),
                        Some(Asking::Needs { id, .. }) => self.use_card(&id, false),
                        None => Vec::new(),
                    };
                }
            }
        }
        self.ensure_thumbs();
        effects
    }

    /// Does what the gallery offers for the chosen card (the web's `templateAction`).
    fn act(&mut self, a: TemplateAction) -> Vec<Effect> {
        let Some((id, source, role, name)) = self.picked_card().map(|c| {
            (
                c.id.to_owned(),
                c.source,
                c.role,
                c.template.meta.name.clone(),
            )
        }) else {
            return Vec::new();
        };
        let view = library::action(source, role, self.library.can(), a);
        // Paylaş… opens even when it cannot share yet: its window says why (design §13).
        if a == TemplateAction::Share {
            return if view.shown {
                self.open_share(&id)
            } else {
                Vec::new()
            };
        }
        if !view.shown {
            return Vec::new();
        }
        if let Some(reason) = view.reason {
            if let Some(g) = &mut self.gallery {
                g.note = Some(reason);
            }
            return Vec::new();
        }
        match a {
            TemplateAction::Use => self.use_card(&id, true),
            TemplateAction::Edit => {
                let Some(t) = self.picked().cloned() else {
                    return Vec::new();
                };
                if !self.use_template_as(&t, None, &format!("Şablonu düzenle: {name}")) {
                    return Vec::new();
                }
                self.gallery = None;
                let own = matches!(source, CardSource::Device | CardSource::Cloud)
                    || (source == CardSource::Shared && role == Some(TemplateRole::Editor))
                    || (source == CardSource::Organisation
                        && matches!(role, Some(TemplateRole::Owner | TemplateRole::Admin)));
                let said = if own {
                    format!(
                        "“{name}” paftada açıldı. Değişiklikleri Şablon olarak kaydet → “Bu şablonu güncelle” ile geri yazın."
                    )
                } else {
                    format!(
                        "“{name}” paftada açıldı. Şablon olarak kaydet ile kendi şablonunuz olarak saklayın; {} değişmez.",
                        match source {
                            CardSource::System => "sistem şablonu",
                            CardSource::Organisation => "kurumun şablonu",
                            _ => "paylaşılan şablon",
                        }
                    )
                };
                vec![Effect::ShowSheet, Effect::Say(Say::Info, said)]
            }
            TemplateAction::Duplicate => {
                let Some(mut t) = self.picked().cloned() else {
                    return Vec::new();
                };
                let new = self.new_template_id();
                let today = self.today();
                t.meta.id = new.clone();
                t.meta.revision = 1;
                t.meta.name = format!("{name} (kopya)");
                t.meta.created = today.clone();
                t.meta.updated = today;
                t.meta.author = self.author();
                let saved = t.meta.name.clone();
                if !self.keep_template(&new, t, None) {
                    return Vec::new();
                }
                // The copy is shown: it is the one to rename or change.
                if let Some(g) = &mut self.gallery {
                    g.source = Source::Mine;
                    g.selected = Some(new);
                }
                vec![Effect::Say(
                    Say::Success,
                    format!("“{saved}” şablonlarınıza eklendi (Benim, bu cihazda)."),
                )]
            }
            TemplateAction::Sync => {
                if source == CardSource::Device {
                    self.upload_template(&id)
                } else {
                    vec![Effect::Library(LibraryRequest::Sync)]
                }
            }
            TemplateAction::Delete => {
                let organisation = self.picked_card().and_then(|c| c.org).map(str::to_owned);
                if let Some(g) = &mut self.gallery {
                    g.asking = Some(Asking::Delete {
                        id,
                        name,
                        cloud: matches!(source, CardSource::Cloud | CardSource::Organisation),
                        organisation,
                    });
                }
                Vec::new()
            }
            TemplateAction::Publish => {
                // Its changes here go up first: the organisation gets the cloud's newest revision.
                let waiting = self.picked_card().is_some_and(|c| {
                    c.badges
                        .iter()
                        .any(|b| matches!(b, Badge::Unsynced | Badge::Syncing))
                });
                if waiting {
                    if let Some(g) = &mut self.gallery {
                        g.note = Some(library::texts::PUBLISH_WAIT_SYNC);
                    }
                    return vec![Effect::Library(LibraryRequest::Sync)];
                }
                self.open_publish(&id)
            }
            TemplateAction::Share => Vec::new(),
        }
    }

    /// Kullan: a sheet from the chosen card on the paper chosen; asks first when the project lacks what it needs.
    fn use_card(&mut self, id: &str, ask: bool) -> Vec<Effect> {
        let Some(t) = self
            .picked_card()
            .filter(|c| c.id == id)
            .map(|c| c.template.clone())
        else {
            return Vec::new();
        };
        let needs: Vec<String> = self
            .gallery
            .as_ref()
            .and_then(|g| g.thumbs.get(id))
            .map(|th| {
                th.needs
                    .iter()
                    .map(|f| format!("{} {}", f.message, f.fix))
                    .collect()
            })
            .unwrap_or_default();
        if ask && !needs.is_empty() {
            if let Some(g) = &mut self.gallery {
                g.asking = Some(Asking::Needs {
                    id: id.to_owned(),
                    name: t.meta.name.clone(),
                    needs,
                });
            }
            return Vec::new();
        }
        // The template's own paper needs no relayout.
        let choice = self
            .gallery
            .as_ref()
            .and_then(|g| g.use_paper)
            .filter(|c| t.meta.papers.first() != Some(c));
        // Its questions first (the web's `useTemplate`): the gallery stays under them.
        let label = format!("Şablondan pafta: {}", t.meta.name);
        self.ask_then_use(&t, choice, label)
    }

    /// “Buluta eşitle”: a template of this device is marked to go up to the account's library; the host syncs.
    pub(crate) fn upload_template(&mut self, id: &str) -> Vec<Effect> {
        let Some(account) = self.library.account.clone() else {
            return vec![Effect::Say(Say::Warn, library::texts::SIGN_IN.to_owned())];
        };
        let Some(r) = self.user_templates.iter().find(|r| r.id == id).cloned() else {
            return Vec::new();
        };
        let name = r.template.meta.name.clone();
        if r.cloud.is_none()
            && !self.keep_template(
                id,
                r.template,
                Some(DeviceCloudState::to_upload(&account.id)),
            )
        {
            return Vec::new();
        }
        vec![Effect::Library(LibraryRequest::Upload {
            id: id.to_owned(),
            name,
        })]
    }

    /// Sil, answered: one of this device at once; one of the library everywhere, when the cloud hears of it.
    fn delete_template(&mut self, id: &str) -> Vec<Effect> {
        let Some(r) = self.user_templates.iter().find(|r| r.id == id).cloned() else {
            return Vec::new();
        };
        let name = r.template.meta.name.clone();
        if r.cloud.as_ref().is_some_and(|c| {
            !(c.role == TemplateRole::Owner
                || (c.organization.is_some() && c.role == TemplateRole::Admin))
        }) {
            return vec![Effect::Say(Say::Warn, library::texts::NOT_OWNER.to_owned())];
        }
        if let Some(store) = self.store.clone() {
            let done = match &r.cloud {
                Some(c) if c.revision > 0 => {
                    let gone = DeviceCloudState {
                        deleted: true,
                        ..c.clone()
                    };
                    store.save_template(id, &r.template, Some(&gone))
                }
                _ => store.remove_template(id),
            };
            if let Err(e) = done {
                return vec![Effect::Say(Say::Error, format!("Şablon silinemedi: {e}"))];
            }
        } else {
            self.user_templates.retain(|x| x.id != id);
        }
        if let Some(g) = &mut self.gallery {
            g.selected = None;
            g.thumbs.remove(id);
        }
        let mut out = self.library_changed();
        if r.cloud.is_some() {
            out.push(Effect::Say(Say::Success, library::texts::deleted(&name)));
            out.push(Effect::Library(LibraryRequest::Sync));
        } else {
            out.push(Effect::Say(
                Say::Success,
                format!("“{name}” bu cihazdan silindi."),
            ));
        }
        out
    }

    /// A new id for a template of this device: a UUIDv7, as the web's (the cloud gives its own later).
    pub(crate) fn new_template_id(&self) -> String {
        uuid::Uuid::now_v7().to_string()
    }

    /// Today, as a template's dates write it.
    pub(crate) fn today(&self) -> String {
        if self.ctx.project.date.is_empty() {
            crate::text::today()
        } else {
            self.ctx.project.date.clone()
        }
    }

    /// Who writes a template: the signed-in account, else the computer's user.
    pub(crate) fn author(&self) -> String {
        self.library
            .account
            .as_ref()
            .map_or_else(|| self.ctx.project.user.clone(), |a| a.name.clone())
    }

    /// Keeps a template under `id` (in the store, when there is one); false when it could not be written.
    pub(crate) fn keep_template(
        &mut self,
        id: &str,
        t: Template,
        cloud: Option<DeviceCloudState>,
    ) -> bool {
        if let Some(store) = &self.store
            && let Err(e) = store.save_template(id, &t, cloud.as_ref())
        {
            self.error = Some(format!("Şablon kaydedilemedi: {e}"));
            return false;
        }
        self.user_templates.retain(|x| x.id != id);
        self.user_templates.push(StoredTemplate {
            id: id.to_owned(),
            saved: 0,
            template: t,
            cloud,
        });
        self.user_templates.sort_by(|a, b| {
            a.template
                .meta
                .name
                .cmp(&b.template.meta.name)
                .then_with(|| a.id.cmp(&b.id))
        });
        true
    }

    /// A new sheet from a template, in front; on `paper` when given (its items follow their constraints).
    pub fn use_template(&mut self, t: &Template, paper: Option<PaperChoice>) -> bool {
        let label = format!("Şablondan pafta: {}", t.meta.name);
        self.use_template_as(t, paper, &label)
    }

    /// The same, as one undo step named `label`.
    pub(crate) fn use_template_as(
        &mut self,
        t: &Template,
        paper: Option<PaperChoice>,
        label: &str,
    ) -> bool {
        self.use_template_with(t, paper, label, Vec::new())
    }

    /// The same with the answers to its questions (none: its own values).
    pub(crate) fn use_template_with(
        &mut self,
        t: &Template,
        paper: Option<PaperChoice>,
        label: &str,
        values: Vec<kentos_sheet::template::VariableValue>,
    ) -> bool {
        let ids = InstanceIds {
            sheet: self.new_id("pafta"),
            master: t.master.as_ref().map(|_| self.new_id("ana")),
            items: t.sheet.items.iter().map(|_| self.new_id("o")).collect(),
            master_items: t
                .master
                .iter()
                .flat_map(|m| m.items.iter())
                .map(|_| self.new_id("ao"))
                .collect(),
        };
        // The template's own sheet name, as the web's (the core names it).
        let options = InstanceOptions {
            paper,
            name: None,
            center: self.ctx.center,
            scale: self.ctx.capabilities.plot_scale,
            values,
        };
        let inst = match instantiate(t, &ids, &options) {
            Ok(i) => i,
            Err(e) => {
                self.error = Some(e.message);
                return false;
            }
        };
        for a in &inst.asset_bytes {
            if let Some(bytes) = kentos_sheet::template::base64_decode(&a.data) {
                self.add_asset_bytes(&a.meta.sha256, bytes);
            }
        }
        let id = inst.sheet.id.clone();
        let mut ops = Vec::new();
        if !inst.assets.is_empty() {
            ops.push(Op::AddAssets(AddAssets {
                assets: inst.assets,
            }));
        }
        if let Some(m) = inst.master {
            ops.push(Op::AddMaster(AddMaster {
                master: m,
                index: None,
            }));
        }
        ops.push(Op::AddSheet(AddSheet {
            sheet: inst.sheet,
            index: None,
        }));
        if self.commit_as(ops, Some(label)) {
            self.open = Some(id);
            self.selection.clear();
            self.view = crate::view_math::View::default();
            self.refresh();
            true
        } else {
            false
        }
    }
}

// ── The window ───────────────────────────────────────────────────────────

/// What one canvas of a card's picture draws: the paper with its shadow and the first layer, a
/// later layer, or the map or picture between two layers (the plan's order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThumbPart {
    Layer(usize),
    Between(usize),
}

/// A card's picture, one canvas of it: the paper with its shadow, fitted in the box, its maps the
/// host's; every canvas of the picture fits it alike.
struct ThumbProgram<'a> {
    thumb: Option<&'a Thumb>,
    painter: Painter<'a>,
    part: ThumbPart,
}

impl ThumbProgram<'_> {
    /// The canvas's cache: the layers first, then the maps and pictures between them.
    fn cache(thumb: &Thumb, part: ThumbPart) -> Option<&canvas::Cache> {
        let at = match part {
            ThumbPart::Layer(i) => i,
            ThumbPart::Between(i) => thumb.plan.layers.len() + i,
        };
        thumb.caches.get(at)
    }
}

impl canvas::Program<Message> for ThumbProgram<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let Some(thumb) = self.thumb else {
            return Vec::new();
        };
        let Some(list) = &thumb.list else {
            if self.part != ThumbPart::Layer(0) {
                return Vec::new();
            }
            let mut frame = canvas::Frame::new(renderer, bounds.size());
            frame.fill_text(canvas::Text {
                content: "çizilemedi".into(),
                position: frame.center(),
                color: Tokens::of(theme).muted,
                align_x: iced::widget::text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
            return vec![frame.into_geometry()];
        };
        let revision = self.painter.revision();
        if thumb.painted.get() != revision {
            for c in &thumb.caches {
                c.clear();
            }
            thumb.painted.set(revision);
        }
        let Some(cache) = Self::cache(thumb, self.part) else {
            return Vec::new();
        };
        let geometry = cache.draw(renderer, bounds.size(), |frame| {
            let (pw, ph) = (f64::from(list.size.width), f64::from(list.size.height));
            let room = (
                f64::from(bounds.width) - 12.0,
                f64::from(bounds.height) - 12.0,
            );
            let k = (room.0 / pw).min(room.1 / ph).max(1e-6);
            let (w, h) = ((pw * k) as f32, (ph * k) as f32);
            let origin = Point::new((bounds.width - w) / 2.0, (bounds.height - h) / 2.0);
            let xf = Xf::new(k, origin);
            let pictures = |sha: &str| thumb.pictures.get(sha).cloned().flatten();
            match self.part {
                ThumbPart::Layer(i) => {
                    if i == 0 {
                        frame.fill_rectangle(
                            origin + Vector::new(0.0, 1.5),
                            Size::new(w, h),
                            iced::Color::from_rgba(0.0, 0.0, 0.0, 0.18),
                        );
                        paint::paper(frame, list, &xf);
                    }
                    if let Some(entries) = thumb.plan.layers.get(i) {
                        paint::paint_entries(frame, list, entries, &xf, &pictures, Options::SCREEN);
                    }
                }
                ThumbPart::Between(i) => {
                    let Some(e) = thumb.plan.maps.get(i) else {
                        return;
                    };
                    match list.prims.get(e.prim) {
                        Some(Prim::Map(m)) => paint::paint_map_in_place(
                            frame,
                            m,
                            &xf,
                            Maps::Painter(&*self.painter, false),
                            e.alpha,
                        ),
                        Some(Prim::Image(_)) => paint::paint_entries(
                            frame,
                            list,
                            std::slice::from_ref(e),
                            &xf,
                            &pictures,
                            Options::SCREEN,
                        ),
                        _ => {}
                    }
                }
            }
        });
        vec![geometry]
    }
}

/// A card's picture as its canvases stacked in the plan's order (a picture and a map's content
/// keep their place among the shapes, as on the stage).
fn thumb_stack<'a>(
    g: &'a Gallery,
    id: &str,
    painter: &Painter<'a>,
    width: impl Into<iced::Length> + Copy,
    height: f32,
) -> Element<'a, Message> {
    let made = g.thumbs.get(id);
    let parts: Vec<ThumbPart> = match made {
        Some(t) if t.list.is_some() => {
            let mut parts = Vec::new();
            for i in 0..t.plan.layers.len() {
                parts.push(ThumbPart::Layer(i));
                if i < t.plan.maps.len() {
                    parts.push(ThumbPart::Between(i));
                }
            }
            parts
        }
        _ => vec![ThumbPart::Layer(0)],
    };
    let canvases: Vec<Element<'a, Message>> = parts
        .into_iter()
        .map(|part| {
            canvas(ThumbProgram {
                thumb: made,
                painter: painter.clone(),
                part,
            })
            .width(Fill)
            .height(Fill)
            .into()
        })
        .collect();
    iced::widget::Stack::with_children(canvases)
        .width(width)
        .height(height)
        .into()
}

fn thumb<'a>(
    g: &'a Gallery,
    id: &str,
    painter: &Painter<'a>,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    thumb_stack(g, id, painter, width, height)
}

/// The gallery's size in the room the window gives it: the web's at its
/// largest (1240 wide, a body 580 high), smaller in a smaller window, the
/// cards two to a row when three would be too narrow.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Fitted {
    /// The window's width, as [`Dialog::width`] takes it (unscaled).
    width: f32,
    /// The body's height: sections, cards and details.
    body: f32,
    columns: usize,
}

impl Fitted {
    /// Title, filters and foot with the window's padding (12-pixel text), and the room left around it.
    const CHROME: f32 = 160.0;
    const MARGIN: f32 = 24.0;
    /// The sections, the details and the spaces between them beside the cards.
    const BESIDE: f32 = 214.0 + 16.0 + 300.0 + 16.0 + 36.0;

    fn of(room: Size) -> Fitted {
        let width = typography::unscaled(room.width - 2.0 * Self::MARGIN).clamp(860.0, 1240.0);
        let chrome = typography::scaled(Self::CHROME);
        let body = (room.height - chrome - 2.0 * Self::MARGIN).clamp(260.0, 580.0);
        let list = typography::scaled(width) - Self::BESIDE;
        Fitted {
            width,
            body,
            columns: if list >= 600.0 { 3 } else { 2 },
        }
    }
}

/// A card's picture as wide as its card.
fn thumb_fill<'a>(
    g: &'a Gallery,
    id: &str,
    painter: &Painter<'a>,
    height: f32,
) -> Element<'a, Message> {
    thumb_stack(g, id, painter, Fill, height)
}

fn badge<'a>(b: Badge) -> Element<'a, Message> {
    badge_text(b, b.label().to_owned())
}

/// A badge with its own words (“Kurum: Harita Bürosu”).
fn badge_text<'a>(b: Badge, text: String) -> Element<'a, Message> {
    tip(
        container(label::caption(text).style(if b.warns() {
            warning
        } else {
            style::text::muted
        }))
        .padding([1, 6])
        .style(style::container::badge),
        Tip::new(b.tip()),
        iced::widget::tooltip::Position::Top,
    )
}

fn badges<'a>(c: &Card<'_>) -> Element<'a, Message> {
    let mut r = row![].spacing(4);
    for b in &c.badges {
        r = r.push(match (b, c.org) {
            (Badge::Organisation, Some(org)) => badge_text(*b, library::texts::organisation(org)),
            _ => badge(*b),
        });
    }
    if let Some(m) = &c.mode {
        r = r.push(
            container(label::caption(m.clone()).style(style::text::muted))
                .padding([1, 6])
                .style(style::container::badge),
        );
    }
    r.wrap().into()
}

/// The question over the gallery.
fn question(asking: &Asking) -> Element<'_, Message> {
    let gm = Message::Gallery;
    let (title, message, detail, confirm) = match asking {
        Asking::Delete {
            name,
            cloud,
            organisation,
            ..
        } => (
            "Şablonu sil",
            match (organisation, *cloud) {
                (Some(org), _) => format!("“{name}” “{org}” kurumunun şablonlarından silinsin mi?"),
                (None, true) => format!("“{name}” bulut hesabınızdan silinsin mi?"),
                (None, false) => format!("“{name}” bu cihazdan silinsin mi?"),
            },
            match (organisation, *cloud) {
                (Some(_), _) => "Kurumun bütün üyelerinin listesinden kalkar. Ondan yapılmış paftalar kalır: onlar şablonun kopyasıdır.".to_owned(),
                (None, true) => "Web ve masaüstündeki bütün cihazlarınızdan, paylaştığınız kişilerin listesinden de kalkar. Ondan yapılmış paftalar kalır: onlar şablonun kopyasıdır."
                    .to_owned(),
                (None, false) => "Ondan yapılmış paftalar kalır: onlar şablonun kopyasıdır.".to_owned(),
            },
            "Sil",
        ),
        Asking::Needs { name, needs, .. } => (
            "Projede eksik olanlar var",
            format!("“{name}” şablonundan yapılacak paftada eksikler olacak:"),
            needs.join("\n"),
            "Yine de kullan",
        ),
    };
    let mut c = Confirm::new(
        title,
        gm(GalleryMessage::Answer(true)),
        gm(GalleryMessage::Answer(false)),
    )
    .message(message)
    .detail(detail)
    .confirm(confirm);
    if matches!(asking, Asking::Delete { .. }) {
        c = c.destructive();
    }
    c.into()
}

impl Designer {
    /// The gallery and what stands over it (Şablonu paylaş, a question).
    pub(crate) fn gallery_window<'a>(
        &'a self,
        g: &'a Gallery,
        painter: Painter<'a>,
    ) -> Element<'a, Message> {
        let gm = Message::Gallery;
        // As large as the window lets it be (the web's: the viewport less a margin).
        let base: Element<'a, Message> = responsive(move |room| {
            overlay::modal(
                self.gallery_view(g, painter.clone(), Fitted::of(room)),
                gm(GalleryMessage::Close),
            )
        })
        .into();
        if let Some(d) = &g.share {
            return stack![
                base,
                overlay::modal(
                    self.share_window(d),
                    Message::Share(crate::share_template::ShareMessage::Close)
                )
            ]
            .into();
        }
        if let Some(d) = &g.publish {
            return stack![
                base,
                overlay::modal(
                    self.publish_window(d),
                    Message::Publish(crate::publish_template::PublishMessage::Close)
                )
            ]
            .into();
        }
        if let Some(a) = &g.asking {
            return stack![
                base,
                overlay::modal(question(a), gm(GalleryMessage::Answer(false)))
            ]
            .into();
        }
        base
    }

    /// The gallery window.
    fn gallery_view<'a>(
        &'a self,
        g: &'a Gallery,
        painter: Painter<'a>,
        fit: Fitted,
    ) -> Element<'a, Message> {
        let gm = Message::Gallery;
        let every = self.every_card();
        // The sections, each with its count or why it has none.
        let mut nav = column![].spacing(2).width(214);
        for s in Source::ALL {
            let state = self.section_state(s, &every);
            let shown = every
                .iter()
                .filter(|c| Source::of(c.source) == s && (g.all_modes || c.matches))
                .count();
            let mut lines = column![
                row![
                    icon(s.glyph()).size(15.0),
                    label::body(s.to_string()).width(Fill),
                    label::caption(if state == State::Ready {
                        shown.to_string()
                    } else {
                        String::new()
                    })
                    .style(style::text::muted)
                ]
                .spacing(8)
                .align_y(Center)
            ]
            .spacing(2);
            match state {
                State::Ready => {}
                State::Soon(why) | State::Unavailable(why) => {
                    lines = lines.push(label::caption(why).style(style::text::muted));
                }
            }
            nav = nav.push(
                button(lines)
                    .on_press(gm(GalleryMessage::Source(s)))
                    .width(Fill)
                    .padding([6, 10])
                    .style(style::button::navigation(g.source == s)),
            );
        }
        nav = nav.push(space().height(8));
        nav = nav.push(
            label::caption(format!(
                "Sıra: önce projenin türüne, sonra çalışma moduna ({}) uyanlar, sonra ortak şablonlar.{}",
                mode_label(self.ctx.workspace),
                if g.all_modes {
                    " Başka kiplerinkiler en sonda, kip rozetiyle."
                } else {
                    ""
                }
            ))
            .style(style::text::muted),
        );
        let line = self.library.line();
        if !line.is_empty() {
            nav = nav.push(space().height(6));
            nav = nav.push(
                row![
                    icon(crate::icons::CLOUD).size(14.0),
                    label::caption(line).style(style::text::muted)
                ]
                .spacing(6),
            );
        }
        if g.source == Source::Mine {
            for r in &self.refused {
                nav = nav.push(
                    label::caption(library::texts::device_refused(&r.file, &r.reason))
                        .style(warning),
                );
            }
        }

        // The filters: the papers and kinds the section's cards use.
        let mut papers: Vec<Paper> = Vec::new();
        let mut kinds: Vec<String> = Vec::new();
        for c in every.iter().filter(|c| Source::of(c.source) == g.source) {
            for p in &c.template.meta.papers {
                if !papers.contains(&p.paper) {
                    papers.push(p.paper);
                }
            }
            let k = &c.template.meta.category;
            if !k.is_empty() && !kinds.contains(k) {
                kinds.push(k.clone());
            }
        }
        papers.sort_by_key(|p| series(*p));
        kinds.sort_by_key(|k| crate::text::fold(k));
        let paper_choices: Vec<Choice> = std::iter::once(Choice::new("Kâğıt: Hepsi"))
            .chain(
                papers
                    .iter()
                    .map(|p| Choice::new(format!("Kâğıt: {}", paper_name(*p)))),
            )
            .collect();
        let paper_at = g
            .paper
            .and_then(|p| papers.iter().position(|x| *x == p))
            .map_or(0, |i| i + 1);
        let kind_choices: Vec<Choice> = std::iter::once(Choice::new("Tür: Hepsi"))
            .chain(
                kinds
                    .iter()
                    .map(|k| Choice::new(format!("Tür: {}", category_label(k)))),
            )
            .collect();
        let kind_at = g
            .kind
            .as_ref()
            .and_then(|k| kinds.iter().position(|x| x == k))
            .map_or(0, |i| i + 1);
        let filters = row![
            InputField::new("Şablon ara: ifraz, imar, rapor…", &g.query)
                .on_input(move |q| gm(GalleryMessage::Query(q)))
                .icon(Icon::Search)
                .clear(gm(GalleryMessage::Query(String::new())))
                .width(Fill),
            container(
                Select::new(paper_choices, Some(paper_at), move |i| {
                    gm(GalleryMessage::Paper(
                        i.checked_sub(1).and_then(|i| papers.get(i).copied()),
                    ))
                })
                .searchable(false)
            )
            .width(150),
            container(
                Select::new(kind_choices, Some(kind_at), move |i| {
                    gm(GalleryMessage::Kind(
                        i.checked_sub(1).and_then(|i| kinds.get(i).cloned()),
                    ))
                })
                .searchable(false)
            )
            .width(170),
            tip(
                Switch::new(g.all_modes, move |on| gm(GalleryMessage::AllModes(on)))
                    .label("Bütün kiplerin şablonları"),
                Tip::new("Bütün kiplerin şablonları").body(
                    "Başka çalışma modlarının şablonlarını da gösterir; kip rozetiyle. Veri kipten bağımsızdır: her şablon kullanılabilir."
                ),
                iced::widget::tooltip::Position::Bottom,
            ),
        ]
        .spacing(10)
        .align_y(Center);

        // The cards, three a row.
        let cards = self.cards(g);
        let state = self.section_state(g.source, &every);
        let hidden = if g.all_modes {
            0
        } else {
            every
                .iter()
                .filter(|c| Source::of(c.source) == g.source && !c.matches)
                .count()
        };
        let mut head = row![label::caption(if state == State::Ready {
            format!("{}: {} şablon", g.source, cards.len())
        } else {
            g.source.to_string()
        })]
        .spacing(6);
        if hidden > 0 {
            head = head.push(
                label::caption(format!(
                    "· {hidden} şablon başka kiplerin (“Bütün kiplerin şablonları” ile görünür)"
                ))
                .style(style::text::muted),
            );
        }
        let searching = !g.query.trim().is_empty() || g.paper.is_some() || g.kind.is_some();
        let body: Element<'a, Message> = if cards.is_empty() {
            let (glyph, title, sub): (Icon, &str, &str) = match state {
                State::Soon(why) => (crate::icons::HISTORY, "Yakında", why),
                State::Unavailable(why) => (Icon::Info, "Şimdi listelenemiyor", why),
                State::Ready if searching => (
                    crate::icons::TEMPLATE,
                    "Eşleşen şablon yok",
                    "Aramayı ya da kâğıt ve tür süzgeçlerini değiştirin; başka kiplerin şablonları için “Bütün kiplerin şablonları”nı açın.",
                ),
                State::Ready => match g.source {
                    Source::System => (
                        crate::icons::TEMPLATE,
                        "Sistem şablonu yok",
                        "Sistem şablonları uygulamayla gelir.",
                    ),
                    Source::Mine => (
                        crate::icons::TEMPLATE,
                        "Henüz şablonunuz yok",
                        "Bir paftayı şeridin Pafta sekmesinde “Şablon olarak kaydet” ile buraya alın; önce bu cihazda saklanır.",
                    ),
                    Source::Shared => (
                        crate::icons::TEMPLATE,
                        "Sizinle paylaşılan şablon yok",
                        "Biri bir şablonu sizinle paylaştığında burada görünür.",
                    ),
                    Source::Organisation => (
                        crate::icons::TEMPLATE,
                        "Kurum şablonu yok",
                        "Kurumunuzun yayımladığı şablonlar burada görünür.",
                    ),
                },
            };
            container(
                column![
                    icon(glyph).size(28.0),
                    label::strong(title),
                    label::muted(sub)
                ]
                .spacing(6)
                .align_x(Center)
                .max_width(420),
            )
            .center(Fill)
            .into()
        } else {
            let mut grid = column![].spacing(12);
            // Kurumum: under each organisation's name (by name, as the list gives them).
            let groups: Vec<(Option<&str>, Vec<&Card<'_>>)> = if g.source == Source::Organisation {
                let mut by: BTreeMap<(String, String), Vec<&Card<'_>>> = BTreeMap::new();
                for c in &cards {
                    let name = c.org.unwrap_or_default();
                    by.entry((crate::text::fold(name), name.to_owned()))
                        .or_default()
                        .push(c);
                }
                by.into_iter()
                    .map(|((_, name), list)| {
                        let head = list
                            .first()
                            .and_then(|c| c.org)
                            .filter(|_| !name.is_empty());
                        (head, list)
                    })
                    .collect()
            } else {
                vec![(None, cards.iter().collect())]
            };
            for (head, list) in groups {
                if let Some(h) = head {
                    grid = grid.push(
                        row![icon(Icon::Layers).size(14.0), label::strong(h)]
                            .spacing(6)
                            .align_y(Center),
                    );
                }
                for chunk in list.chunks(fit.columns) {
                    let mut r = row![].spacing(12);
                    for c in chunk.iter().copied() {
                        let selected = g.selected.as_deref() == Some(c.id);
                        let meta = format!(
                            "{} · {}",
                            c.template
                                .meta
                                .papers
                                .first()
                                .map(paper_text)
                                .unwrap_or_default(),
                            category_label(&c.template.meta.category)
                        );
                        // Three to a row, each a third of the list's width (the web's grid).
                        let card = column![
                            container(thumb_fill(g, c.id, &painter, 124.0))
                                .width(Fill)
                                .style(style::container::bordered),
                            label::strong(c.template.meta.name.clone()),
                            label::caption(meta).style(style::text::muted),
                            badges(c),
                        ]
                        .spacing(4)
                        .width(Fill);
                        r = r.push(
                            button(card)
                                .on_press(gm(GalleryMessage::Pick(c.id.to_owned())))
                                .padding(5)
                                .width(Fill)
                                .style(style::button::library_tile(selected)),
                        );
                    }
                    // A short last row keeps the cards' width.
                    for _ in chunk.len()..fit.columns {
                        r = r.push(space().width(Fill));
                    }
                    grid = grid.push(r);
                }
            }
            scrollable(container(grid).padding(iced::Padding::from([4, 2]).right(14)))
                .height(Fill)
                .into()
        };

        let details = self.details_view(g, &painter);
        let content = column![
            filters,
            row![
                nav,
                column![head, body].spacing(8).width(Fill),
                container(details).width(300).height(Fill)
            ]
            .spacing(16)
            .height(fit.body),
        ]
        .spacing(12);

        // At the foot: why the chosen action cannot run, the paper Kullan uses, Kapat, Kullan.
        let picked = self.picked_card();
        let mut window = Dialog::new("Pafta şablonları").hint("Esc").push(content);
        if let Some(note) = g.note {
            window = window.aside(
                row![
                    icon(Icon::Warning).size(14.0),
                    label::caption(note).style(warning)
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        if let Some(c) = &picked
            && !c.template.meta.papers.is_empty()
        {
            let papers = c.template.meta.papers.clone();
            let chosen = g
                .use_paper
                .and_then(|p| papers.iter().position(|x| *x == p))
                .unwrap_or(0);
            let choices: Vec<Choice> = papers
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    Choice::new(format!(
                        "{}{}",
                        paper_text(p),
                        if i == 0 { " (önerilen)" } else { "" }
                    ))
                })
                .collect();
            window = window.action(
                row![
                    label::caption("Kâğıt").style(style::text::muted),
                    container(
                        Select::new(choices, Some(chosen), move |i| {
                            gm(GalleryMessage::UsePaper(
                                (i > 0).then(|| papers.get(i).copied()).flatten(),
                            ))
                        })
                        .searchable(false)
                    )
                    .width(190)
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        let use_button = tip(
            button(
                row![icon(Icon::Check).size(16.0), label::body("Kullan")]
                    .spacing(6)
                    .align_y(Center),
            )
            .on_press_maybe(picked.is_some().then(|| gm(GalleryMessage::Use)))
            .padding([6, 16])
            .style(style::button::primary),
            Tip::new("Kullan").body(TemplateAction::Use.tip()),
            iced::widget::tooltip::Position::Top,
        );
        window
            .action(
                button(label::body("Kapat"))
                    .on_press(gm(GalleryMessage::Close))
                    .padding([6, 14])
                    .style(style::button::secondary),
            )
            .action(use_button)
            .width(fit.width)
            .into()
    }

    /// The chosen card on the right: picture, badges, fit, actions, needs, what it is.
    fn details_view<'a>(&'a self, g: &'a Gallery, painter: &Painter<'a>) -> Element<'a, Message> {
        let gm = Message::Gallery;
        let Some(c) = self.picked_card() else {
            return container(
                column![
                    icon(crate::icons::TEMPLATE).size(28.0),
                    label::strong("Bir şablon seçin"),
                    label::muted(
                        "Sistem şablonları salt okunurdur: kullanılır, kopyalanır, kopyası düzenlenir. Kendi şablonlarınız önce bu cihazda saklanır; buluta eşitlenince paylaşılır."
                    ),
                ]
                .spacing(6)
                .align_x(Center),
            )
            .center(Fill)
            .into();
        };
        let can = self.library.can();
        let mut actions = row![].spacing(6);
        for a in TemplateAction::DETAILS {
            let v = library::action(c.source, c.role, can, a);
            if !v.shown {
                continue;
            }
            let glyph = match a {
                TemplateAction::Duplicate => crate::icons::COPY,
                TemplateAction::Edit => crate::icons::EDIT,
                TemplateAction::Share => crate::icons::SHARE,
                TemplateAction::Publish => Icon::Layers,
                TemplateAction::Sync => crate::icons::CLOUD,
                TemplateAction::Use | TemplateAction::Delete => crate::icons::TRASH,
            };
            // Paylaş… opens even when it cannot share yet: its window says why (design §13).
            let on = v.reason.is_none() || a == TemplateAction::Share;
            let b = button(
                row![icon(glyph).size(14.0), label::caption(v.label)]
                    .spacing(4)
                    .align_y(Center),
            )
            .on_press_maybe(on.then(|| gm(GalleryMessage::Act(a))))
            .padding([4, 8])
            .style(if a == TemplateAction::Delete {
                style::button::danger_outline
            } else {
                style::button::secondary
            });
            // The reason it cannot run now, under what it does (the web's tooltip note).
            let t = Tip::new(v.label).body(match v.reason {
                Some(reason) => format!("{}\n{reason}", a.tip()),
                None => a.tip().to_owned(),
            });
            actions = actions.push(tip(b, t, iced::widget::tooltip::Position::Top));
        }
        let m = &c.template.meta;
        let field = |k: &'static str, v: String| {
            column![label::caption(k).style(style::text::muted), label::body(v)].spacing(1)
        };
        let mut col = column![
            container(thumb(g, c.id, painter, 280.0, 182.0)).style(style::container::bordered),
            label::heading(m.name.clone()),
            badges(&c),
            row![
                icon(if c.fit == TemplateFit::Other {
                    Icon::Info
                } else {
                    Icon::Check
                })
                .size(14.0),
                label::caption(fit_label(c.fit))
            ]
            .spacing(6)
            .align_y(Center),
            actions.wrap(),
        ]
        .spacing(8);
        if let Some(th) = g.thumbs.get(c.id)
            && !th.needs.is_empty()
        {
            let said: Vec<&str> = th.needs.iter().map(|f| f.message.as_str()).collect();
            col = col.push(
                container(
                    row![
                        icon(Icon::Warning).size(14.0),
                        label::caption(format!(
                            "Projede eksik: {} Kullanmadan önce sorulur.",
                            said.join(" ")
                        ))
                    ]
                    .spacing(6),
                )
                .padding(8)
                .style(style::container::bordered),
            );
        }
        if c.conflict {
            col = col.push(
                container(
                    row![
                        icon(Icon::Info).size(14.0),
                        label::caption(
                            "Çakışmada ayrılan kopya. Siz bu cihazda değiştirirken şablon bulutta da değişmişti: bu, sizin sürümünüz; bulutun sürümü asıl adıyla ayrıca duruyor. İkisini karşılaştırıp birini silebilirsiniz."
                        )
                    ]
                    .spacing(6),
                )
                .padding(8)
                .style(style::container::bordered),
            );
        }
        let mut fields = column![].spacing(6);
        if !m.description.is_empty() {
            fields = fields.push(field("Açıklama", m.description.clone()));
        }
        if let Some(first) = m.papers.first() {
            let size = paper_mm(first).map_or(String::new(), |(w, h)| {
                format!(
                    " ({} × {} mm)",
                    kentos_geometry_core::display::fixed(w, 0),
                    kentos_geometry_core::display::fixed(h, 0)
                )
            });
            let rest = m.papers.iter().skip(1).map(paper_text);
            fields = fields.push(field(
                if m.papers.len() > 1 {
                    "Önerilen kâğıtlar"
                } else {
                    "Önerilen kâğıt"
                },
                std::iter::once(format!("{}{size}", paper_text(first)))
                    .chain(rest)
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
        fields = fields.push(field("Yerleşim düzenleri", layouts_of(c.template)));
        fields = fields.push(field("Tür", category_label(&m.category)));
        let common = m.workspaces.is_empty()
            || (m.workspaces.contains(&Workspace::Cad) && m.workspaces.contains(&Workspace::Gis));
        fields = fields.push(field(
            "Çalışma modu",
            if common {
                "Ortak (bütün kipler)".to_owned()
            } else {
                m.workspaces
                    .iter()
                    .map(|w| mode_label(Some(*w)))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        ));
        if !m.project_types.is_empty() {
            fields = fields.push(field(
                "Proje türleri",
                m.project_types
                    .iter()
                    .map(|t| type_label(*t))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
        fields = fields.push(field(
            "Sürüm",
            if m.updated.is_empty() {
                m.revision.to_string()
            } else {
                format!(
                    "{} · {}",
                    m.revision,
                    m.updated.get(..10).unwrap_or(&m.updated)
                )
            },
        ));
        fields = fields.push(field("Saklandığı yer", where_text(&c)));
        if !m.author.is_empty() && c.source != CardSource::Shared {
            fields = fields.push(field("Yazar", m.author.clone()));
        }
        fields = fields.push(field("Kimlik", c.id.to_owned()));
        col = col.push(fields);
        // Room for the scroll bar: nothing of the details goes under it.
        scrollable(container(col).padding(iced::Padding::ZERO.right(14)))
            .height(Fill)
            .into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct OrientationName(pub Orientation);

impl std::fmt::Display for OrientationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0 == Orientation::Landscape {
            "Yatay"
        } else {
            "Dikey"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A card's picture keeps the items' order (the stage's rule): a shape drawn after a picture
    /// is over it, though a renderer draws a canvas's pictures over its shapes.
    #[test]
    fn a_card_s_picture_keeps_what_is_drawn_after_a_picture_over_it() {
        use kentos_sheet::kinds::{ItemKind, PictureFit, PictureItem, default_kind};
        use kentos_sheet::model::{AssetKind, AssetMeta, Item};
        use kentos_sheet::template::{AssetWithBytes, base64_encode, sha256_hex};
        use kentos_sheet::units::RectUm;
        let mut t = kentos_sheet::template::system_template("sys:genel-a4-dikey")
            .expect("a system template")
            .clone();
        // A red picture, and a green square drawn after it over its middle.
        let mut png = Vec::new();
        {
            let mut e = png::Encoder::new(&mut png, 8, 8);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            let mut w = e.write_header().unwrap();
            w.write_image_data(&[220, 20, 20, 255].repeat(64)).unwrap();
        }
        let sha = sha256_hex(&png);
        t.assets.push(AssetWithBytes {
            meta: AssetMeta {
                sha256: sha.clone(),
                kind: AssetKind::Png,
                name: "kirmizi.png".into(),
                width: 8,
                height: 8,
                bytes: png.len() as u32,
                dpi: None,
            },
            data: base64_encode(&png),
        });
        t.sheet.items.push(Item::new(
            "resim",
            "Resim",
            RectUm::new(20_000, 20_000, 120_000, 120_000),
            ItemKind::Picture(PictureItem {
                asset: Some(sha),
                fit: PictureFit::Contain,
                clip: true,
            }),
        ));
        let mut square = default_kind("shape").expect("a shape");
        if let ItemKind::Shape(s) = &mut square {
            s.fill = Some("#10a040".into());
            s.stroke = None;
        }
        t.sheet.items.push(Item::new(
            "kare",
            "Kare",
            RectUm::new(60_000, 60_000, 40_000, 40_000),
            square,
        ));
        let d = Designer::new(crate::designer::Context::default());
        let mut g = Gallery::default();
        g.thumbs.insert("t".into(), d.thumb_of(&t, "t".into()));
        let maps = crate::painter::NoMaps;
        let painter = crate::painter::lend(&maps);
        let (w, h) = (200.0, 280.0);
        let image = kentos_ui::snapshot::Snapshot::software(Size::new(w, h))
            .expect("a renderer")
            .render(thumb(&g, "t", &painter, w, h), &Theme::Light);
        // The paper fitted as the card fits it: A4 upright in 188 × 268 pixels.
        let k = (188.0 / 210_000.0f64).min(268.0 / 297_000.0);
        let at = |x_um: f64, y_um: f64| {
            let x = ((f64::from(w) - 210_000.0 * k) / 2.0 + x_um * k) as usize;
            let y = ((f64::from(h) - 297_000.0 * k) / 2.0 + y_um * k) as usize;
            let i = (y * w as usize + x) * 4;
            [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2]]
        };
        let (square_px, picture_px) = (at(80_000.0, 80_000.0), at(30_000.0, 30_000.0));
        assert!(
            square_px[1] > 120 && square_px[0] < 80,
            "the square over the picture: {square_px:?}"
        );
        assert!(
            picture_px[0] > 180 && picture_px[1] < 80,
            "the picture where nothing is over it: {picture_px:?}"
        );
        // Why: the same list painted in one canvas (the card's picture of before) puts the
        // picture over the square.
        struct One<'a>(&'a Thumb);
        impl canvas::Program<Message> for One<'_> {
            type State = ();
            fn draw(
                &self,
                _: &(),
                renderer: &Renderer,
                _: &Theme,
                bounds: Rectangle,
                _: mouse::Cursor,
            ) -> Vec<canvas::Geometry> {
                let mut f = canvas::Frame::new(renderer, bounds.size());
                let list = self.0.list.as_ref().unwrap();
                let (pw, ph) = (f64::from(list.size.width), f64::from(list.size.height));
                let k = ((f64::from(bounds.width) - 12.0) / pw)
                    .min((f64::from(bounds.height) - 12.0) / ph);
                let origin = Point::new(
                    (bounds.width - (pw * k) as f32) / 2.0,
                    (bounds.height - (ph * k) as f32) / 2.0,
                );
                let pictures = |sha: &str| self.0.pictures.get(sha).cloned().flatten();
                paint::paint_all(
                    &mut f,
                    list,
                    &Xf::new(k, origin),
                    &pictures,
                    Maps::Skip,
                    Options::SCREEN,
                );
                vec![f.into_geometry()]
            }
        }
        let one: Element<'_, Message> = canvas(One(&g.thumbs["t"])).width(w).height(h).into();
        let image = kentos_ui::snapshot::Snapshot::software(Size::new(w, h))
            .expect("a renderer")
            .render(one, &Theme::Light);
        let x = ((f64::from(w) - 210_000.0 * k) / 2.0 + 80_000.0 * k) as usize;
        let y = ((f64::from(h) - 297_000.0 * k) / 2.0 + 80_000.0 * k) as usize;
        let i = (y * w as usize + x) * 4;
        assert!(
            image.rgba[i] > 180,
            "one canvas: the picture over the square"
        );
    }

    #[test]
    fn categories_are_written_with_a_turkish_capital() {
        assert_eq!(category_label("kadastro"), "Kadastro");
        assert_eq!(category_label("imar"), "İmar");
        assert_eq!(category_label(""), "");
    }

    #[test]
    fn a_paper_s_size_follows_its_orientation() {
        let c = PaperChoice {
            paper: Paper::A3,
            orientation: Orientation::Landscape,
        };
        assert_eq!(paper_mm(&c), Some((420.0, 297.0)));
        assert_eq!(paper_text(&c), "A3 yatay");
    }

    /// The web's size in a large window; in a laptop's, inside it with a
    /// margin, the cards two to a row.
    #[test]
    fn the_gallery_fits_the_window() {
        let large = Fitted::of(Size::new(1440.0, 900.0));
        assert_eq!((large.width, large.body, large.columns), (1240.0, 580.0, 3));
        let laptop = Fitted::of(Size::new(1100.0, 720.0));
        assert!(typography::scaled(laptop.width) <= 1100.0 - 2.0 * Fitted::MARGIN + 1.0);
        assert!(laptop.body + typography::scaled(Fitted::CHROME) + 2.0 * Fitted::MARGIN <= 720.0);
        assert!(laptop.body > 400.0);
        assert_eq!(laptop.columns, 2);
        let low = Fitted::of(Size::new(1100.0, 400.0));
        assert_eq!(low.body, 260.0, "never lower than a row of cards");
    }
}
