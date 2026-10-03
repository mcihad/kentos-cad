//! Uygulama ayarları's sections, as the web's (AppSettingsDialog.ts and
//! SettingsShell.ts; docs/inventory/parity-audit.md A2, A3, A5): the list of
//! sections on the left with where the settings are kept under it; each
//! section's title, lead and groups of settings; “Bu bölümü varsayılana
//! döndür” for the section's own values. The draft, its events and saving
//! are settings_view.rs's.

use iced::widget::{Column, button, column, container, row, scrollable, space, text};
use iced::{Center, Element, Fill};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Accent, typography};
use kentos_ui::widget::{Banner, Dialog, Form, NumberInput, Switch, Tip, overlay, tip};
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::{App, Message};
use crate::settings::schema;
use crate::settings_look as look;
use crate::settings_view::{
    Edit, METRES, PX, SCALE, SNAP_KINDS, SettingsDraft, action, choices, crs_choice, listed,
    presets, range,
};

/// A section of the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Appearance,
    Snap,
    NewProjects,
    Engine,
    File,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Self::Appearance,
        Self::Snap,
        Self::NewProjects,
        Self::Engine,
        Self::File,
    ];

    /// Its name in the list.
    pub fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Görünüm",
            Self::Snap => "Kenetleme",
            Self::NewProjects => "Yeni projeler",
            Self::Engine => "Çizim motoru",
            Self::File => "Ayar dosyası",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Appearance => Icon::Contrast,
            Self::Snap => Icon::Magnet,
            Self::NewProjects => Icon::DocumentNew,
            Self::Engine => Icon::Cube,
            Self::File => Icon::Open,
        }
    }

    /// Its title over its settings.
    pub fn title(self) -> &'static str {
        match self {
            Self::Appearance => "Görünüm",
            Self::Snap => "Kenetleme ve seçim",
            Self::NewProjects => "Yeni proje varsayılanları",
            Self::Engine => "Çizim motoru",
            Self::File => "Ayar dosyası",
        }
    }

    /// What it holds, in a sentence.
    pub fn lead(self) -> &'static str {
        match self {
            Self::Appearance => {
                "Tema, vurgu rengi, yazı tipi, yazı boyutu, çizim zemini, artı imleç ve fare yardımcıları."
            }
            Self::Snap => {
                "İmlecin hangi noktalara yapışacağı ve nesneleri ne kadar yakından yakalayacağı; bu oturumun çizim yardımcıları."
            }
            Self::NewProjects => {
                "Oluşturacağınız her yeni projede başlangıçta önerilecek proje türü, çizim yazı tipi ve koordinat sistemi."
            }
            Self::Engine => {
                "Çizim alanını ekran kartında çizerken kenar yumuşatma, çözünürlük, sembol boyutu ve çizgi kalınlığı. Bu cihaza özgüdür; menüler, paneller ve pencereler her zaman tam kalitede çizilir."
            }
            Self::File => {
                "Ayarları bir dosyaya aktarın, başka bir bilgisayardan ya da web uygulamasından alın, varsayılanlara döndürün."
            }
        }
    }

    /// The values “Bu bölümü varsayılana döndür” sets back; none in Ayar dosyası.
    pub fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Appearance => &[
                "appearance.theme",
                "appearance.accent",
                "appearance.uiFont",
                "appearance.monoFont",
                "appearance.textSize",
                "appearance.corners",
                "appearance.shadows",
                "appearance.crosshair",
                "appearance.drawingBackground",
                "drafting.cursorInput",
                "drafting.commandBar",
                "drafting.hoverInfo",
                "appearance.startScreen",
            ],
            Self::Snap => &[
                "snap.endpoint",
                "snap.midpoint",
                "snap.center",
                "snap.node",
                "snap.intersection",
                "snap.perpendicular",
                "snap.tangent",
                "snap.nearest",
                "snap.centroid",
                "snap.extension",
                "snap.parallel",
                "snap.grid",
                "snap.gridEast",
                "snap.gridNorth",
                "snap.self",
                "snap.scaleMin",
                "snap.scaleMax",
                "drafting.snapAperture",
                "drafting.pickAperture",
                "drafting.polarIncrement",
                "drafting.ortho",
                "drafting.polar",
                "drafting.snap",
            ],
            Self::NewProjects => &[
                "newProjects.workspace",
                "newProjects.drawingFont",
                "newProjects.srid",
            ],
            Self::Engine => &[
                "graphics.msaa",
                "graphics.hiDpi",
                "graphics.symbolSize",
                "graphics.lineWeights",
            ],
            Self::File => &[],
        }
    }
}

