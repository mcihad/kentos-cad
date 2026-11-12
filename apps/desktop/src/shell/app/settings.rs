//! Settings put in use (docs/adr/0023): the theme, labels, graphics and device, the overlap and session
//! choices, and a drawing put on screen.

use super::*;

impl App {
    /// Puts the settings in use (docs/adr/0023): the tool session's drafting
    /// aids and value field, the theme; the drawing area reads `graphics()`
    /// each frame. Called after every change.
    pub(crate) fn apply_settings(&mut self) {
        let s = &self.settings;
        self.draft = Draft {
            ortho: s.bool("drafting.ortho"),
            right_angle: s.bool("drafting.rightAngle"),
            polar: s
                .bool("drafting.polar")
                .then(|| s.number("drafting.polarIncrement")),
            snap_aperture: s.number("drafting.snapAperture"),
            snap: s.bool("drafting.snap"),
            snap_kinds: snap_kinds(|key| s.bool(key)),
            snap_grid: [s.number("snap.gridEast"), s.number("snap.gridNorth")],
            snap_self: s.bool("snap.self"),
            snap_scale: [s.number("snap.scaleMin"), s.number("snap.scaleMax")],
            pick_aperture: s.number("drafting.pickAperture"),
            tracking: s.bool("drafting.tracking"),
            topology: s.bool("drafting.topology"),
            topology_points: s.bool("drafting.topologyPoints"),
            topology_stop_on_locked: s.bool("drafting.topologyStopOnLocked"),
            overlap: kentos_interaction::Overlap::parse(&s.text("drafting.overlap")),
            // The session's, not settings: kept through a settings change.
            color: self.draft.color,
            line_weight: self.draft.line_weight,
            line_type: self.draft.line_type,
            ucs: self.draft.ucs,
            geographic: kentos_interaction::second::Notation::parse(&s.text("display.geographic")),
            // Seçim süzgeci's kinds are the session's (docs/adr/0187 §5).
            select_kinds: s.bool("drafting.selectFilter").then_some(self.select_kinds),
            unplaced_labels: s.bool("graphics.unplacedLabels"),
            // The program's, not a setting: a module's program leaves out what is not its work (docs/adr/0802).
            offers: crate::module_surfaces::tool_offers_in(self.program),
        };
        self.cursor_input = s.bool("drafting.cursorInput");
        self.command_bar = s.bool("drafting.commandBar");
        self.hover_info = s.bool("drafting.hoverInfo");
        self.crosshair = crate::marks::CrosshairSize::parse(&s.text("appearance.crosshair"));
        self.viewport.grid_shown = s.bool("drafting.grid");
        // Nokta adları and Nokta kodları: the label engine's texts of points (docs/adr/1540).
        let (names, codes) = (s.bool("graphics.pointNames"), s.bool("graphics.pointCodes"));
        self.spatial
            .set_point_texts(self.document.as_ref().map(|d| &d.model), names, codes);
        self.apply_appearance();
    }

    /// What the drawing area draws with: the effective sample count and pixel ratio.
    /// Görünüm kipleri's colour mode (`graphics.colorMode`, docs/adr/0195 §1).
    pub fn color_mode(&self) -> kentos_native_style::color::ColorMode {
        kentos_native_style::color::ColorMode::from_key(&self.settings.text("graphics.colorMode"))
    }

    /// What the label engine is asked besides the labels (docs/adr/0212 §4):
    /// Yerleşmeyen etiketleri göster, Etiketi gizle's Göster, Sabit etiketleri
    /// vurgula; the drawing area's labels are kept for the tools' hit test.
    pub fn label_engine(&self) -> crate::labels::Engine {
        crate::labels::Engine {
            options: kentos_geometry_core::store::placing::PlaceOptions {
                unplaced: self.settings.bool("graphics.unplacedLabels"),
                hidden: self.session.shows_hidden_labels(),
            },
            pinned: self.settings.bool("graphics.pinnedLabels"),
            keep: true,
            moving: self.viewport.moving(),
        }
    }

    /// How the drawing's text is sized on the screen (docs/adr/0205 §5, 0626 §6): the
    /// preference, at the open project's plot scale; Türe göre a CAD project's true sizes.
    pub fn label_size(&self) -> kentos_interaction::spatial::LabelSize {
        let plot = self
            .document
            .as_ref()
            .map_or(1000.0, |d| d.settings().plot_scale);
        kentos_interaction::spatial::LabelSize::for_project(
            &self.settings.text("graphics.annotationSize"),
            plot,
            self.work_mode() == kentos_commands::Workspace::Cad,
        )
    }

    pub fn graphics(&self) -> Graphics {
        Graphics {
            samples: self.settings.number("graphics.msaa").max(1.0) as u32,
            hi_dpi: self.settings.bool("graphics.hiDpi"),
            zoom_scaled: self.settings.bool("graphics.wheelZoomScaled"),
            highlight_width: self
                .settings
                .number("graphics.highlightWidth")
                .clamp(1.0, 5.0) as u32,
        }
    }

