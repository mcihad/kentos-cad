//! Görünüm: bileşenlerin yerleşimi.
//!
//! ```text
//! ┌ Şerit ─────────────────────────────────────────────────┐
//! ├ Model alanı ────────────────────────────┬ Katmanlar  ⋯ ┤
//! │                                         │              │
//! ├ Model │ Düzen 1 │ Düzen 2 │ + ──────────┼ Özellikler ⋯ ┤
//! ├ Öznitelik tablosu │ Görevler ─────── ⋯ ─┤ (nesne       │
//! │                                         │  inceleyici) │
//! ├ Komut satırı ───────────────────────────┴──────────────┤
//! ├ Durum çubuğu ──────────────────────────────────────────┤
//! ```
//!
//! Paneller yuvadadır (`DockSpace`): sekmeleri sürüklenerek yeniden
//! düzenlenir, yüzdürülür, kapatılır; yerleşim ayar dosyasında saklanır.
//! Model ve düzen sekmeleri haritanın altındadır; düzen sekmesinde harita
//! yerini kâğıt paftaya bırakır. Model alanının üstünde kayan araç
//! pencereleri (ölçüm, koordinata git, katman stili) durur; bildirimler
//! pencerenin sağ alt köşesindedir. Galeri sekmesinde model alanı ve yan
//! paneller yerini bileşen kataloğuna bırakır. Üst katmanlar önceliğe göre
//! tek tek açılır: uygulama menüsü, sorgu penceresi, kısayollar.

mod app_menu;
mod attribute_table;
mod dock;
mod gallery;
mod help;
mod import;
mod layers;
mod menus;
mod panes;
mod properties;
mod query;
mod ribbon;
mod sheet;
mod status;

use std::fmt;

use iced::widget::{Column, column, container, stack};
use iced::{Element, Fill};

use kentos_rc::icon::Icon;
use kentos_rc::spatial::model_space::Backdrop;
use kentos_rc::spatial::{Layer, ModelSpace, Tool, ViewCube, format};
use kentos_rc::style;
use kentos_rc::theme::typography::{self, Family, Mono, Typography};
use kentos_rc::theme::{Accent, Mode};
use kentos_rc::widget::command_line::{self, Prompt};
use kentos_rc::widget::docking::Side;
use kentos_rc::widget::{
    Banner, CommandLine, Confirm, ContextMenu, EmptyState, Floating, NavigationBar, Toaster,
    overlay, status_bar,
};

use crate::app::{COMMAND_INPUT, DRAWING_LAYER, Showcase};
use crate::command::{self, Command};
use crate::message::{Confirmation, Keyword, Message, Pending, RibbonTab};