/// A section's controls over the draft.
struct Fields<'a> {
    app: &'a App,
    draft: &'a SettingsDraft,
}

impl<'a> Fields<'a> {
    fn value(&self, key: &str) -> Value {
        self.draft.values.get(key).cloned().unwrap_or(Value::Null)
    }

    fn title(key: &str) -> &'static str {
        schema().get(key).map_or("", |d| d.title.as_str())
    }

    fn help(key: &str) -> &'static str {
        schema().get(key).map_or("", |d| d.description.as_str())
    }

    /// A switch; one the organisation's policy fixes shows the value in use
    /// and cannot be turned.
    fn switch(&self, key: &'static str, caption: Option<&'static str>) -> Switch<'a, Message> {
        let control = match self.app.settings.resolved(key).filter(|r| r.locked) {
            Some(r) => Switch::disabled(r.effective.as_bool().unwrap_or(false))
                .label("Kurum politikası sabitliyor"),
            None => Switch::new(self.value(key).as_bool().unwrap_or(false), move |v| {
                Message::Settings(Edit::Value(key, Value::Bool(v)))
            }),
        };
        match caption {
            Some(caption) if self.app.settings.resolved(key).is_some_and(|r| !r.locked) => {
                control.label(caption)
            }
            _ => control,
        }
    }

    /// A whole number of pixels.
    /// Metres to a millimetre, written with a point (Karelaj's spacings; DESIGN.md §10.3).
    fn metres(&self, key: &'static str, fallback: f64) -> NumberInput<'a, Message> {
        NumberInput::new(self.value(key).as_f64().unwrap_or(fallback), move |v| {
            Message::Settings(Edit::Value(key, Value::from(v)))
        })
        .units(METRES)
        .range(range(key))
        .step(1.0)
        .decimals(3)
        .point()
        .width(120)
    }

    /// A screen scale's denominator, 0 no limit (the snap's scale range).
    fn scale(&self, key: &'static str) -> NumberInput<'a, Message> {
        NumberInput::new(self.value(key).as_f64().unwrap_or(0.0), move |v| {
            Message::Settings(Edit::Value(key, Value::from(v.round() as i64)))
        })
        .units(SCALE)
        .range(range(key))
        .step(500.0)
        .decimals(0)
        .width(120)
    }

    fn pixels(&self, key: &'static str, fallback: f64) -> NumberInput<'a, Message> {
        NumberInput::new(self.value(key).as_f64().unwrap_or(fallback), move |v| {
            Message::Settings(Edit::Value(key, Value::from(v.round() as i64)))
        })
        .units(PX)
        .range(range(key))
        .step(1.0)
        .decimals(0)
        .width(120)
    }

    /// A switch with its title and description.
    fn switch_field(
        &self,
        form: Form<'a, Message>,
        key: &'static str,
        caption: Option<&'static str>,
    ) -> Form<'a, Message> {
        form.field(Self::title(key), self.switch(key, caption))
            .help(Self::help(key))
    }
}

impl App {
    /// The window: the sections, the one chosen, and its buttons.
    pub(crate) fn settings_dialog(&self) -> Element<'_, Message> {
        let Some(draft) = &self.settings_draft else {
            return text("").into();
        };
        let current = draft.section;
        let list = Section::ALL
            .iter()
            .fold(Column::new().spacing(2), |list, section| {
                list.push(
                    button(
                        row![
                            icon(section.icon()).size(16.0),
                            label::body(section.label())
                        ]
                        .spacing(8)
                        .align_y(Center),
                    )
                    .on_press(Message::Settings(Edit::Section(*section)))
                    .padding([6, 10])
                    .width(Fill)
                    .style(style::button::navigation(*section == current)),
                )
            });
        // Where these settings are kept (the web's scope box).
        let scope = container(
            row![
                icon(Icon::Properties).size(16.0),
                column![
                    label::body("Bu bilgisayarda saklanır").font(typography::ui_strong()),
                    label::caption("Tüm projeler için geçerlidir").style(style::text::muted),
                ]
                .spacing(2),
            ]
            .spacing(8),
        )
        .padding(10)
        .width(Fill)
        .style(style::container::field_box);
        let nav = column![list, space::vertical(), scope]
            .width(200)
            .height(Fill);
        let fields = Fields { app: self, draft };
        let page = match current {
            Section::Appearance => self.appearance_section(&fields),
            Section::Snap => snap_section(&fields),
            Section::NewProjects => self.new_projects_section(&fields),
            Section::Engine => self.engine_section(&fields),
            Section::File => self.file_section(draft),
        };
        let content = column![
            text(current.title())
                .font(typography::ui_strong())
                .size(typography::heading()),
            label::caption(current.lead()).style(style::text::muted),
            // The page scrolls; the buttons stay in view whatever the window's height.
            scrollable(page)
                .direction(style::field::body_scrollbar())
                .height(Fill),
        ]
        .spacing(10)
        .width(Fill);
        let own = !current.keys().is_empty();
        let reset = button(label::body("Bu bölümü varsayılana döndür"))
            .on_press_maybe(own.then_some(Message::Settings(Edit::ResetSection)))
            .padding([5, 16])
            .style(style::button::ghost);
        let reset: Element<'_, Message> = if own {
            reset.into()
        } else {
            tip(
                reset,
                Tip::new("Bu bölümde varsayılana dönecek ayar yok."),
                iced::widget::tooltip::Position::Top,
            )
        };
        let save = button(label::body("Kaydet"))
            .on_press_maybe(draft.changed().then_some(Message::Settings(Edit::Save)))
            .padding([5, 16])
            .style(style::button::primary);
        let cancel = button(label::body("Vazgeç"))
            .on_press(Message::DialogClosed)
            .padding([5, 16])
            .style(style::button::secondary);
        overlay::modal(
            Dialog::new("Uygulama ayarları")
                .hint("Ctrl+,")
                .push(row![nav, content].spacing(16))
                .aside(reset)
                .action(cancel)
                .action(save)
                .width(940.0)
                .max_height(760.0),
            Message::DialogClosed,
        )
    }

    fn appearance_section<'a>(&self, f: &Fields<'a>) -> Element<'a, Message> {
        // The web's order and pickers (AppSettingsDialog.ts appearance()):
        // theme cards, accent swatches and typeface cards, each with the
        // desktop's own settings of its kind under it.
        let accent = f.value("appearance.accent");
        let accent = accent.as_str().unwrap_or("navy");
        let form = || Form::new().label_width(170.0);
        let theme = look::group(
            Fields::title("appearance.theme"),
            None,
            column![
                look::theme_cards(
                    &f.value("appearance.theme"),
                    Accent::parse(accent).unwrap_or_default(),
                ),
                form()
                    .field(
                        Fields::title("appearance.drawingBackground"),
                        choices(
                            "appearance.drawingBackground",
                            &f.value("appearance.drawingBackground"),
                        ),
                    )
                    .help(Fields::help("appearance.drawingBackground")),
            ]
            .spacing(14),
        );
        let accent = look::group(
            Fields::title("appearance.accent"),
            Some(Fields::help("appearance.accent")),
            look::accent_swatches(accent),
        );
        let typeface = look::group(
            Fields::title("appearance.uiFont"),
            Some(Fields::help("appearance.uiFont")),
            column![
                look::typeface_cards(&f.value("appearance.uiFont")),
                form()
                    .field(
                        Fields::title("appearance.monoFont"),
                        choices("appearance.monoFont", &f.value("appearance.monoFont")),
                    )
                    .help(Fields::help("appearance.monoFont"))
                    .field(
                        Fields::title("appearance.textSize"),
                        look::text_sizes(
                            &f.value("appearance.textSize"),
                            f.pixels("appearance.textSize", 13.0),
                        ),
                    )
                    .help(Fields::help("appearance.textSize")),
            ]
            .spacing(14),
        );
        let rest = form()
            .section("Biçim")
            .field(
                Fields::title("appearance.corners"),
                choices("appearance.corners", &f.value("appearance.corners")),
            )
            .help(Fields::help("appearance.corners"))
            .field(
                Fields::title("appearance.shadows"),
                choices("appearance.shadows", &f.value("appearance.shadows")),
            )
            .help(Fields::help("appearance.shadows"))
            .section("İmleç ve fare yardımcıları")
            .field(
                Fields::title("appearance.crosshair"),
                choices("appearance.crosshair", &f.value("appearance.crosshair")),
            )
            .help(Fields::help("appearance.crosshair"));
        let rest = [
            "drafting.cursorInput",
            "drafting.commandBar",
            "drafting.hoverInfo",
        ]
        .into_iter()
        .fold(rest, |form, key| f.switch_field(form, key, None));
        let rest = f.switch_field(rest.section("Açılış"), "appearance.startScreen", None);
        column![theme, accent, typeface, rest].spacing(18).into()
    }

    fn new_projects_section<'a>(&self, f: &Fields<'a>) -> Element<'a, Message> {
        let mut form = Form::new()
            .label_width(170.0)
            .section("Önerilenler")
            .field(
                Fields::title("newProjects.workspace"),
                choices("newProjects.workspace", &f.value("newProjects.workspace")),
            )
            .help(Fields::help("newProjects.workspace"))
            .field(
                Fields::title("newProjects.drawingFont"),
                listed(
                    "newProjects.drawingFont",
                    &f.value("newProjects.drawingFont"),
                ),
            )
            .help(Fields::help("newProjects.drawingFont"))
            .field(
                Fields::title("newProjects.srid"),
                crs_choice(
                    u32::try_from(f.value("newProjects.srid").as_u64().unwrap_or(5256))
                        .unwrap_or(5256),
                ),
            )
            .help(Fields::help("newProjects.srid"));
        // The open project keeps its own system: said, with the way to it (the web's).
        if let Some(doc) = &self.document {
            let srid = doc.settings().srid;
            let name = crate::crs::system(srid).map_or_else(String::new, |c| c.name.clone());
            form = form
                .section("Açık proje")
                .field(
                    "Bu projenin sistemi",
                    action("Proje ayarlarını aç", Edit::OpenProject),
                )
                .help(format!(
                    "{name} (EPSG:{srid}). Proje ayarlarından değiştirilir ve proje dosyasına kaydedilir."
                ));
        }
        form.into()
    }

    fn engine_section<'a>(&'a self, f: &Fields<'a>) -> Element<'a, Message> {
        let form = Form::new()
            .label_width(170.0)
            .section("Grafik (bu cihaz)")
            .field("Hazır ayar", presets(f.draft))
            .help("Hızlı, Dengeli ve Kaliteli yalnız aşağıdaki iki değeri doldurur; her biri ayrıca değiştirilebilir. Çizimin kaydını ve hassasiyetini değiştirmez.")
            .field(
                Fields::title("graphics.msaa"),
                choices("graphics.msaa", &f.value("graphics.msaa")),
            )
            .help(Fields::help("graphics.msaa"))
            .row(self.effective_note("graphics.msaa", &f.value("graphics.msaa")));
        let form = f
            .switch_field(form, "graphics.hiDpi", None)
            // The styled drawing (docs/adr/0090), as the web's Çizim motoru → Semboller ve çizgiler.
            .section("Semboller ve çizgiler")
            .field(
                Fields::title("graphics.symbolSize"),
                choices("graphics.symbolSize", &f.value("graphics.symbolSize")),
            )
            .help(Fields::help("graphics.symbolSize"));
        f.switch_field(form, "graphics.lineWeights", None).into()
    }

    fn file_section<'a>(&self, draft: &SettingsDraft) -> Element<'a, Message> {
        let mut form = Form::new()
            .label_width(170.0)
            .section("Dışa ve içe aktarma")
            .field("Dışa aktar", action("Dışa aktar…", Edit::Export))
            .help("Kaydedilmiş uygulama ayarlarını bir kentos.settings dosyasına yazar. Web uygulaması da aynı dosyayı okur; tema, vurgu rengi, yazı tipi ve yazı boyutu da onunla taşınır.")
            .field("İçe aktar", action("İçe aktar…", Edit::Import))
            .help("Bir ayar dosyasının değerleri bu pencereye gelir; Kaydet ile uygulanır. Geçersiz değerler alınmaz ve adıyla söylenir.");
        if let Some((warn, note)) = &draft.note {
            form = form.row(if *warn {
                Banner::warning(note.clone())
            } else {
                Banner::info(note.clone())
            });
        }
        form.section("Varsayılanlar")
            .field("Varsayılanlara döndür", action("Varsayılanlara döndür", Edit::Reset))
            .help("Bütün bölümlerdeki ayarlar varsayılanına döner; Kaydet ile saklanan değerler silinir.")
            .section("Kayıt")
            .row(label::caption(self.where_kept()))
            .into()
    }
}

