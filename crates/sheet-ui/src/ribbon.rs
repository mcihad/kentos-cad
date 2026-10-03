//! The contextual Pafta tab of the ribbon (design §11, §11a; the web's
//! `app/sheet/ribbonTab.ts`), shown while a sheet is in front. Its own
//! panels: Pafta (Yeni ▾, Şablondan, Sayfa ayarları, Modele dön), Görünüm
//! (Sayfayı sığdır, Gerçek boyut) and Çıktı (Ön denetim, Dışa aktar ▾,
//! Şablon olarak kaydet). Between them the mode's profile's groups as the
//! core gives them: Araçlar (Seç, El), Ekle (the mode's tools in the mode's
//! names; a tool with several ready looks opens their list) and Düzen
//! (Hizala ▾, Dağıt ▾, Sıra ▾, Grupla, Grubu çöz). A tool the profile hides
//! is not there; one it shows off says why in its tip. The groups are built
//! in the host's message type and own what they show (`'static`), so the
//! host's ribbon takes them beside its own.

use kentos_sheet::model::Axis;
use kentos_sheet::ops::{AlignEdge, AlignTo, DistributeMode, ReorderTo, SizeDimension};
use kentos_sheet::profile::{ToolInfo, ToolState};
use kentos_ui::icon::Icon;
use kentos_ui::widget::Menu;
use kentos_ui::widget::Tip;
use kentos_ui::widget::ribbon::{Button, Group};

use crate::designer::Designer;
use crate::gallery::GalleryMessage;
use crate::icons;
use crate::message::{ExportKind, InspectorTab, Message, Tool};
use crate::save_template::SaveMessage;

/// The tab's label.
pub const TAB: &str = "Pafta";

/// A tool's icon by its id.
pub fn tool_icon(id: &str) -> Icon {
    match id {
        "select" => icons::SELECT,
        "pan" => icons::PAN,
        "map" => icons::MAP,
        "overviewMap" => icons::OVERVIEW,
        "text" => icons::TEXT,
        "legend" => icons::LEGEND,
        "scaleBar" => icons::SCALE_BAR,
        "northArrow" => icons::NORTH,
        "table" | "attributeTable" => icons::TABLE,
        "coordinateList" => icons::COORDINATES,
        "titleBlock" => icons::TITLE_BLOCK,
        "border" => icons::BORDER,
        "picture" => icons::PICTURE,
        "shape" => icons::SHAPE,
        "line" => icons::LINE,
        "grid" => icons::GRID,
        "atlas" => icons::ATLAS,
        "group" => icons::GROUP,
        _ => icons::LAYOUT,
    }
}

/// A ready look's icon in its tool's list: its own picture, or the tool's (the web's `presetIcon`).
pub fn preset_icon(tool: &str, preset: &str) -> Icon {
    match preset {
        "rect" => icons::RECT,
        "rounded" => icons::ROUNDED,
        "ellipse" => icons::ELLIPSE,
        "triangle" => icons::TRIANGLE,
        "polygon" => icons::POLYGON,
        "plain" => icons::LINE_PLAIN,
        "arrow" => icons::ARROW,
        "numeric" => icons::SCALE_NUMERIC,
        "neat" => icons::BORDER_NEAT,
        "revisions" => icons::TABLE_REVISIONS,
        "drawings" => icons::TABLE_DRAWINGS,
        "compass" => icons::COMPASS,
        "viewport" => icons::VIEWPORT,
        "layers" => icons::LEGEND_LAYERS,
        "thematic" => icons::LEGEND_THEMATIC,
        _ => tool_icon(tool),
    }
}