    /// Feeds the drawing area's device into the settings (TODOS.md AA-01,
    /// SET-03): the sample counts it takes, or the count it could not make
    /// targets for (AA-02); the effective value follows, the requested one stays.
    pub(crate) fn sync_device(&mut self) {
        let status = self.viewport.status();
        if status.supported.is_empty() {
            return;
        }
        let values = |counts: &mut dyn Iterator<Item = u32>| counts.map(Value::from).collect();
        let listed: Vec<String> = status.supported.iter().map(|&n| samples_label(n)).collect();
        let constraint = match &status.failure {
            Some(f) => SettingConstraint {
                allowed: values(
                    &mut status
                        .supported
                        .iter()
                        .copied()
                        .filter(|&c| c < f.requested),
                ),
                reason: ResolveReason::DeviceFailed,
                detail: format!("{}× hedefi kurulamadı ({}).", f.requested, f.error),
            },
            None => SettingConstraint {
                allowed: values(&mut status.supported.iter().copied()),
                reason: ResolveReason::DeviceUnsupported,
                detail: format!("Desteklenenler: {}.", listed.join(", ")),
            },
        };
        self.settings
            .set_constraint("graphics.msaa", Some(constraint));
        if let Some(f) = &status.failure
            && self.reported_failure != Some((f.requested, f.working))
        {
            self.reported_failure = Some((f.requested, f.working));
            self.warn(format!(
                "Kenar yumuşatma {}× bu aygıtta kurulamadı; son çalışan ayara ({}) dönüldü. Neden: {}",
                f.requested,
                samples_label(f.working),
                f.error
            ));
        }
    }

    /// What opening the settings did, in the command line: the one migration, a recovery.
    pub(super) fn report_settings_open(&mut self) {
        let report = self.settings.report.clone();
        if let Some(m) = &report.migrated {
            self.output(format!(
                "Ayarlar {} dosyasından alındı ({} değer); o dosya olduğu gibi duruyor.",
                m.from, m.moved
            ));
        }
        if let Some(r) = &report.recovered {
            self.warn(format!(
                "Ayar dosyası okunamadı ya da geçersiz değer içeriyordu; eski metni {} olarak saklandı.",
                r.backup.display()
            ));
        }
    }

    /// A drafting aid of this session turned over (F8, F10): said as the classic CAD programs say it.
    /// Çakışma's cell: between Serbest and the last mode that avoided overlap (docs/adr/0162 §1).
    pub(super) fn toggle_overlap(&mut self) {
        use kentos_interaction::Overlap;
        let next = if self.draft.overlap == Overlap::Allow {
            self.overlap_last
        } else {
            Overlap::Allow
        };
        self.choose_overlap(next);
        self.output(match next {
            Overlap::Allow => "Çakışma serbest.".to_owned(),
            Overlap::Layer => "Çakışma önleniyor: kendi katmanında.".to_owned(),
            Overlap::Layers => "Çakışma önleniyor: seçili katmanlarda.".to_owned(),
        });
    }

    /// One of the overlap control's modes: the session's setting; a mode that avoids is the cell's next.
    pub(crate) fn choose_overlap(&mut self, mode: kentos_interaction::Overlap) {
        let _ = self
            .settings
            .choose(&[("drafting.overlap", Value::String(mode.key().to_owned()))]);
        if mode != kentos_interaction::Overlap::Allow {
            self.overlap_last = mode;
        }
        self.apply_settings();
    }

    /// A layer ticked on the Çakışma cell's menu joins Seçili katmanlarda önle's
    /// layers (unticked, it leaves them), and that mode is put on (docs/adr/0162 §1).
    pub(super) fn toggle_overlap_layer(&mut self, id: String) {
        match self.overlap_layers.iter().position(|l| *l == id) {
            Some(i) => {
                self.overlap_layers.remove(i);
            }
            None => self.overlap_layers.push(id),
        }
        self.choose_overlap(kentos_interaction::Overlap::Layers);
    }

    pub(crate) fn toggle_session(&mut self, key: &'static str, name: &str) {
        let on = !self.settings.bool(key);
        let _ = self.settings.choose(&[(key, Value::Bool(on))]);
        self.apply_settings();
        self.output(format!("{name} {}", if on { "açık" } else { "kapalı" }));
        // The running command keeps its points, options and locks; its preview takes the aid at once
        // (docs/adr/0268 §3).
        if self.session.takes_points() {
            self.repoint();
        }
    }

    /// The theme chosen from a command: a preference, kept.
    pub(super) fn choose_theme(&mut self, mode: Mode) {
        self.set_theme(mode);
    }

    /// Puts a drawing on screen: the one there before is left (its cloud
    /// project's copy written whole and let go), and so are the tool's draft,
    /// the selection and the geometry store, which belong to the drawing they
    /// were made on.
    pub(crate) fn show_document(&mut self, doc: Document) {
        self.close_cloud_project();
        self.leave_document_interaction();
        // A point template's names go on in the drawing they were drawn in:
        // another drawing starts them again (docs/adr/0176 §3b).
        self.release_template();
        self.template_names.clear();
        // The annotation tools' heights typed in the last drawing: the new one's annotations take
        // its own project's (docs/adr/0205 §2).
        self.memory.forget_heights();
        self.selected_layer = None;
        // A linked photo's key is its path from the drawing's folder (attachments.rs); Bilgi's card was
        // on an object of the drawing left.
        self.photos.clear();
        self.info_card = None;
        self.networks_reset();
        self.address_reset();
        self.spatial.reload(&doc.model);
        self.viewport.opened(&doc, self.spatial.extent());
        // The views left belong to the drawing they were left on (docs/adr/0141).
        self.view_history.clear();
        self.styles.follow_project(Some(doc.model.styles()));
        self.selection = Selection::new();
        // İstatistik and Grafik start again on another drawing (docs/adr/0227 §8).
        self.charts = Default::default();
        self.followed = Some((doc.session, doc.model.generation()));
        self.xrefs = Default::default();
        self.document = Some(doc);
        // The level and view it opens with are the file's (docs/adr/0720 §1); a tab brought in front gives its own.
        self.take_level_state();
        // A linked file that changed or went is said, never loaded silently (docs/adr/0348 §5, 0349 §2).
        self.xrefs_check_opened();
    }
}