impl Showcase {
    pub fn view(&self) -> Element<'_, Message> {
        let workspace: Element<'_, Message> = if self.ribbon_tab == RibbonTab::Gallery {
            column![self.gallery(), self.command_line()]
                .height(Fill)
                .into()
        } else {
            let mut drawing = Column::new();

            // Süren durumlar haritanın üstünde şerit olarak durur.
            if self.read_only_notice {
                drawing = drawing.push(
                    Banner::warning(
                        "Örnek veri katmanları salt okunur: yalnızca Çizimler katmanındaki \
                         öğeler silinir ve düzenlenir.",
                    )
                    .action("Çizimlere geç", Message::LayerActivated(DRAWING_LAYER))
                    .on_dismiss(Message::BannerDismissed),
                );
            }

            // Model ve düzen sekmeleri haritanın altına asılır; paneller
            // yuvada, haritanın çevresindedir. Komut satırı en altta,
            // pencere boyunca uzanır.
            drawing = drawing.push(self.model_space()).push(self.sheet_tabs());

            column![self.dock_space(drawing), self.command_line()]
                .height(Fill)
                .into()
        };

        let base = container(column![self.ribbon(), workspace, self.status_bar()])
            .width(Fill)
            .height(Fill)
            .style(style::container::window);

        // Bildirimler bütün pencereye bağlıdır: her sekmede aynı yerde,
        // komut satırının hemen üstünde, sağ alt köşede durur. Sağ alan
        // açıksa onun genişliğine sığar; çizim alanına girmez.
        let right = if self.docks.stacks(Side::Right).is_empty() {
            340.0
        } else {
            typography::scaled(self.docks.size(Side::Right)) - 16.0
        };
        let base = Toaster::new(base, &self.toasts, Message::ToastClosed)
            .width(right.clamp(240.0, 340.0))
            .padding(iced::Padding {
                bottom: status_bar::height()
                    + command_line::height(command_line::LINES, self.command_expanded)
                    + 8.0,
                ..iced::Padding::new(8.0)
            });

        let overlay = if let Some(confirmation) = self.confirm {
            Some(self.confirmation(confirmation))
        } else if let Some(wizard) = &self.import {
            Some(self.import_wizard(wizard))
        } else if let Some(properties) = &self.properties {
            Some(self.layer_properties(properties))
        } else if self.app_menu_open {
            Some(self.app_menu())
        } else if let Some(dialog) = &self.query {
            Some(self.query_dialog(dialog))
        } else if self.help_open {
            Some(self.help())
        } else {
            None
        };

        match overlay {
            Some(overlay) => stack![base, overlay].into(),
            None => base.into(),
        }
    }

    /// Açık sekmenin alanı: harita ya da düzen. Kayan araç pencereleri
    /// alanın üstündedir; dışlarında harita çalışmayı sürdürür.
    fn model_space(&self) -> Element<'_, Message> {
        let area = match self.sheets.sheet() {
            Some(sheet) => self.sheet_view(sheet),
            None => self.map(),
        };

        Floating::new(area, &self.windows, Message::Window, move |pane| {
            self.pane(pane)
        })
        .into()
    }

    fn map(&self) -> Element<'_, Message> {
        let drawing_color = self
            .layers
            .get(DRAWING_LAYER)
            .map(|layer| layer.color)
            .unwrap_or_default();

        let navigation = NavigationBar::new()
            .button(Icon::ZoomIn, "Yakınlaştır", Message::ZoomIn)
            .button(Icon::ZoomOut, "Uzaklaştır", Message::ZoomOut)
            .separator()
            .button(Icon::ZoomExtents, "Tümünü gör", Message::FitAll)
            .button(
                Icon::Target,
                "Aktif katmana sığdır",
                Message::ZoomToLayer(self.active_layer),
            )
            .button(Icon::Home, "Başlangıç görünümü", Message::ResetView);

        let model_space = ModelSpace::new(self.viewport, &self.layers, Message::ModelSpace)
            .backdrop(self.backdrop)
            .tool(self.tool)
            .selection(&self.selection)
            .hover(self.hover)
            .measurement(self.measurement.points())
            .draft(self.draft.points(), drawing_color)
            .options(self.options)
            .prompt(self.picking.as_ref().map(|pick| pick.prompt.as_str()))
            .view_cube(self.view_cube.then(|| ViewCube::new(self.cube_rotation)))
            .navigation(navigation);

        // Seç ve Kaydır araçlarında sağ tık bağlam menüsünü açar; çizim ve
        // ölçüm araçlarında model alanı sağ tıkı kendisi kullanır.
        let map = ContextMenu::new(model_space, move |position| self.map_menu(position));

        // Görünür katman yokken harita boştur; ortada ne olduğu ve nasıl
        // düzeltileceği yazar.
        if self.layers.iter().any(|layer| layer.visible) {
            map.into()
        } else {
            stack![
                map,
                EmptyState::new(Icon::Layers, "Haritada görünür katman yok")
                    .description(
                        "Bütün katmanlar gizli. Katman ağacındaki kutularla tek tek ya da \
                         buradan hepsini birden gösterin.",
                    )
                    .primary("Tümünü göster", Message::ShowAllLayers),
            ]
            .into()
        }
    }

    /// Onay bekleyen işin onay kutusu; Enter onaylar, Esc vazgeçer.
    fn confirmation(&self, confirmation: Confirmation) -> Element<'_, Message> {
        let drawings = self
            .layers
            .get(DRAWING_LAYER)
            .map_or(0, |layer| layer.features.len());

        let dialog = match confirmation {
            Confirmation::ClearDrawings => Confirm::new(
                "Bütün çizimler silinsin mi?",
                Message::ConfirmAccepted,
                Message::ConfirmCancelled,
            )
            .message(format!("Çizimler katmanındaki {drawings} öğe silinecek."))
            .detail("Silinen çizimler bildirimdeki Geri al ile geri getirilebilir.")
            .confirm("Tümünü sil")
            .destructive(),
            Confirmation::Quit => Confirm::new(
                "Çizimler kaydedilmeden çıkılsın mı?",
                Message::ConfirmAccepted,
                Message::ConfirmCancelled,
            )
            .message(format!(
                "Çizimler katmanında {drawings} öğe var. Kaydetme bu sürümde yok; çıkınca \
                 çizimler kaybolur."
            ))
            .confirm("Kaydetmeden çık")
            .destructive(),
        };

        overlay::modal(dialog, Message::ConfirmCancelled)
    }

    fn command_line(&self) -> Element<'_, Message> {
        CommandLine::new(&self.history, &self.command_input)
            .id(COMMAND_INPUT)
            .placeholder("Komut ya da enlem, boylam yazın")
            .commands(command::catalog())
            .prompt(self.prompt())
            .on_input(Message::CommandInput)
            .on_submit(Message::CommandSubmitted)
            .on_run(Message::CommandRun)
            .expanded(self.command_expanded, |_| Message::CommandHistoryToggled)
            .into()
    }

    /// Etkin komutun istemi: haritadan seçim, çizim ya da ölçüm sürerken
    /// beklenen adım ve seçenekleri. Seç ve Kaydır araçlarında istem yoktur.
    pub(crate) fn prompt(&self) -> Option<Prompt<'static, Message>> {
        let keyword = Message::Keyword;

        if let Some(pending) = self.pending {
            return Some(self.pending_prompt(pending));
        }

        if let Some(pick) = &self.picking {
            return Some(
                Prompt::new(pick.prompt.clone())
                    .command("SEC")
                    .option("İptal", keyword(Keyword::Cancel))
                    .key("Esc")
                    .description("Haritadan seçimi bırakır; alanın değeri değişmez."),
            );
        }

        let points = match self.tool {
            Tool::Measure => self.measurement.points().len(),
            _ => self.draft.points().len(),
        };

        let (text, finish) = match (self.tool, points) {
            (Tool::Select | Tool::Pan, _) => return None,
            (Tool::Point, _) => ("Noktanın yerini belirtin".to_owned(), None),
            (Tool::Measure, 0) => ("Ölçülecek ilk noktayı belirtin".to_owned(), None),
            (Tool::Measure, _) => (
                format!(
                    "Sonraki noktayı belirtin; toplam {}",
                    format::distance(self.measurement.total_meters())
                ),
                Some((
                    "Temizle",
                    Keyword::Clear,
                    "Ölçümü siler; yeni ölçüm ilk noktadan başlar.",
                )),
            ),
            (Tool::Polygon, 0) | (Tool::Rectangle, 0) => ("İlk köşeyi belirtin".to_owned(), None),
            (Tool::Polygon, count) => (
                "Sonraki köşeyi belirtin".to_owned(),
                (count >= 3).then_some((
                    "Kapat",
                    Keyword::Close,
                    "Son köşeyi ilk köşeye bağlayıp alanı tamamlar.",
                )),
            ),
            (Tool::Rectangle, _) => ("Karşı köşeyi belirtin".to_owned(), None),
            (Tool::Circle, 0) => ("Merkezi belirtin".to_owned(), None),
            (Tool::Circle, _) => ("Çember üzerinde bir nokta belirtin".to_owned(), None),
            (_, 0) => ("İlk noktayı belirtin".to_owned(), None),
            (Tool::Polyline, count) => (
                "Sonraki noktayı belirtin".to_owned(),
                (count >= 2).then_some((
                    "Bitir",
                    Keyword::Finish,
                    "Çoklu çizgiyi son noktada tamamlar.",
                )),
            ),
            (_, _) => (
                "Sonraki noktayı belirtin".to_owned(),
                Some((
                    "Bitir",
                    Keyword::Finish,
                    "Çizgi zincirini bitirir; sonraki çizgi yeni bir noktadan başlar.",
                )),
            ),
        };

        let mut prompt = Prompt::new(text)
            .command(command::name(Command::Tool(self.tool)))
            .placeholder("ya da enlem, boylam yazın");

        if points > 0 {
            prompt = prompt
                .option("Geri al", keyword(Keyword::Undo))
                .description("Son noktayı kaldırır.");
        }

        if let Some((label, choice, description)) = finish {
            prompt = prompt.option(label, keyword(choice));

            // Ölçümde boş Enter bir şey yapmaz; çizimde bitirir.
            if choice != Keyword::Clear {
                prompt = prompt.key("Enter");
            }

            prompt = prompt.description(description);
        }

        Some(prompt)
    }

    /// Seçenek bekleyen komutların istemi: yazı ailesi, yazı boyutu, vurgu
    /// rengi ve harita zemini.
    fn pending_prompt(&self, pending: Pending) -> Prompt<'static, Message> {
        let current = self.typography;
        let choose = move |typography: Typography| Message::TypographyChanged(typography);

        match pending {
            Pending::Accent => Accent::PRESETS.into_iter().fold(
                Prompt::new(format!("Vurgu rengini seçin; şimdi {}", self.accent.name()))
                    .command(command::name(Command::Accent))
                    .placeholder("ya da #RRGGBB yazın"),
                |prompt, accent| {
                    let [dark, light] =
                        [Mode::Dark, Mode::Light].map(|mode| hex_of(accent.color(mode)));

                    prompt
                        .option(accent.name(), Message::AccentChanged(accent))
                        .description(format!("Koyu temada {dark}, aydınlık temada {light}."))
                },
            ),
            Pending::Backdrop => Backdrop::ALL.into_iter().fold(
                Prompt::new(format!(
                    "Harita zeminini seçin; şimdi {}",
                    self.backdrop.name()
                ))
                .command(command::name(Command::Backdrop)),
                |prompt, backdrop| {
                    prompt
                        .option(backdrop.name(), Message::BackdropChanged(backdrop))
                        .description(backdrop_note(backdrop))
                },
            ),
            Pending::Typeface => {
                let prompt = Family::ALL.into_iter().fold(
                    Prompt::new("Yazı ailesini seçin")
                        .command(command::name(Command::Typeface))
                        .placeholder("ya da adını yazın"),
                    |prompt, family| {
                        prompt
                            .option(family.name(), choose(Typography { family, ..current }))
                            .description(family_note(family))
                    },
                );

                Mono::ALL.into_iter().fold(prompt, |prompt, mono| {
                    prompt
                        .option(mono.name(), choose(Typography { mono, ..current }))
                        .description("Koordinat, ölçü ve komutların eş aralıklı yazısı.")
                })
            }
            Pending::TextSize => {
                let (smallest, largest) = (
                    *Typography::SIZES.start() as u8,
                    *Typography::SIZES.end() as u8,
                );

                (smallest..=largest).fold(
                    Prompt::new(format!(
                        "Yazı boyutunu seçin; şimdi {} piksel",
                        current.size
                    ))
                    .command(command::name(Command::TextSize))
                    .placeholder("ya da sayıyı yazın"),
                    |prompt, size| {
                        prompt
                            .option(
                                size.to_string(),
                                choose(Typography {
                                    size: f32::from(size),
                                    ..current
                                }),
                            )
                            .description(format!("Gövde metni {size} piksel."))
                    },
                )
            }
        }
    }

    /// Katman seçim kutularının seçenekleri.
    fn layer_choices(&self) -> Vec<LayerChoice<'_>> {
        self.layers
            .iter()
            .enumerate()
            .map(|(index, layer)| LayerChoice::new(index, layer))
            .collect()
    }
}