impl Designer {
    fn tool_button<M: Clone + 'static>(
        &self,
        t: &ToolInfo,
        wrap: impl Fn(Message) -> M + Copy + 'static,
    ) -> Option<Button<'static, M>> {
        if t.state == ToolState::Hidden {
            return None;
        }
        let enabled = t.state == ToolState::Enabled;
        let mut tip = Tip::new(t.label.clone());
        if let Some(k) = &t.shortcut {
            tip = Tip::new(format!("{} ({k})", t.label));
        }
        if let Some(reason) = &t.reason {
            tip = tip.body(reason.clone());
        }
        let button = match t.id.as_str() {
            "select" => Button::large(tool_icon("select"), "Seç")
                .active(self.tool == Tool::Select)
                .on_press(wrap(Message::Tool(Tool::Select))),
            "pan" => Button::large(tool_icon("pan"), "El")
                .active(self.tool == Tool::Hand)
                .on_press(wrap(Message::Tool(Tool::Hand))),
            "group" => Button::small(icons::GROUP, "Grupla")
                .on_press_maybe((self.selection.len() > 1).then(|| wrap(Message::Group))),
            // As the web's: the command's own name (a mode's name for it stays in the tooltip).
            "grid" => Button::small(tool_icon(&t.id), "Karelaj"),
            "atlas" => Button::small(tool_icon(&t.id), "Atlas"),
            _ if t.item.is_some() => {
                let active = matches!(&self.tool, Tool::Add { tool, .. } if *tool == t.id);
                let face = if t.shortcut.is_some() {
                    Button::large(tool_icon(&t.id), t.label.clone())
                } else {
                    Button::small(tool_icon(&t.id), t.label.clone())
                };
                let face = face.active(active);
                if t.presets.len() > 1 {
                    let presets = t.presets.clone();
                    let id = t.id.clone();
                    face.on_press_maybe(enabled.then(|| {
                        wrap(Message::Tool(Tool::Add {
                            tool: id.clone(),
                            preset: None,
                        }))
                    }))
                    .menu(move || {
                        presets.iter().fold(Menu::new(), |m, p| {
                            m.item(
                                p.label.clone(),
                                enabled.then(|| {
                                    wrap(Message::Tool(Tool::Add {
                                        tool: id.clone(),
                                        preset: Some(p.id.clone()),
                                    }))
                                }),
                            )
                            .icon(preset_icon(&id, &p.id))
                        })
                    })
                } else {
                    face.on_press_maybe(enabled.then(|| {
                        wrap(Message::Tool(Tool::Add {
                            tool: t.id.clone(),
                            preset: None,
                        }))
                    }))
                }
            }
            _ => return None,
        };
        Some(button.tip(tip))
    }

    /// The Pafta tab's panels, in the host's message type.
    pub fn ribbon_groups<M: Clone + 'static>(
        &self,
        wrap: impl Fn(Message) -> M + Copy + 'static,
    ) -> Vec<Group<'static, M>> {
        let open = self.open.clone();
        let some = !self.selection.is_empty();
        let several = self.selection.len() > 1;
        let mut groups = Vec::new();

        let new_menu = {
            let open = open.clone();
            move || {
                let mut m = Menu::new()
                    .item("Yeni pafta", wrap(Message::NewSheet))
                    .icon(icons::NEW);
                if let Some(id) = &open {
                    m = m
                        .item("Paftayı çoğalt", wrap(Message::DuplicateSheet(id.clone())))
                        .icon(icons::COPY);
                }
                m.separator()
                    .item(".kpafta dosyasından…", wrap(Message::ImportKpafta))
                    .icon(icons::IMPORT_KPAFTA)
            }
        };
        groups.push(
            Group::new("Pafta")
                .icon(icons::LAYOUT)
                .tool(
                    Button::large(icons::NEW, "Yeni")
                        .on_press(wrap(Message::NewSheet))
                        .menu(new_menu)
                        .tip(
                            Tip::new("Yeni pafta")
                                .body("Kipin varsayılan şablonundan yeni bir pafta açar."),
                        ),
                )
                .tool(
                    Button::large(icons::TEMPLATE, "Şablondan")
                        .on_press(wrap(Message::Gallery(GalleryMessage::Open)))
                        .tip(
                            Tip::new("Şablondan…")
                                .body("Pafta şablonları: sistemin, bu cihazın ve paylaşılanlar."),
                        ),
                )
                .tool(
                    Button::small(icons::PAGE, "Sayfa")
                        .on_press(wrap(Message::InspectorTab(InspectorTab::Page)))
                        .tip(Tip::new("Sayfa ayarları").body(
                            "Kâğıt, yön, kenar boşlukları, yerleşim düzeni ve ızgara: denetçinin Sayfa sekmesi.",
                        )),
                )
                .tool(
                    Button::small(icons::VARIABLES, "Değişkenler")
                        .on_press(wrap(Message::Variables(
                            crate::variables::VariablesMessage::Open,
                        )))
                        .tip(
                            Tip::new("Değişkenler")
                                .body("[% @ad %] ile yazılan değerler: paftanın ve projenin."),
                        ),
                )
                .tool(
                    Button::small(icons::MODEL, "Model")
                        .on_press(wrap(Message::Model))
                        .tip(Tip::new("Modele dön").body("Çizim alanı öne gelir; paftalar sekmelerinde kalır.")),
                ),
        );

        for g in &self.profile.groups {
            let mut group = Group::new(g.label.clone()).icon(
                g.tools
                    .iter()
                    .find(|t| t.state != ToolState::Hidden)
                    .map_or(icons::LAYOUT, |t| tool_icon(&t.id)),
            );
            let mut any = false;
            for t in &g.tools {
                if let Some(b) = self.tool_button(t, wrap) {
                    group = group.tool(b);
                    any = true;
                }
            }
            if g.id == "arrange" {
                group = arrange(group, some, several, wrap);
                any = true;
            }
            if any {
                groups.push(group);
            }
        }
        if !self.profile.groups.iter().any(|g| g.id == "arrange") {
            groups.push(
                arrange(
                    Group::new("Düzen").icon(icons::ALIGN_LEFT),
                    some,
                    several,
                    wrap,
                )
                .tool(
                    Button::small(icons::GROUP, "Grupla")
                        .on_press_maybe(several.then(|| wrap(Message::Group))),
                ),
            );
        }

        groups.push(
            Group::new("Görünüm")
                .icon(icons::ZOOM_PAGE)
                .tool(
                    Button::large(icons::ZOOM_PAGE, "Sığdır")
                        .on_press(wrap(Message::ZoomPage))
                        .tip(Tip::new("Sayfayı sığdır (Ctrl+0)")),
                )
                .tool(
                    Button::large(icons::ZOOM_REAL, "Gerçek")
                        .on_press(wrap(Message::ZoomReal))
                        .tip(
                            Tip::new("Gerçek boyut (Ctrl+1)").body(
                                "%100: kâğıdın bir milimetresi ekranda bir milimetre (96 dpi).",
                            ),
                        ),
                )
                .tool(
                    Button::large(icons::ZOOM_SELECTION, "Seçim")
                        .on_press_maybe(
                            (!self.selection.is_empty()).then(|| wrap(Message::ZoomSelection)),
                        )
                        .tip(
                            Tip::new("Seçime yakınlaş")
                                .body("Seçili öğeler pafta alanını doldurur."),
                        ),
                ),
        );

        let export_menu = move || {
            Menu::new()
                .item("PDF olarak…", wrap(Message::Export(ExportKind::Pdf)))
                .icon(icons::EXPORT_PDF)
                .item("SVG olarak…", wrap(Message::Export(ExportKind::Svg)))
                .icon(icons::EXPORT_SVG)
                .item("PNG olarak…", wrap(Message::Export(ExportKind::Png)))
                .icon(icons::EXPORT_PNG)
                .separator()
                .item(
                    "Pafta dosyası (.kpafta)…",
                    wrap(Message::Export(ExportKind::Kpafta)),
                )
                .icon(icons::EXPORT_KPAFTA)
        };
        let errors = self
            .findings
            .iter()
            .filter(|f| f.severity == kentos_sheet::preflight::Severity::Error)
            .count();
        groups.push(
            Group::new("Çıktı")
                .icon(icons::PREFLIGHT)
                .tool(
                    Button::large(icons::PREFLIGHT, "Ön denetim")
                        .on_press(wrap(Message::InspectorTab(InspectorTab::Preflight)))
                        .tip(Tip::new("Ön denetim").body(if errors > 0 {
                            format!("{errors} hata: dışa aktarmadan önce bakın.")
                        } else {
                            "Sayfa dışında kalan, örtülen, bağı kopuk öğeler, sığmayan yazılar …"
                                .to_owned()
                        })),
                )
                .tool(Button::large(icons::EXPORT, "Dışa aktar").menu(export_menu))
                .tool(
                    Button::large(icons::PRINT, "Yazdır")
                        .on_press(wrap(Message::Print))
                        .tip(Tip::new("Yazdır…").body(
                            "Paftayı PDF olarak hazırlar ve sistemin görüntüleyicisinde açar; oradan yazdırılır.",
                        )),
                )
                .tool(
                    Button::large(icons::SAVE_TEMPLATE, "Şablon olarak kaydet")
                        .on_press(wrap(Message::SaveTemplate(SaveMessage::Open))),
                ),
        );
        groups
    }
}

