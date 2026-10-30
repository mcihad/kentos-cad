//! Reusable print settings. The core owns validation, application, relayout and inverse operations.
use crate::{
    Message,
    designer::{Designer, Dialog as Window},
};
use iced::widget::{button, column, row};
use iced::{Element, Fill};
use kentos_sheet::{
    Page,
    ops::Op,
    print_setup::{
        self, AppliedPageSetup, ApplyPageSetup, NamedPageSetup, PlotArea, PlotScale, PlotSettings,
        PrintDevice, SavePageSetups, SetSheetPlot,
    },
    units::RectUm,
    viewport::ScaleRatio,
};
use kentos_ui::{
    label, style,
    widget::{Dialog, InputField, NumberInput, Segmented, Switch},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Layout,
    Extents,
    Window,
}
impl std::fmt::Display for Area {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Layout => "Pafta · 1:1",
            Self::Extents => "Tüm öğeler",
            Self::Window => "Pencere",
        })
    }
}
#[derive(Clone, Debug)]
pub enum SetupMessage {
    Open,
    Name(String),
    Load(String),
    Remove(String),
    Area(Area),
    Window(usize, f64),
    Fit(bool),
    Ratio(bool, f64),
    Offset(usize, f64),
    Center(bool),
    Device(bool),
    DeviceName(String),
    Profile(String),
    Apply,
    Save,
}
#[derive(Clone, Debug)]
pub(crate) struct SetupDialog {
    sheet: String,
    page: Page,
    name: String,
    source: Option<String>,
    settings: PlotSettings,
    window: RectUm,
    device: String,
    error: Option<String>,
}
impl SetupDialog {
    fn check(&self) -> Result<(), String> {
        print_setup::validate_settings(&self.settings).map_err(|e| e.message)
    }
}
impl Designer {
    pub(crate) fn page_setup_update(&mut self, m: SetupMessage) {
        match m {
            SetupMessage::Open => {
                if let Some(s) = self.open_sheet() {
                    self.dialog = Some(Window::PageSetup(Box::new(SetupDialog {
                        sheet: s.id.clone(),
                        page: s.page.clone(),
                        name: s
                            .plot
                            .as_ref()
                            .and_then(|p| p.source.as_ref())
                            .and_then(|id| self.book.page_setups.iter().find(|p| &p.id == id))
                            .map(|p| p.name.clone())
                            .unwrap_or_default(),
                        source: s.plot.as_ref().and_then(|p| p.source.clone()),
                        settings: s
                            .plot
                            .as_ref()
                            .map(|p| p.settings.clone())
                            .unwrap_or_default(),
                        window: s
                            .plot
                            .as_ref()
                            .and_then(|p| match p.settings.area {
                                PlotArea::Window { rect } => Some(rect),
                                _ => None,
                            })
                            .unwrap_or_else(|| s.page.margin_rect()),
                        device: s
                            .plot
                            .as_ref()
                            .and_then(|p| match &p.settings.device {
                                PrintDevice::System { name } => Some(name.clone()),
                                _ => None,
                            })
                            .unwrap_or_default(),
                        error: None,
                    })));
                }
            }
            SetupMessage::Load(id) => {
                let setup = self.book.page_setups.iter().find(|p| p.id == id).cloned();
                if let (Some(setup), Some(Window::PageSetup(d))) = (setup, &mut self.dialog) {
                    d.name = setup.name;
                    d.page = setup.page;
                    d.source = Some(setup.id);
                    d.settings = setup.settings;
                    if let PlotArea::Window { rect } = d.settings.area {
                        d.window = rect;
                    }
                    if let PrintDevice::System { name } = &d.settings.device {
                        d.device = name.clone();
                    }
                    d.error = None;
                }
            }
            SetupMessage::Remove(id) => {
                let setups = self
                    .book
                    .page_setups
                    .iter()
                    .filter(|p| p.id != id)
                    .cloned()
                    .collect();
                self.commit(vec![Op::SavePageSetups(SavePageSetups { setups })]);
                if let Some(Window::PageSetup(d)) = &mut self.dialog {
                    if d.source.as_ref() == Some(&id) {
                        d.source = None;
                    }
                    d.error = self.error.clone();
                }
            }
            SetupMessage::Apply | SetupMessage::Save => {
                let Some(Window::PageSetup(d)) = &self.dialog else {
                    return;
                };
                let save = matches!(m, SetupMessage::Save);
                let invalid = d.check().err().or_else(|| {
                    (save && d.name.trim().is_empty())
                        .then(|| "Sayfa düzenine bir ad verin.".into())
                });
                if let Some(error) = invalid {
                    if let Some(Window::PageSetup(d)) = &mut self.dialog {
                        d.error = Some(error);
                    }
                    return;
                }
                let d = (**d).clone();
                let mut ops = Vec::new();
                if save {
                    let id = d
                        .source
                        .clone()
                        .unwrap_or_else(|| self.new_id("page-setup"));
                    let setup = NamedPageSetup {
                        id: id.clone(),
                        name: d.name.trim().into(),
                        page: d.page,
                        settings: d.settings,
                    };
                    let mut setups = self.book.page_setups.clone();
                    if let Some(old) = setups.iter_mut().find(|p| p.id == id) {
                        *old = setup;
                    } else {
                        setups.push(setup);
                    }
                    ops.push(Op::SavePageSetups(SavePageSetups { setups }));
                    ops.push(Op::ApplyPageSetup(ApplyPageSetup {
                        sheet: d.sheet,
                        setup: id,
                    }));
                } else {
                    ops.push(Op::SetPage(kentos_sheet::ops::SetPage {
                        owner: kentos_sheet::Owner::sheet(&d.sheet),
                        page: d.page,
                        relayout: true,
                    }));
                    ops.push(Op::SetSheetPlot(SetSheetPlot {
                        sheet: d.sheet,
                        plot: Some(AppliedPageSetup::capture(&self.book, d.source, d.settings)),
                    }));
                }
                self.commit(ops);
                if self.error.is_none() {
                    self.dialog = None;
                } else if let Some(Window::PageSetup(d)) = &mut self.dialog {
                    d.error = self.error.clone();
                }
            }
            other => {
                if let Some(Window::PageSetup(d)) = &mut self.dialog {
                    d.error = None;
                    match other {
                        SetupMessage::Name(v) => d.name = v,
                        SetupMessage::Profile(v) => {
                            d.settings.profile =
                                (!v.trim().is_empty()).then(|| v.trim().to_owned());
                            let saved = d
                                .settings
                                .profile_snapshot
                                .take()
                                .filter(|p| Some(&p.id) == d.settings.profile.as_ref());
                            d.settings.profile_snapshot = self
                                .book
                                .plot_profiles
                                .iter()
                                .find(|p| Some(&p.id) == d.settings.profile.as_ref())
                                .cloned()
                                .or(saved);
                        }
                        SetupMessage::Area(v) => {
                            d.settings.area = match v {
                                Area::Layout => PlotArea::Layout,
                                Area::Extents => PlotArea::Extents,
                                Area::Window => PlotArea::Window { rect: d.window },
                            }
                        }
                        SetupMessage::Window(i, v) => {
                            let v = (v * 1000.).round() as i32;
                            match i {
                                0 => d.window.left = v,
                                1 => d.window.top = v,
                                2 => d.window.width = v,
                                _ => d.window.height = v,
                            }
                            d.settings.area = PlotArea::Window { rect: d.window };
                        }
                        SetupMessage::Fit(v) => {
                            d.settings.scale = if v {
                                PlotScale::Fit
                            } else {
                                PlotScale::default()
                            }
                        }
                        SetupMessage::Ratio(paper, v) => {
                            if let PlotScale::Ratio { ratio } = &mut d.settings.scale {
                                if paper {
                                    ratio.paper = v;
                                } else {
                                    ratio.model = v;
                                }
                            }
                        }
                        SetupMessage::Offset(i, v) => {
                            d.settings.offset[i.min(1)] = (v * 1000.).round() as i32
                        }
                        SetupMessage::Center(v) => d.settings.center = v,
                        SetupMessage::Device(v) => {
                            d.settings.device = if v {
                                PrintDevice::System {
                                    name: d.device.clone(),
                                }
                            } else {
                                PrintDevice::Pdf
                            }
                        }
                        SetupMessage::DeviceName(v) => {
                            d.device = v.clone();
                            d.settings.device = PrintDevice::System { name: v };
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    pub(crate) fn page_setup_view<'a>(&'a self, d: &'a SetupDialog) -> Element<'a, Message> {
        let on = Message::PageSetup;
        let mut body=column![label::muted("Kâğıt ve kenar boşluklarını Sayfa bölümünde ayarlayın. Kayıt bunları da saklar; görünüm pencerelerinin ölçeği değişmez.")].spacing(10);
        for p in &self.book.page_setups {
            let id = p.id.clone();
            let remove = p.id.clone();
            body = body.push(
                row![
                    button(label::body(&p.name))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press(on(SetupMessage::Load(id)))
                        .style(style::button::secondary),
                    button(label::body("Sil"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press(on(SetupMessage::Remove(remove)))
                        .style(style::button::secondary)
                ]
                .spacing(6),
            );
        }
        body = body.push(
            InputField::new("Düzen adı", &d.name)
                .on_input(move |s| on(SetupMessage::Name(s)))
                .width(Fill),
        );
        body = body.push(label::body(format!(
            "Kâğıt: {:?} · {} × {} mm",
            d.page.paper,
            d.page.size.width / 1000,
            d.page.size.height / 1000
        )));
        let area = match d.settings.area {
            PlotArea::Layout => Area::Layout,
            PlotArea::Extents => Area::Extents,
            PlotArea::Window { .. } => Area::Window,
        };
        body = body.push(
            Segmented::new(
                [Area::Layout, Area::Extents, Area::Window],
                area,
                move |v| on(SetupMessage::Area(v)),
            )
            .width(Fill),
        );
        if area == Area::Window {
            for (i, (name, v)) in [
                ("Sol (mm)", d.window.left),
                ("Üst (mm)", d.window.top),
                ("Genişlik (mm)", d.window.width),
                ("Yükseklik (mm)", d.window.height),
            ]
            .into_iter()
            .enumerate()
            {
                body = body.push(
                    row![
                        label::body(name).width(Fill),
                        NumberInput::new(f64::from(v) / 1000., move |v| on(SetupMessage::Window(
                            i, v
                        )))
                        .decimals(2)
                    ]
                    .spacing(8),
                );
            }
        }
        if area != Area::Layout {
            body = body.push(row![
                label::body("Yazılabilir alana sığdır").width(Fill),
                Switch::new(matches!(d.settings.scale, PlotScale::Fit), move |v| on(
                    SetupMessage::Fit(v)
                ))
            ]);
            if let PlotScale::Ratio {
                ratio: ScaleRatio { paper, model },
            } = d.settings.scale
            {
                body = body.push(
                    row![
                        label::body("Baskı / pafta oranı").width(Fill),
                        NumberInput::new(paper, move |v| on(SetupMessage::Ratio(true, v)))
                            .range(0.000001..=1000000.)
                            .decimals(4),
                        label::body(":"),
                        NumberInput::new(model, move |v| on(SetupMessage::Ratio(false, v)))
                            .range(0.000001..=1000000.)
                            .decimals(4)
                    ]
                    .spacing(5),
                );
            }
            body = body.push(row![
                label::body("Ortala").width(Fill),
                Switch::new(d.settings.center, move |v| on(SetupMessage::Center(v)))
            ]);
        }
        for (i, title) in ["Yatay kayma (mm)", "Düşey kayma (mm)"]
            .into_iter()
            .enumerate()
        {
            body = body.push(
                row![
                    label::body(title).width(Fill),
                    NumberInput::new(f64::from(d.settings.offset[i]) / 1000., move |v| on(
                        SetupMessage::Offset(i, v)
                    ))
                    .decimals(2)
                ]
                .spacing(8),
            );
        }
        body = body.push(row![
            label::body("Sistem yazıcısı").width(Fill),
            Switch::new(
                matches!(d.settings.device, PrintDevice::System { .. }),
                move |v| on(SetupMessage::Device(v))
            )
        ]);
        if let PrintDevice::System { name } = &d.settings.device {
            body = body.push(
                InputField::new("Yazıcı adı", name)
                    .on_input(move |v| on(SetupMessage::DeviceName(v)))
                    .width(Fill),
            );
        }
        let mut profiles = vec![(String::new(), "Çizim görünüşü".to_owned())];
        profiles.extend(
            self.book
                .plot_profiles
                .iter()
                .map(|p| (p.id.clone(), p.name.clone())),
        );
        if let Some(id) = &d.settings.profile
            && !profiles.iter().any(|p| &p.0 == id)
        {
            profiles.push((
                id.clone(),
                d.settings
                    .profile_snapshot
                    .as_ref()
                    .filter(|p| &p.id == id)
                    .map_or_else(
                        || format!("Eksik profil: {id}"),
                        |p| format!("Kaydedilen: {}", p.name),
                    ),
            ));
        }
        let selected = profiles
            .iter()
            .position(|p| Some(p.0.as_str()) == d.settings.profile.as_deref())
            .unwrap_or(0);
        let keys = profiles.iter().map(|p| p.0.clone()).collect::<Vec<_>>();
        body = body.push(
            column![
                label::body("Baskı profili"),
                kentos_ui::widget::Select::new(
                    profiles
                        .iter()
                        .map(|p| kentos_ui::widget::Choice::new(&p.1)),
                    Some(selected),
                    move |i| on(SetupMessage::Profile(keys[i].clone()))
                )
            ]
            .spacing(4),
        );
        body=body.push(label::muted("PDF dışa aktarma her cihazda kullanılabilir. Fiziksel yazdırma, seçilen cihazı masaüstü yazdırma penceresinde doğrular."));
        if let Some(e) = d.error.clone().or_else(|| d.check().err()) {
            body = body.push(label::body(e).style(style::text::danger));
        }
        Dialog::new("Sayfa düzenleri")
            .on_close(crate::Message::CloseDialog)
            .width(650.)
            .scroll(body)
            .action(
                button(label::body("Vazgeç"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(Message::CloseDialog)
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Uygula"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(on(SetupMessage::Apply))
                    .style(style::button::secondary),
            )
            .action(
                button(label::strong("Kaydet ve uygula"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(on(SetupMessage::Save))
                    .style(style::button::primary),
            )
            .into()
    }
}