/// Katman seçim kutusunun seçeneği.
#[derive(Debug, Clone, PartialEq)]
struct LayerChoice<'a> {
    index: usize,
    name: &'a str,
}

impl<'a> LayerChoice<'a> {
    fn new(index: usize, layer: &'a Layer) -> Self {
        Self {
            index,
            name: &layer.name,
        }
    }
}

impl fmt::Display for LayerChoice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// Rengin onaltılık yazımı (#rrggbb).
pub(crate) fn hex_of(color: iced::Color) -> String {
    let [red, green, blue, _] = color.into_rgba8();

    format!("#{red:02x}{green:02x}{blue:02x}")
}

/// Temanın kısa tanımı.
pub(crate) fn theme_note(mode: Mode) -> &'static str {
    match mode {
        Mode::Dark => "CAD programlarının grafit arayüzü; uzun çalışmada göz yormaz.",
        Mode::Light => {
            "Kâğıt zeminli aydınlık arayüz; aydınlık ortamda ve çıktıya yakın çalışırken."
        }
        Mode::Night => {
            "Çok koyu, az parlak arayüz ve kısık renkli gece haritası; karanlık odada ekran \
             parlamaz."
        }
        Mode::HighContrast => {
            "Siyah zemin, beyaz yazı ve parlak kenarlar; yazılar ve vurgu en az 7:1 karşıtlıkta."
        }
    }
}

/// Harita zemininin kısa tanımı.
pub(crate) fn backdrop_note(backdrop: Backdrop) -> &'static str {
    match backdrop {
        Backdrop::Theme => {
            "Temaya uyar: koyu temada arduvaz, aydınlıkta kâğıt, gecede gece haritası, yüksek \
             karşıtlıkta siyah."
        }
        Backdrop::Slate => "AutoCAD'in koyu gri-mavi model alanı; göz yormaz.",
        Backdrop::Black => "Klasik AutoCAD: saf siyah zemin, parlak çizgiler.",
        Backdrop::Paper => "Beyaza yakın zemin; çıktıya en yakın görünüm.",
    }
}

/// Yazı ailesinin kısa tanımı.
pub(crate) fn family_note(family: Family) -> &'static str {
    match family {
        Family::IbmPlexSans => {
            "Mühendislik çizgili, dar ve sakin bir grotesk; ekrana çok metin sığar."
        }
        Family::Inter => "Ekran için çizilmiş; x yüksekliği büyük, küçük boyutta en okunaklısı.",
        Family::PlusJakartaSans => "Geometrik, açık ve yumuşak hatlı; ferah bir görünüm.",
    }
}