/// Hizala ▾, Dağıt ▾, Sıra ▾ and Grubu çöz.
fn arrange<M: Clone + 'static>(
    group: Group<'static, M>,
    some: bool,
    several: bool,
    wrap: impl Fn(Message) -> M + Copy + 'static,
) -> Group<'static, M> {
    let align = move || {
        Menu::new()
            .item("Sola", wrap(Message::Align(AlignEdge::Left)))
            .icon(icons::ALIGN_LEFT)
            .item("Ortaya", wrap(Message::Align(AlignEdge::Center)))
            .icon(icons::ALIGN_CENTER)
            .item("Sağa", wrap(Message::Align(AlignEdge::Right)))
            .icon(icons::ALIGN_RIGHT)
            .separator()
            .item("Üste", wrap(Message::Align(AlignEdge::Top)))
            .icon(icons::ALIGN_TOP)
            .item("Düşeyde ortaya", wrap(Message::Align(AlignEdge::Middle)))
            .icon(icons::ALIGN_MIDDLE)
            .item("Alta", wrap(Message::Align(AlignEdge::Bottom)))
            .icon(icons::ALIGN_BOTTOM)
            .separator()
            .item("Sayfanın ortasına", wrap(Message::AlignTo(AlignTo::Page)))
            .icon(icons::ALIGN_TO_PAGE)
            .item(
                "Kenar boşluklarının ortasına",
                wrap(Message::AlignTo(AlignTo::Margins)),
            )
            .icon(icons::ALIGN_TO_MARGINS)
    };
    let distribute = move || {
        Menu::new()
            .item(
                "Yatayda ortalar eşit",
                wrap(Message::Distribute(Axis::X, DistributeMode::Centers)),
            )
            .icon(icons::DISTRIBUTE_H)
            .item(
                "Yatayda aralıklar eşit",
                wrap(Message::Distribute(Axis::X, DistributeMode::Gaps)),
            )
            .icon(icons::DISTRIBUTE_H_GAPS)
            .separator()
            .item(
                "Düşeyde ortalar eşit",
                wrap(Message::Distribute(Axis::Y, DistributeMode::Centers)),
            )
            .icon(icons::DISTRIBUTE_V)
            .item(
                "Düşeyde aralıklar eşit",
                wrap(Message::Distribute(Axis::Y, DistributeMode::Gaps)),
            )
            .icon(icons::DISTRIBUTE_V_GAPS)
            .separator()
            .item(
                "Genişlikleri eşitle",
                wrap(Message::MatchSize(SizeDimension::Width)),
            )
            .icon(icons::MATCH_WIDTH)
            .item(
                "Yükseklikleri eşitle",
                wrap(Message::MatchSize(SizeDimension::Height)),
            )
            .icon(icons::MATCH_HEIGHT)
    };
    let order = move || {
        Menu::new()
            .item("En öne getir", wrap(Message::Order(ReorderTo::Front)))
            .icon(icons::FRONT)
            .item("Bir öne", wrap(Message::Order(ReorderTo::Forward)))
            .icon(icons::FORWARD)
            .item("Bir arkaya", wrap(Message::Order(ReorderTo::Backward)))
            .icon(icons::BACKWARD)
            .item("En arkaya gönder", wrap(Message::Order(ReorderTo::Back)))
            .icon(icons::BACK)
    };
    let _ = several;
    group
        .tool(Button::small(icons::ALIGN_LEFT, "Hizala").menu(align))
        .tool(Button::small(icons::DISTRIBUTE_H, "Dağıt").menu(distribute))
        .tool(Button::small(icons::FRONT, "Sıra").menu(order))
        .tool(
            Button::small(icons::UNGROUP, "Grubu çöz")
                .on_press_maybe(some.then(|| wrap(Message::Ungroup))),
        )
}
