//! Plot table drafts; one shared operation writes the library, optionally applying a captured profile to the open page.
use crate::{
    designer::{Designer, Dialog as Window},
};
use kentos_sheet::{
    ops::Op,
    plot_profile::{
        PlotProfile, PlotStyle, PlotColor, StyleKey, StyleEntry, TableKind, Assignment, Target,
        SavePlotProfiles,
    },
    print_setup::{AppliedPageSetup, SetSheetPlot},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorMode {
    Object,
    Black,
    Gray,
    Color,
}
impl std::fmt::Display for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Object => "Özgün",
            Self::Black => "Siyah",
            Self::Gray => "Gri",
            Self::Color => "Renk",
        })
    }
}
#[derive(Clone, Debug)]
pub enum PlotMessage {
    Open,
    New(TableKind),
    Select(usize),
    Remove,
    Name(String),
    Entry(usize),
    NewEntry,
    RemoveEntry,
    Key(StyleKey),
    Default(bool),
    ColorMode(ColorMode),
    Color(String),
    Screening(f64),
    Weight(bool),
    WeightMm(f64),
    Dash(bool),
    DashText(String),
    Opacity(bool),
    Alpha(f64),
    Cap(Option<kentos_sheet::LineCap>),
    Join(Option<kentos_sheet::LineJoin>),
    AssignLayer(String),
    AssignSelected,
    Unassign(Target),
    Save(bool),
}
#[derive(Clone, Debug)]
pub(crate) struct PlotDialog {
    pub profiles: Vec<PlotProfile>,
    pub selected: Option<usize>,
    pub entry: usize,
    pub dash: String,
    pub error: Option<String>,
}
impl PlotDialog {
    pub fn current(&self) -> Option<&PlotProfile> {
        self.profiles.get(self.selected?)
    }
    pub fn rule(&self) -> Option<&StyleEntry> {
        self.current()?.entries.get(self.entry)
    }
    fn current_mut(&mut self) -> Option<&mut PlotProfile> {
        self.profiles.get_mut(self.selected?)
    }
    fn rule_mut(&mut self) -> Option<&mut StyleEntry> {
        let i = self.entry;
        self.current_mut()?.entries.get_mut(i)
    }
    fn sync_dash(&mut self) {
        self.dash = self.rule().and_then(|r| r.style.dash.as_ref()).map_or_else(
            || "2 1".into(),
            |v| {
                v.iter()
                    .map(|v| (f64::from(*v) / 1000.).to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            },
        );
    }
    fn dash_values(&self) -> Result<Vec<i32>, String> {
        let values=self.dash.split_whitespace().map(|v|v.replace(',',".").parse::<f64>()).collect::<Result<Vec<_>,_>>().map_err(|_|"Çizgi ve boşluk uzunluklarını mm olarak, aralarına boşluk koyarak yazın; örnek: 2 1.".to_owned())?;
        if values.len() > 64
            || values
                .iter()
                .any(|v| !v.is_finite() || *v < 0. || (*v > 0. && *v < 0.001) || *v > 1000.)
            || (!values.is_empty() && values.iter().all(|v| *v == 0.))
        {
            return Err("En çok 64 aralık yazın: nokta için 0, diğer uzunluklar 0.001–1000 mm. Tümü sıfır olamaz; boş alan sürekli çizgidir.".into());
        }
        Ok(values.iter().map(|v| (v * 1000.).round() as i32).collect())
    }
    fn keep_dash(&mut self) -> bool {
        if self.rule().is_some_and(|r| r.style.dash.is_some()) {
            match self.dash_values() {
                Ok(v) => {
                    if let Some(r) = self.rule_mut() {
                        r.style.dash = Some(v);
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                    return false;
                }
            }
        }
        true
    }
    fn rename_key(&mut self, key: StyleKey) {
        let Some(old) = self.rule().map(|r| r.key.clone()) else {
            return;
        };
        let i = self.entry;
        if let Some(p) = self.current_mut() {
            if let Some(r) = p.entries.get_mut(i) {
                r.key = key.clone();
            }
            for a in &mut p.assignments {
                if a.key == old {
                    a.key = key.clone();
                }
            }
            if let (StyleKey::Named { name: old }, StyleKey::Named { name: new }) = (&old, &key)
                && p.default_style.as_ref() == Some(old)
            {
                p.default_style = Some(new.clone());
            }
        }
    }
}
impl Designer {
    pub(crate) fn plot_profiles_update(&mut self, m: PlotMessage) {
        match m {
            PlotMessage::Open => {
                let mut d = PlotDialog {
                    profiles: self.book.plot_profiles.clone(),
                    selected: (!self.book.plot_profiles.is_empty()).then_some(0),
                    entry: 0,
                    dash: String::new(),
                    error: None,
                };
                d.sync_dash();
                self.dialog = Some(Window::PlotProfiles(Box::new(d)));
            }
            PlotMessage::New(kind) => {
                let id = self.new_id("plot-profile");
                if let Some(Window::PlotProfiles(d)) = &mut self.dialog {
                    if !d.keep_dash() {
                        return;
                    }
                    let mut n = 1;
                    while d.profiles.iter().any(|p| p.name == format!("Baskı {n}")) {
                        n += 1;
                    }
                    let key = match kind {
                        TableKind::ColorDependent => StyleKey::Aci { index: 1 },
                        TableKind::Named => StyleKey::Named {
                            name: "Standart".into(),
                        },
                    };
                    d.profiles.push(PlotProfile {
                        id,
                        name: format!("Baskı {n}"),
                        kind,
                        entries: vec![StyleEntry {
                            key: key.clone(),
                            style: PlotStyle::default(),
                        }],
                        assignments: vec![],
                        default_style: match key {
                            StyleKey::Named { name } => Some(name),
                            _ => None,
                        },
                    });
                    d.selected = Some(d.profiles.len() - 1);
                    d.entry = 0;
                    d.sync_dash();
                    d.error = None;
                }
            }
            PlotMessage::Save(apply) => {
                let Some(Window::PlotProfiles(d)) = &mut self.dialog else {
                    return;
                };
                if !d.keep_dash() {
                    return;
                }
                let profiles = d.profiles.clone();
                let selected = d.current().cloned();
                let mut check = self.book.clone();
                check.plot_profiles = profiles.clone();
                if let Err(e) = kentos_sheet::validate::validate_book(&check) {
                    d.error = Some(e.message);
                    return;
                }
                let mut ops = vec![Op::SavePlotProfiles(SavePlotProfiles { profiles })];
                if apply && let (Some(profile), Some(sheet)) = (selected, self.open_sheet()) {
                    let mut settings = sheet
                        .plot
                        .as_ref()
                        .map(|p| p.settings.clone())
                        .unwrap_or_default();
                    settings.profile = Some(profile.id.clone());
                    settings.profile_snapshot = Some(profile);
                    ops.push(Op::SetSheetPlot(SetSheetPlot {
                        sheet: sheet.id.clone(),
                        plot: Some(AppliedPageSetup {
                            source: sheet.plot.as_ref().and_then(|p| p.source.clone()),
                            settings,
                        }),
                    }));
                }
                self.commit(ops);
                if self.error.is_none() {
                    self.dialog = None;
                } else if let Some(Window::PlotProfiles(d)) = &mut self.dialog {
                    d.error = self.error.clone();
                }
            }
            other => {
                let selected = self
                    .selection
                    .iter()
                    .filter(|id| {
                        self.book
                            .item(id)
                            .is_some_and(|i| !matches!(i.kind, kentos_sheet::ItemKind::Group(_)))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if let Some(Window::PlotProfiles(d)) = &mut self.dialog {
                    d.error = None;
                    match other {
                        PlotMessage::Select(i) => {
                            if !d.keep_dash() {
                                return;
                            }
                            d.selected = (i < d.profiles.len()).then_some(i);
                            d.entry = 0;
                            d.sync_dash();
                        }
                        PlotMessage::Remove => {
                            if let Some(i) = d.selected {
                                d.profiles.remove(i);
                                d.selected = (!d.profiles.is_empty())
                                    .then_some(i.min(d.profiles.len().saturating_sub(1)));
                                d.entry = 0;
                                d.sync_dash();
                            }
                        }
                        PlotMessage::Name(v) => {
                            if let Some(p) = d.current_mut() {
                                p.name = v;
                            }
                        }
                        PlotMessage::Entry(i) => {
                            if !d.keep_dash() {
                                return;
                            }
                            d.entry = i;
                            d.sync_dash();
                        }
                        PlotMessage::NewEntry => {
                            if !d.keep_dash() {
                                return;
                            }
                            if let Some(p) = d.current_mut() {
                                let key = match p.kind {
                                    TableKind::ColorDependent => match (1..=255).find(|n| {
                                        !p.entries
                                            .iter()
                                            .any(|e| e.key == StyleKey::Aci { index: *n })
                                    }) {
                                        Some(index) => StyleKey::Aci { index },
                                        None => {
                                            d.error = Some("255 rengin hepsi tanımlı.".into());
                                            return;
                                        }
                                    },
                                    TableKind::Named => {
                                        let mut n = 1;
                                        while p.entries.iter().any(|e| {
                                            e.key
                                                == StyleKey::Named {
                                                    name: format!("Kalem {n}"),
                                                }
                                        }) {
                                            n += 1;
                                        }
                                        StyleKey::Named {
                                            name: format!("Kalem {n}"),
                                        }
                                    }
                                };
                                p.entries.push(StyleEntry {
                                    key,
                                    style: PlotStyle::default(),
                                });
                                d.entry = p.entries.len() - 1;
                                d.sync_dash();
                            }
                        }
                        PlotMessage::RemoveEntry => {
                            let i = d.entry;
                            if let Some(p) = d.current_mut()
                                && i < p.entries.len()
                            {
                                let e = p.entries.remove(i);
                                p.assignments.retain(|a| a.key != e.key);
                                if let StyleKey::Named { name } = &e.key
                                    && p.default_style.as_ref() == Some(name)
                                {
                                    p.default_style = None;
                                }
                                d.entry = i.min(p.entries.len().saturating_sub(1));
                                d.sync_dash();
                            }
                        }
                        PlotMessage::Key(v) => d.rename_key(v),
                        PlotMessage::Default(on) => {
                            let key = d.rule().map(|e| e.key.clone());
                            if let (Some(p), Some(StyleKey::Named { name })) =
                                (d.current_mut(), key)
                            {
                                p.default_style = on.then_some(name);
                            }
                        }
                        PlotMessage::ColorMode(mode) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.color = match mode {
                                    ColorMode::Object => PlotColor::Object,
                                    ColorMode::Black => PlotColor::Black,
                                    ColorMode::Gray => PlotColor::Grayscale,
                                    ColorMode::Color => match &r.style.color {
                                        PlotColor::Color { value } => PlotColor::Color {
                                            value: value.clone(),
                                        },
                                        _ => PlotColor::Color {
                                            value: "#000000".into(),
                                        },
                                    },
                                };
                            }
                        }
                        PlotMessage::Color(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.color = PlotColor::Color { value: v };
                            }
                        }
                        PlotMessage::Screening(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.screening = v.round().clamp(0., 100.) as u8;
                            }
                        }
                        PlotMessage::Weight(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.line_weight = v.then_some(250);
                            }
                        }
                        PlotMessage::WeightMm(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.line_weight = Some((v * 1000.).round() as i32);
                            }
                        }
                        PlotMessage::Dash(v) => {
                            let value = if v {
                                Some(d.dash_values().unwrap_or_else(|_| vec![2000, 1000]))
                            } else {
                                None
                            };
                            if let Some(r) = d.rule_mut() {
                                r.style.dash = value;
                            }
                        }
                        PlotMessage::DashText(v) => d.dash = v,
                        PlotMessage::Opacity(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.opacity = v.then_some(100);
                            }
                        }
                        PlotMessage::Alpha(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.opacity = Some(v.round().clamp(0., 100.) as u8);
                            }
                        }
                        PlotMessage::Cap(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.cap = v;
                            }
                        }
                        PlotMessage::Join(v) => {
                            if let Some(r) = d.rule_mut() {
                                r.style.join = v;
                            }
                        }
                        PlotMessage::AssignLayer(id) => {
                            let key = d.rule().map(|r| r.key.clone());
                            if let (Some(p), Some(key)) = (d.current_mut(), key) {
                                let target = Target::ModelLayer { id };
                                p.assignments.retain(|a| a.target != target);
                                p.assignments.push(Assignment { target, key });
                            }
                        }
                        PlotMessage::AssignSelected => {
                            let key = d.rule().map(|r| r.key.clone());
                            if let (Some(p), Some(key)) = (d.current_mut(), key) {
                                for id in &selected {
                                    let target = Target::PaperItem { id: id.clone() };
                                    p.assignments.retain(|a| a.target != target);
                                    p.assignments.push(Assignment {
                                        target,
                                        key: key.clone(),
                                    });
                                }
                            }
                        }
                        PlotMessage::Unassign(t) => {
                            if let Some(p) = d.current_mut() {
                                p.assignments.retain(|a| a.target != t);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