fn snap_section<'a>(f: &Fields<'a>) -> Element<'a, Message> {
    let form = Form::new().label_width(170.0).section("Kenet türleri");
    let form = SNAP_KINDS
        .iter()
        .fold(form, |form, key| f.switch_field(form, key, None))
        // Karelaj's spacings and where snapping works (docs/adr/0163 §1, §3, §5).
        .section("Karelaj")
        .field(
            Fields::title("snap.gridEast"),
            f.metres("snap.gridEast", 1.0),
        )
        .help(Fields::help("snap.gridEast"))
        .field(
            Fields::title("snap.gridNorth"),
            f.metres("snap.gridNorth", 1.0),
        )
        .help(Fields::help("snap.gridNorth"))
        .section("Kenedin kapsamı");
    let form = f
        .switch_field(form, "snap.self", None)
        .field(Fields::title("snap.scaleMin"), f.scale("snap.scaleMin"))
        .help(Fields::help("snap.scaleMin"))
        .field(Fields::title("snap.scaleMax"), f.scale("snap.scaleMax"))
        .help(Fields::help("snap.scaleMax"))
        .section("Yakalama")
        .field(
            Fields::title("drafting.snapAperture"),
            f.pixels("drafting.snapAperture", 11.0),
        )
        .help(Fields::help("drafting.snapAperture"))
        .field(
            Fields::title("drafting.pickAperture"),
            f.pixels("drafting.pickAperture", 5.0),
        )
        .help(Fields::help("drafting.pickAperture"))
        .section("Kutupsal izleme")
        .field(
            Fields::title("drafting.polarIncrement"),
            choices(
                "drafting.polarIncrement",
                &f.value("drafting.polarIncrement"),
            ),
        )
        .help(Fields::help("drafting.polarIncrement"))
        // Orto, Kutupsal izleme and Kenetleme hold for this session (the status bar's toggles).
        .section("Bu oturum");
    ["drafting.ortho", "drafting.polar", "drafting.snap"]
        .into_iter()
        .fold(form, |form, key| {
            f.switch_field(form, key, Some("Bu oturum"))
        })
        .into()
}
