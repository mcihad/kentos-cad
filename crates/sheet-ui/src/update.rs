//! The sheet mode's messages: each change of the book is an operation of the
//! core (one undo step); views, windows and the pointer change only the
//! mode's own state.

use iced::keyboard::{self, Key, key::Named};
use kentos_sheet::model::{
    Axis, Constraints, Guide, ItemId, Orientation, Owner, Page, Paper, SnapGrid,
};
use kentos_sheet::ops::{
    Align, AlignTo, Distribute, DuplicateSheet, GroupItems, IdRef, ItemFrame, ItemIds, MatchSize,
    MoveGuide, MoveItems, MoveSheet, Op, RemoveGuide, Rename, Reorder, SetFlag, SetFrames,
    SetItemProps, SetOrder, SetPage, SetSnapGrid,
};
use kentos_sheet::paper::paper_size;
use kentos_sheet::preflight::Severity;
use kentos_sheet::units::{RectUm, Um, round_um};
use kentos_ui::widget::rulers;
use kentos_ui::widget::tree_view::Place;
use serde_json::json;

use crate::designer::{Designer, Dialog, Effect};
use crate::message::{ExportKind, FrameField, InspectorTab, Message, Side, Tool};
use crate::view_math::View;

fn mm(v: f64) -> Um {
    round_um(v * 1000.0)
}

impl Designer {
    pub fn update(&mut self, m: Message) -> Vec<Effect> {
        match m {
            Message::Tab(0) | Message::Model => {
                self.open = None;
                self.drag = None;
                self.preview = None;
                self.refresh();
                return vec![Effect::ShowModel];
            }
            Message::Tab(i) => {
                if let Some(s) = self.book.sheets.get(i - 1) {
                    if self.open.as_deref() != Some(s.id.as_str()) {
                        self.open = Some(s.id.clone());
                        self.selection.clear();
                        self.view = View::default();
                        self.refresh();
                    }
                    return vec![Effect::ShowSheet];
                }
            }
            Message::CloseTab(i) => {
                if let Some(s) = i.checked_sub(1).and_then(|i| self.book.sheets.get(i)) {
                    let (id, name) = (s.id.clone(), s.name.clone());
                    if self.commit(vec![Op::RemoveSheet(IdRef { id })]) {
                        return vec![
                            if self.open.is_some() {
                                Effect::ShowSheet
                            } else {
                                Effect::ShowModel
                            },
                            Effect::Notice(format!(
                                "“{name}” paftası silindi; Geri al ile geri gelir."
                            )),
                        ];
                    }
                }
            }
            Message::MoveTab(from, to) => {
                if let (Some(f), Some(t)) = (from.checked_sub(1), to.checked_sub(1))
                    && let Some(s) = self.book.sheets.get(f)
                {
                    let id = s.id.clone();
                    self.commit(vec![Op::MoveSheet(MoveSheet { id, to: t as u32 })]);
                }
            }
            // Yeni pafta: the mode's template, its questions asked first (the web's `newSheet`).
            Message::NewSheet => {
                let id = self.profile.default_template.clone();
                if let Some(t) = kentos_sheet::template::system_template(&id).cloned() {
                    let label = format!("Şablondan pafta: {}", t.meta.name);
                    return self.ask_then_use(&t, None, label);
                }
            }
            Message::DuplicateSheet(id) => {
                if let Some(s) = self.book.sheet(&id) {
                    let n = s.items.len();
                    let name = format!("{} (kopya)", s.name);
                    let new_id = self.new_id("pafta");
                    let item_ids = (0..n).map(|_| self.new_id("o")).collect();
                    if self.commit(vec![Op::DuplicateSheet(DuplicateSheet {
                        id,
                        new_id: new_id.clone(),
                        name,
                        item_ids,
                        index: None,
                    })]) {
                        self.open = Some(new_id);
                        self.selection.clear();
                        self.refresh();
                    }
                }
            }
            Message::RemoveSheet(id) => {
                self.commit(vec![Op::RemoveSheet(IdRef { id })]);
                if self.open.is_none() {
                    return vec![Effect::ShowModel];
                }
            }
            Message::RenameSheet(id, name) => {
                let name = name.trim().to_owned();
                if !name.is_empty() {
                    self.commit(vec![Op::RenameSheet(Rename { id, name })]);
                }
            }

            Message::Tool(t) => {
                self.tool = t;
                self.drag = None;
                self.preview = None;
            }
            Message::Stage(e) => return self.stage_event(e),
            Message::Space(on) => self.space = on,
            Message::Guide(e) => self.guide_event(e),
            // A new view draws the paper again (its layers are cached at the zoom they were drawn
            // at, as a wheel's zoom on the stage clears them).
            Message::ZoomPage => {
                self.view = View::default();
                self.reset_caches();
            }
            Message::ZoomReal => {
                if let Some(page) = self.open_sheet().map(|s| s.page.size) {
                    self.view = self.view.real(self.stage, page);
                    self.reset_caches();
                }
            }
            Message::ZoomSelection => {
                if let Some(b) = self.chosen_bounds() {
                    self.view = View::around(self.stage, b, 72.0);
                    self.reset_caches();
                }
            }

            Message::Select(ids) => self.select(ids),
            Message::TreeClick(id, add) => {
                if add {
                    if let Some(i) = self.selection.iter().position(|x| *x == id) {
                        self.selection.remove(i);
                    } else {
                        self.selection.push(id);
                    }
                    self.after_select();
                } else {
                    self.select(vec![id]);
                }
            }
            Message::TreeExpand(id) => {
                if !self.expanded.remove(&id) {
                    self.expanded.insert(id);
                }
            }
            Message::TreeMove(from, to, place) => self.tree_move(from, to, place),
            Message::Hidden(id, on) => {
                self.commit(vec![Op::Hide(SetFlag {
                    ids: vec![id],
                    value: on,
                })]);
            }
            Message::Locked(id, on) => {
                self.commit(vec![Op::Lock(SetFlag {
                    ids: vec![id],
                    value: on,
                })]);
            }
            Message::RenameItem(id, name) => {
                let name = name.trim().to_owned();
                if !name.is_empty() {
                    self.commit(vec![Op::RenameItem(Rename { id, name })]);
                }
            }
            Message::Align(edge) => {
                let to = if self.selection.len() > 1 {
                    AlignTo::Selection
                } else {
                    AlignTo::Margins
                };
                self.commit(vec![Op::Align(Align {
                    ids: self.selection.clone(),
                    edge,
                    to,
                    key: None,
                })]);
            }
            Message::AlignTo(to) => {
                // The edge of the last alignment is kept by the menu; alone, the chosen items' centres.
                self.commit(vec![Op::Align(Align {
                    ids: self.selection.clone(),
                    edge: kentos_sheet::ops::AlignEdge::Center,
                    to,
                    key: None,
                })]);
            }
            Message::Distribute(axis, mode) => {
                self.commit(vec![Op::Distribute(Distribute {
                    ids: self.selection.clone(),
                    axis,
                    mode,
                })]);
            }
            Message::MatchSize(dimension) => {
                self.commit(vec![Op::MatchSize(MatchSize {
                    ids: self.selection.clone(),
                    dimension,
                    key: None,
                })]);
            }
            Message::Order(to) => {
                self.commit(vec![Op::Reorder(Reorder {
                    ids: self.selection.clone(),
                    to,
                })]);
            }
            Message::Group => self.group(),
            Message::Ungroup => {
                let groups: Vec<ItemId> = self
                    .chosen()
                    .iter()
                    .filter(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Group(_)))
                    .map(|i| i.id.clone())
                    .collect();
                if !groups.is_empty() {
                    let children: Vec<ItemId> = self
                        .items()
                        .iter()
                        .filter(|i| i.group.as_ref().is_some_and(|g| groups.contains(g)))
                        .map(|i| i.id.clone())
                        .collect();
                    if self.commit(
                        groups
                            .into_iter()
                            .map(|id| Op::Ungroup(IdRef { id }))
                            .collect(),
                    ) {
                        self.select(children);
                    }
                }
            }
            Message::Duplicate => self.duplicate(),
            Message::Delete => {
                let ids: Vec<ItemId> = self
                    .chosen()
                    .iter()
                    .filter(|i| !i.locked)
                    .map(|i| i.id.clone())
                    .collect();
                if !ids.is_empty() && self.commit(vec![Op::RemoveItems(ItemIds { ids })]) {
                    self.selection.clear();
                    self.after_select();
                }
            }
            Message::Nudge(dx, dy) => {
                let ids: Vec<ItemId> = self
                    .chosen()
                    .iter()
                    .filter(|i| !i.locked)
                    .map(|i| i.id.clone())
                    .collect();
                if !ids.is_empty() {
                    self.commit(vec![Op::MoveItems(MoveItems {
                        ids,
                        delta: [dx, dy],
                    })]);
                }
            }
            Message::Undo => {
                self.undo_step();
            }
            Message::Redo => {
                self.redo_step();
            }

            Message::InspectorTab(t) => self.inspector_tab = t,
            Message::Section(s) => {
                if !self.closed_sections.remove(s) {
                    self.closed_sections.insert(s);
                }
            }
            Message::Frame(field, v) => self.set_frame(field, v),
            Message::FrameText(field, text) => {
                if let Some(v) = kentos_ui::widget::number::evaluate(&text, &[])
                    .ok()
                    .filter(|v| v.is_finite())
                {
                    self.set_frame(field, v);
                }
            }
            Message::ConstraintH(h) => self.set_constraints(|c| c.h = h),
            Message::ConstraintV(v) => self.set_constraints(|c| c.v = v),
            Message::ConstraintBox(b) => self.set_constraints(|c| c.relative_to = b),
            Message::Inspector(e) => self.inspector_event(e),
            Message::GestureStart => self.gesture = Some(crate::designer::Gesture::default()),
            Message::GestureEnd => self.gesture = None,
            Message::Binding(b) => return self.binding_update(b),
            Message::Fill(fill) => self.patch_chosen(json!({ "fill": fill })),
            Message::Opacity(v) => {
                self.patch_chosen(json!({ "opacity": v.round().clamp(0.0, 100.0) as u8 }))
            }
            Message::Padding(v) => self.patch_chosen(json!({ "padding": mm(v.max(0.0)) })),
            Message::Printable(on) => self.patch_chosen(json!({ "printable": on })),

            Message::Paper(p) => self.set_paper(Some(p), None),
            Message::Orientation(o) => self.set_paper(None, Some(o)),
            Message::Margin(side, v) => {
                if let Some(s) = self.open_sheet() {
                    let mut page = s.page.clone();
                    let v = mm(v.max(0.0));
                    match side {
                        Side::Top => page.margins.top = v,
                        Side::Right => page.margins.right = v,
                        Side::Bottom => page.margins.bottom = v,
                        Side::Left => page.margins.left = v,
                    }
                    let owner = Owner::sheet(&s.id);
                    self.commit(vec![Op::SetPage(SetPage {
                        owner,
                        page,
                        relayout: true,
                    })]);
                }
            }
            Message::SnapToGrid(on) => self.set_grid(|g| g.enabled = on),
            Message::GridSpacing(v) => {
                if v > 0.0 {
                    self.set_grid(|g| g.spacing = mm(v));
                }
            }

            Message::Fix(f, x) => return self.apply_fix(f, x),
            Message::Reveal(id) => {
                self.select(vec![id]);
                self.inspector_tab = InspectorTab::Item;
            }

            Message::Gallery(g) => return self.gallery_update(g),
            Message::Share(m) => return self.share_update(m),
            Message::Publish(m) => return self.publish_update(m),
            Message::North(m) => return self.north_update(m),
            Message::Library(e) => return self.library_event(e),
            Message::SaveTemplate(s) => return self.save_template_update(s),
            Message::Variables(v) => return self.variables_update(v),
            Message::Questions(q) => return self.questions_update(q),
            Message::Export(kind) => {
                let errors = self
                    .findings
                    .iter()
                    .filter(|f| f.severity == Severity::Error)
                    .count();
                self.dialog = Some(Dialog::Export {
                    kind,
                    dpi: self.export_dpi,
                    errors,
                    print: false,
                });
                if kind == ExportKind::Pdf {
                    self.open_pdf(false);
                } else if errors == 0 && kind != ExportKind::Png {
                    self.dialog = None;
                    return vec![self.ask_path(kind)];
                }
            }
            Message::Print => self.open_pdf(true),
            Message::Pdf(m) => return self.pdf_update(m),
            Message::ChoosePicture => {
                if self.picture_frames().is_empty() {
                    return Vec::new();
                }
                return vec![Effect::AskPicturePath];
            }
            Message::PictureFile(name, bytes) => return self.picture_file(&name, bytes),
            Message::ImportKpafta => return vec![Effect::AskImportPath],
            Message::ImportText(label, text) => {
                let have = self.book.sheets.len();
                match crate::export::read_file(&text) {
                    Err(e) => self.error = Some(format!("“{label}” okunamadı: {e}")),
                    Ok((book, _)) if book.sheets.is_empty() => {
                        self.error = Some(format!("“{label}” dosyasında pafta yok."))
                    }
                    Ok((book, _)) if have > 0 => {
                        self.dialog = Some(Dialog::Import {
                            label,
                            text,
                            sheets: book.sheets.len(),
                            have,
                        });
                    }
                    Ok(_) => return self.import_kpafta(&label, &text, false),
                }
            }
            Message::ImportChoose(replace) => {
                if let Some(Dialog::Import { label, text, .. }) = self.dialog.take() {
                    return self.import_kpafta(&label, &text, replace);
                }
            }
            Message::ExportDpi(dpi) => {
                self.export_dpi = dpi;
                if let Some(Dialog::Export { dpi: d, .. }) = &mut self.dialog {
                    *d = dpi;
                }
            }
            Message::ExportAnyway => {
                if let Some(Dialog::Export { kind, .. }) = self.dialog.take() {
                    return vec![self.ask_path(kind)];
                }
            }
            Message::CloseDialog => self.dialog = None,
        }
        Vec::new()
    }

    /// The export window for a PDF: the sheet in front chosen first, GeoPDF on where it can be.
    fn open_pdf(&mut self, print: bool) {
        if let Some(id) = &self.open
            && self.pdf.chosen.is_empty()
        {
            self.pdf.chosen.insert(id.clone());
        }
        let errors = self.pdf_errors();
        self.dialog = Some(Dialog::Export {
            kind: ExportKind::Pdf,
            dpi: self.export_dpi,
            errors,
            print,
        });
    }

    /// The preflight's errors on every sheet the PDF writes.
    pub(crate) fn pdf_errors(&self) -> usize {
        let inputs = self.inputs(kentos_sheet::display::RenderMode::Export);
        self.pdf_sheets()
            .iter()
            .flat_map(|id| {
                kentos_sheet::preflight::preflight(&self.book, id, &inputs).unwrap_or_default()
            })
            .filter(|f| f.severity == Severity::Error)
            .count()
    }

    fn pdf_update(&mut self, m: crate::message::PdfMessage) -> Vec<Effect> {
        use crate::message::PdfMessage;
        match m {
            PdfMessage::Scope(scope) => self.pdf.scope = scope,
            PdfMessage::Sheet(id, on) => {
                if on {
                    self.pdf.chosen.insert(id);
                } else {
                    self.pdf.chosen.remove(&id);
                }
            }
            PdfMessage::Geo(on) => self.pdf.geo = on,
            PdfMessage::Layers(on) => self.pdf.layers = on,
            PdfMessage::Save => {
                self.dialog = None;
                return vec![self.ask_path(ExportKind::Pdf)];
            }
            PdfMessage::Print => {
                self.dialog = None;
                return vec![Effect::Print];
            }
        }
        let errors = self.pdf_errors();
        if let Some(Dialog::Export { errors: e, .. }) = &mut self.dialog {
            *e = errors;
        }
        Vec::new()
    }

    fn ask_path(&self, kind: ExportKind) -> Effect {
        Effect::AskExportPath {
            kind,
            suggested: self.ask_path_name(kind),
        }
    }

    /// The file name an export suggests: the sheet's export name, else its name.
    /// The file name an export offers (the web's `fileName`, `pdfName`, `exportName`): a sheet's
    /// name for SVG and PNG; the project's for a `.kpafta`; for a PDF the sheet's export name
    /// (`[% … %]` text, by default `[% @pafta_adi %]`, as the core writes it), or for several
    /// sheets “<çizim> paftaları”.
    pub(crate) fn ask_path_name(&self, kind: ExportKind) -> String {
        let doc = self.ctx.project.name.trim();
        let base = match kind {
            ExportKind::Kpafta if doc.is_empty() => "paftalar".to_owned(),
            ExportKind::Kpafta => doc.to_owned(),
            ExportKind::Pdf => match self.pdf_sheets().as_slice() {
                [] => "paftalar".to_owned(),
                [one] => self.export_name(one),
                _ => format!("{} paftaları", strip_extension(doc).unwrap_or("çizim")),
            },
            ExportKind::Svg | ExportKind::Png => self
                .open_sheet()
                .map_or_else(|| "pafta".to_owned(), |s| s.name.clone()),
        };
        file_name(&base, kind.extension())
    }

    /// A sheet's export name as the core writes it (`display::export_name`); its name when the
    /// core cannot read the book.
    pub(crate) fn export_name(&self, id: &str) -> String {
        kentos_sheet::display::export_name(&self.book, id, &self.export_inputs()).unwrap_or_else(
            |_| {
                self.book
                    .sheet(id)
                    .map_or_else(|| "pafta".to_owned(), |s| s.name.clone())
            },
        )
    }

    // ── Choosing ──

    pub(crate) fn select(&mut self, ids: Vec<ItemId>) {
        let mut seen = std::collections::BTreeSet::new();
        self.selection = ids
            .into_iter()
            .filter(|id| seen.insert(id.clone()) && self.book.find_item(id).is_some())
            .collect();
        self.after_select();
    }

    pub(crate) fn after_select(&mut self) {
        // A new subject for the inspector: its typed drafts start again.
        let subject = self.selection.iter().fold(0u64, |h, id| {
            id.bytes().fold(h.wrapping_mul(31), |h, b| {
                h.wrapping_mul(131).wrapping_add(u64::from(b))
            })
        });
        self.inspector
            .inspect((!self.selection.is_empty()).then_some(subject));
        crate::inspect::refresh_fields(self);
        if !self.selection.is_empty() && self.inspector_tab == InspectorTab::Page {
            self.inspector_tab = InspectorTab::Item;
        }
    }

    // ── Changing the chosen items ──

    fn patch_chosen(&mut self, patch: serde_json::Value) {
        let ops: Vec<Op> = self
            .selection
            .iter()
            .map(|id| {
                Op::SetItemProps(SetItemProps {
                    id: id.clone(),
                    patch: patch.clone(),
                })
            })
            .collect();
        self.commit(ops);
    }

    fn set_frame(&mut self, field: FrameField, v: f64) {
        let frames: Vec<ItemFrame> = self
            .chosen()
            .iter()
            .filter(|i| !i.locked)
            .map(|i| {
                let mut f = i.frame;
                let mut rotation = i.rotation;
                match field {
                    FrameField::Left => f.left = mm(v),
                    FrameField::Top => f.top = mm(v),
                    FrameField::Width => f.width = mm(v).max(1),
                    FrameField::Height => f.height = mm(v).max(1),
                    FrameField::Rotation => rotation = round_um(v * 1000.0).rem_euclid(360_000),
                }
                ItemFrame {
                    id: i.id.clone(),
                    frame: f,
                    rotation,
                }
            })
            .collect();
        if !frames.is_empty() {
            self.commit(vec![Op::SetFrames(SetFrames { frames })]);
        }
    }

    fn set_constraints(&mut self, change: impl Fn(&mut Constraints)) {
        let ops: Vec<Op> = self
            .chosen()
            .iter()
            .filter_map(|i| {
                let mut c = i.constraints;
                change(&mut c);
                serde_json::to_value(c).ok().map(|c| {
                    Op::SetItemProps(SetItemProps {
                        id: i.id.clone(),
                        patch: json!({ "constraints": c }),
                    })
                })
            })
            .collect();
        self.commit(ops);
    }

    fn inspector_event(&mut self, e: kentos_ui::widget::inspector::Event) {
        use kentos_ui::widget::inspector::{Action, Event};
        // A field's name dragged: its changes are one undo step until it is let go.
        match &e {
            Event::ScrubStart(..) => self.gesture = Some(crate::designer::Gesture::default()),
            Event::ScrubEnd => self.gesture = None,
            _ => {}
        }
        let Some(action) = self.inspector.update(e) else {
            return;
        };
        if let Action::Change { id, value } = action
            && let Some(kf) = self.kind_fields.get(id)
        {
            let maps = crate::inspect::maps_of(self.items());
            if let Some(patch) = crate::inspect::write(&kf.prop, &value, &maps) {
                self.patch_chosen(patch);
            }
        }
    }

    /// The chosen north arrows' declination (the inspector's part; the web's `northSection`).
    pub(crate) fn north_update(&mut self, m: crate::message::NorthMessage) -> Vec<Effect> {
        use crate::message::NorthMessage;
        use kentos_sheet::kinds::ItemKind;
        // The model's value to the minute, as the paper writes it, where the core gave one.
        let model = self
            .north_info
            .as_ref()
            .and_then(|(_, i)| i.as_ref())
            .and_then(crate::inspect::model_declination)
            .map(|d| ((d * 60.0).round() / 60.0 * 1000.0).round() as i64);
        let ops: Vec<Op> = self
            .chosen()
            .iter()
            .filter_map(|i| match &i.kind {
                ItemKind::NorthArrow(k) => Some((i.id.clone(), k.declination)),
                _ => None,
            })
            .map(|(id, typed)| {
                let kind = match &m {
                    // Turned on with nothing typed yet: it starts from the model's value.
                    NorthMessage::Hand(true) => match model.filter(|_| typed == 0) {
                        Some(d) => json!({ "type": "northArrow", "declinationHand": true, "declination": d }),
                        None => json!({ "type": "northArrow", "declinationHand": true }),
                    },
                    NorthMessage::Hand(false) => {
                        json!({ "type": "northArrow", "declinationHand": false })
                    }
                    NorthMessage::Declination(deg) => {
                        json!({ "type": "northArrow", "declination": (deg * 1000.0).round() as i64 })
                    }
                    NorthMessage::Year(y) => {
                        json!({ "type": "northArrow", "declinationYear": y.round() as i64 })
                    }
                };
                Op::SetItemProps(SetItemProps {
                    id,
                    patch: json!({ "kind": kind }),
                })
            })
            .collect();
        if !ops.is_empty() {
            self.commit(ops);
        }
        Vec::new()
    }

    fn group(&mut self) {
        let ids: Vec<ItemId> = self.selection.clone();
        if ids.len() < 2 {
            return;
        }
        let id = self.new_id("grup");
        let n = self
            .items()
            .iter()
            .filter(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Group(_)))
            .count()
            + 1;
        let frame = self.chosen_bounds().unwrap_or(RectUm::new(0, 0, 1, 1));
        let group = kentos_sheet::model::Item::new(
            &id,
            &format!("Grup {n}"),
            frame,
            kentos_sheet::kinds::ItemKind::Group(Default::default()),
        );
        if self.commit(vec![Op::Group(Box::new(GroupItems {
            ids,
            group,
            index: None,
        }))]) {
            self.select(vec![id]);
        }
    }

    fn duplicate(&mut self) {
        let chosen: Vec<kentos_sheet::model::Item> = self.chosen().into_iter().cloned().collect();
        let Some(owner) = self.owner() else {
            return;
        };
        if chosen.is_empty() {
            return;
        }
        // Children of a chosen group go with it, with their own new ids.
        let mut all: Vec<kentos_sheet::model::Item> = Vec::new();
        for i in self.items() {
            let in_group = i
                .group
                .as_ref()
                .is_some_and(|g| chosen.iter().any(|c| &c.id == g));
            if chosen.iter().any(|c| c.id == i.id) || in_group {
                all.push(i.clone());
            }
        }
        let mut map = std::collections::BTreeMap::new();
        for i in &all {
            let id = self.new_id("o");
            map.insert(i.id.clone(), id);
        }
        let offset = 5_000;
        let copies: Vec<kentos_sheet::model::Item> = all
            .into_iter()
            .map(|mut i| {
                i.id = map[&i.id].clone();
                i.name = format!("{} (kopya)", i.name);
                i.group = i.group.and_then(|g| map.get(&g).cloned());
                i.frame = i.frame.translate(i64::from(offset), i64::from(offset));
                i.locked = false;
                i
            })
            .collect();
        let tops: Vec<ItemId> = copies
            .iter()
            .filter(|i| i.group.is_none() || !map.values().any(|v| Some(v) == i.group.as_ref()))
            .map(|i| i.id.clone())
            .collect();
        if self.commit(vec![Op::AddItems(kentos_sheet::ops::AddItems {
            to: owner,
            items: copies,
            index: None,
        })]) {
            self.select(tops);
        }
    }

    fn tree_move(&mut self, from: usize, to: usize, place: Place) {
        let rows = self.tree_rows();
        let (Some(a), Some(b)) = (rows.get(from).cloned(), rows.get(to).cloned()) else {
            return;
        };
        let Some(owner) = self.owner() else {
            return;
        };
        let items = self.items();
        let group_of = |id: &str| {
            items
                .iter()
                .find(|i| i.id == id)
                .and_then(|i| i.group.clone())
        };
        if place == Place::Into || group_of(&a) != group_of(&b) || a == b {
            self.error = Some("Öğe yalnız aynı grubun öğeleri arasında taşınır.".into());
            return;
        }
        // The tree shows the top first: “before” it is above it, later in the drawing order.
        let mut order: Vec<ItemId> = items.iter().map(|i| i.id.clone()).collect();
        let moving: Vec<ItemId> = items
            .iter()
            .filter(|i| i.id == a || i.group.as_deref() == Some(a.as_str()))
            .map(|i| i.id.clone())
            .collect();
        order.retain(|id| !moving.contains(id));
        let Some(at) = order.iter().position(|id| *id == b) else {
            return;
        };
        let at = match place {
            Place::Before => at + 1,
            _ => at,
        };
        // A group's own item stands above its children: the moved block keeps its order.
        for (k, id) in moving.into_iter().enumerate() {
            order.insert((at + k).min(order.len()), id);
        }
        self.commit(vec![Op::SetOrder(SetOrder { owner, order })]);
    }

    /// The tree's rows, top first: each item, a group's children under it.
    pub(crate) fn tree_rows(&self) -> Vec<ItemId> {
        let items = self.items();
        let mut out = Vec::new();
        for i in items.iter().rev().filter(|i| i.group.is_none()) {
            out.push(i.id.clone());
            self.push_children(&i.id, &mut out);
        }
        out
    }

    fn push_children(&self, group: &str, out: &mut Vec<ItemId>) {
        for c in self
            .items()
            .iter()
            .rev()
            .filter(|i| i.group.as_deref() == Some(group))
        {
            out.push(c.id.clone());
            self.push_children(&c.id, out);
        }
    }

    // ── The page ──

    fn set_paper(&mut self, paper: Option<Paper>, orientation: Option<Orientation>) {
        let Some(s) = self.open_sheet() else {
            return;
        };
        let paper = paper.unwrap_or(s.page.paper);
        let orientation = orientation.unwrap_or(s.page.orientation);
        let size = match paper_size(paper, orientation) {
            Some(size) => size,
            // A custom paper keeps its size; turning it swaps its sides.
            None if orientation != s.page.orientation => kentos_sheet::units::SizeUm {
                width: s.page.size.height,
                height: s.page.size.width,
            },
            None => s.page.size,
        };
        let page = Page {
            paper,
            orientation,
            size,
            ..s.page.clone()
        };
        let owner = Owner::sheet(&s.id);
        if self.commit(vec![Op::SetPage(SetPage {
            owner,
            page,
            relayout: true,
        })]) {
            self.view = View::default();
        }
    }

    fn set_grid(&mut self, change: impl Fn(&mut SnapGrid)) {
        if let Some(s) = self.open_sheet() {
            let mut grid = s.snap_grid;
            change(&mut grid);
            let sheet = s.id.clone();
            self.commit(vec![Op::SetSnapGrid(SetSnapGrid { sheet, grid })]);
        }
    }

    fn guide_event(&mut self, e: rulers::Event) {
        let Some(owner) = self.owner() else {
            return;
        };
        let op = match e {
            rulers::Event::Added(g) => {
                let id = self.new_id("k");
                Op::AddGuide(kentos_sheet::ops::AddGuide {
                    owner,
                    guide: Guide {
                        id,
                        axis: match g.orientation {
                            rulers::Orientation::Vertical => Axis::X,
                            rulers::Orientation::Horizontal => Axis::Y,
                        },
                        at: mm(f64::from(g.value)),
                        locked: false,
                    },
                    index: None,
                })
            }
            rulers::Event::Moved(i, v) => {
                let Some(id) = self.guide_ids.get(i).cloned() else {
                    return;
                };
                Op::MoveGuide(MoveGuide {
                    owner,
                    id,
                    at: mm(f64::from(v)),
                })
            }
            rulers::Event::Removed(i) => {
                let Some(id) = self.guide_ids.get(i).cloned() else {
                    return;
                };
                Op::RemoveGuide(RemoveGuide { owner, id })
            }
        };
        self.commit(vec![op]);
    }

    // ── Preflight ──

    fn apply_fix(&mut self, finding: usize, fix: usize) -> Vec<Effect> {
        let Some(f) = self
            .findings
            .get(finding)
            .and_then(|f| f.fixes.get(fix))
            .cloned()
        else {
            return Vec::new();
        };
        if let Some(action) = f.action {
            // The variables are this mode's own window, a map's place is the drawing
            // area's (the host gives its centre); the other actions are the host's.
            match action.as_str() {
                "sheet.variables" => {
                    return self.variables_update(crate::variables::VariablesMessage::Open);
                }
                "map.placeFromView" => {
                    let item = self.findings.get(finding).and_then(|x| x.item.clone());
                    match (item, self.ctx.center) {
                        (Some(id), Some(c)) => {
                            self.commit_as(
                                vec![Op::SetItemProps(SetItemProps {
                                    id,
                                    patch: json!({ "kind": { "view": { "center": { "x": c.x, "y": c.y } } } }),
                                })],
                                Some("Harita merkezi: görünümden"),
                            );
                        }
                        _ => {
                            self.error = Some(
                                "Çizim alanında bir görünüm yok: önce bir çizim açın.".to_owned(),
                            )
                        }
                    }
                    return Vec::new();
                }
                _ => return vec![Effect::HostAction(action)],
            }
        }
        self.commit(f.ops);
        Vec::new()
    }

    /// The sheet mode's keys (design §11), for a key no field took: the host routes them here while a sheet is in front.
    pub fn key(&self, key: &Key, m: keyboard::Modifiers) -> Option<Message> {
        let ctrl = m.command();
        let step = if m.shift() {
            kentos_sheet::snap::NUDGE_SHIFT
        } else if m.alt() {
            kentos_sheet::snap::NUDGE_ALT
        } else {
            kentos_sheet::snap::NUDGE
        };
        Some(match key {
            Key::Named(Named::ArrowLeft) => Message::Nudge(-step, 0),
            Key::Named(Named::ArrowRight) => Message::Nudge(step, 0),
            Key::Named(Named::ArrowUp) => Message::Nudge(0, -step),
            Key::Named(Named::ArrowDown) => Message::Nudge(0, step),
            Key::Named(Named::Delete) | Key::Named(Named::Backspace) => Message::Delete,
            Key::Named(Named::Escape) => {
                if self.dialog.is_some() {
                    Message::CloseDialog
                } else if self.gallery.is_some() {
                    Message::Gallery(crate::gallery::GalleryMessage::Close)
                } else if self.tool != Tool::Select {
                    Message::Tool(Tool::Select)
                } else {
                    Message::Select(Vec::new())
                }
            }
            Key::Character(c) => {
                let c = c.as_str();
                match (ctrl, m.shift(), c) {
                    (true, false, "z") => Message::Undo,
                    (true, true, "z") | (true, false, "y") => Message::Redo,
                    (true, false, "g") => Message::Group,
                    (true, true, "g") => Message::Ungroup,
                    (true, false, "d") => Message::Duplicate,
                    (true, _, "]") => Message::Order(kentos_sheet::ops::ReorderTo::Forward),
                    (true, _, "[") => Message::Order(kentos_sheet::ops::ReorderTo::Backward),
                    (true, _, "0") => Message::ZoomPage,
                    (true, _, "1") => Message::ZoomReal,
                    (true, false, "a") => Message::Select(
                        self.items()
                            .iter()
                            .filter(|i| i.group.is_none() && !i.locked)
                            .map(|i| i.id.clone())
                            .collect(),
                    ),
                    (false, _, "v") => Message::Tool(Tool::Select),
                    (false, _, "h") => Message::Tool(Tool::Hand),
                    (false, _, "m") => self.add_tool("map")?,
                    (false, _, "t") => self.add_tool("text")?,
                    (false, _, "l") => self.add_tool("legend")?,
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// A tool of the profile by its id, if the mode shows it and it is on.
    fn add_tool(&self, id: &str) -> Option<Message> {
        self.profile
            .groups
            .iter()
            .flat_map(|g| g.tools.iter())
            .find(|t| t.id == id && t.state == kentos_sheet::profile::ToolState::Enabled)
            .map(|t| {
                Message::Tool(Tool::Add {
                    tool: t.id.clone(),
                    preset: None,
                })
            })
    }
}

impl Designer {
    /// The chosen picture frames a picture file may be given to (not locked).
    pub(crate) fn picture_frames(&self) -> Vec<ItemId> {
        self.chosen()
            .into_iter()
            .filter(|i| !i.locked && matches!(i.kind, kentos_sheet::kinds::ItemKind::Picture(_)))
            .map(|i| i.id.clone())
            .collect()
    }

    /// A picture file for the chosen picture frames (the web's `choosePicture`): its bytes
    /// kept with the book, the frames given its digest, one undo step “Resim: …”.
    fn picture_file(&mut self, name: &str, bytes: Vec<u8>) -> Vec<Effect> {
        let frames = self.picture_frames();
        if frames.is_empty() {
            return Vec::new();
        }
        let meta = match crate::pictures::asset_of(name, &bytes) {
            Ok(m) => m,
            Err(why) => return vec![Effect::Say(crate::designer::Say::Warn, why)],
        };
        let sha = meta.sha256.clone();
        let label = format!("Resim: {}", meta.name);
        let mut ops = Vec::new();
        if !self.book.assets.iter().any(|a| a.sha256 == sha) {
            ops.push(Op::AddAssets(kentos_sheet::ops::AddAssets {
                assets: vec![meta],
            }));
        }
        for id in frames {
            ops.push(Op::SetItemProps(SetItemProps {
                id,
                patch: json!({ "kind": { "asset": sha } }),
            }));
        }
        self.add_asset_bytes(&sha, bytes);
        self.commit_as(ops, Some(&label));
        Vec::new()
    }
}

/// A file's name from a sheet's (the web's `fileName`): the characters no file system takes
/// become spaces, runs of spaces one; an empty name is “pafta”.
pub(crate) fn file_name(name: &str, ext: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut space = false;
    for c in name.chars() {
        let c = if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            || c.is_whitespace()
        {
            ' '
        } else {
            c
        };
        if c == ' ' {
            if !space && !out.is_empty() {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    let base = out.trim();
    format!("{}.{ext}", if base.is_empty() { "pafta" } else { base })
}

/// A file's name without its extension (“Ada.kcad” → “Ada”); none for an empty name.
fn strip_extension(name: &str) -> Option<&str> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some(match name.rfind('.') {
        Some(i)
            if i > 0
                && name[i + 1..].chars().all(|c| c.is_ascii_alphanumeric())
                && i + 1 < name.len() =>
        {
            &name[..i]
        }
        _ => name,
    })
}
