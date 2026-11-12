//! Ortofoto doğruluğu (docs/adr/1480; `raster.accuracy`): the selected orthophoto checked against the points measured
//! on the ground and the same points read on the image, by `cad.orthophoto.check`, read only. The layers, the name
//! attribute and the scale on two lines; the summary says KOH_X,Y and KOH_Z against BÖHHBÜY's limits (the rule
//! package's), the ground sampling distance against Article 54's, the fit points apart, and a KOH under half a pixel;
//! the table lists every point's error (centimetres). CSV kaydet writes them.

use crate::map;
use std::path::PathBuf;

use iced::widget::{Column, container, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, Entity, OrthophotoCheck, OrthophotoChecked};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, orthophoto_check};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{col, num, sheet};
use crate::style::fields;
use crate::traces::Control;

pub const TITLE: &str = "Ortofoto doğruluğu";
const CLOSE: &str = "Kapat";
const SAVE: &str = "CSV kaydet…";

#[derive(Clone, Debug, Default)]
pub struct State {
    pub raster: String,
    /// The drawing's layers: id and name.
    pub layers: Vec<(String, String)>,
    pub known: String,
    pub observed: String,
    pub field: String,
    pub scale: f64,
    pub answer: Option<Result<OrthophotoChecked, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Known(String),
    Observed(String),
    Field(String),
    Scale(f64),
    Save,
    Saved(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Map(map::Event::Ortho(e))
}

fn cm(v: f64) -> String {
    format!("{:.1}", v * 100.0)
}

fn role_word(r: &str) -> &'static str {
    if r == "fit" { "Uyum" } else { "Denetim" }
}

/// The points as CSV (metres as computed).
pub fn csv(r: &OrthophotoChecked) -> String {
    let mut out = format!(
        "Kural paketi;{};Ölçek;1/{};YÖA (m);{};KOH_X,Y (m);{};Sınır (m);{}\n\nNokta;Rol;dx (m);dy (m);dz (m);Yatay hata (m)\n",
        r.rules, r.scale, r.gsd, r.check.koh_xy, r.limit_xy
    );
    for p in &r.points {
        out.push_str(&format!(
            "{};{};{};{};{};{}\n",
            p.name,
            role_word(&p.role),
            p.dx,
            p.dy,
            p.dz.map(|z| z.to_string()).unwrap_or_default(),
            p.d
        ));
    }
    out
}

impl App {
    fn ortho_run(&mut self) {
        let s = &self.map.ortho;
        let input = OrthophotoCheck {
            raster: s.raster.clone(),
            known: s.known.clone(),
            observed: s.observed.clone(),
            field: Some(s.field.clone()),
            scale: Some(s.scale.max(1.0) as u32),
            rules: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        self.map.ortho.answer = Some(
            match orthophoto_check::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Denetlenemedi.".to_owned()),
            },
        );
    }

    /// Ortofoto doğruluğu (`raster.accuracy`): the one selected raster; the layers guessed from their names (a GNSS
    /// layer, a layer read on the image), else the first two point layers.
    pub(crate) fn open_ortho(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let picked = match self.selection.ids() {
            [one] => match doc.model.get(*one) {
                Some(Entity::Raster(_)) => doc.model.uid(*one).map(|u| u.to_string()),
                _ => None,
            },
            _ => None,
        };
        let Some(raster) = picked else {
            self.warn("Ortofoto doğruluğu: önce denetlenecek ortofotoyu (bir raster) seçin.");
            return Task::none();
        };
        let layers: Vec<(String, String)> =
            doc.model.layers().leaves().iter().map(|n| (n.id.clone(), n.name.clone())).collect();
        let with_points: Vec<&(String, String)> = layers
            .iter()
            .filter(|(id, _)| doc.model.entities().any(|e| e.base().layer_id == *id && matches!(e, Entity::Point(_))))
            .collect();
        let find = |words: &[&str]| {
            with_points
                .iter()
                .find(|(_, n)| words.iter().any(|w| n.to_lowercase().contains(w)))
                .map(|(id, _)| id.clone())
        };
        let known = find(&["gnss", "arazi", "denetim"]).or_else(|| with_points.first().map(|(id, _)| id.clone())).unwrap_or_default();
        let observed = find(&["okun", "ortofoto", "görüntü"])
            .filter(|o| *o != known)
            .or_else(|| with_points.iter().map(|(id, _)| id.clone()).find(|id| *id != known))
            .unwrap_or_default();
        self.map.ortho = State {
            raster,
            layers,
            known,
            observed,
            field: "Ad".into(),
            scale: doc.model.settings().plot_scale.round().max(1.0),
            answer: None,
        };
        self.ortho_run();
        self.dialog = Some(Dialog::Map(map::MapDialogKind::Ortho));
        Task::none()
    }

    /// The points written to `path` as CSV (a test or a trace gives it here, the picker in the app).
    pub(crate) fn ortho_save(&mut self, path: PathBuf) {
        let Some(Ok(r)) = &self.map.ortho.answer else {
            return;
        };
        match std::fs::write(&path, csv(r)) {
            Ok(()) => self.say(Level::Success, format!("{TITLE}: {} nokta {} dosyasına yazıldı.", r.points.len(), path.display())),
            Err(e) => self.warn(format!("{TITLE}: {} yazılamadı: {e}.", path.display())),
        }
    }

    pub(crate) fn ortho_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Known(v) => self.map.ortho.known = v,
            Event::Observed(v) => self.map.ortho.observed = v,
            Event::Field(v) => self.map.ortho.field = v,
            Event::Scale(v) => self.map.ortho.scale = v,
            Event::Save => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title(TITLE)
                            .set_file_name("ortofoto-dogrulugu.csv")
                            .add_filter("CSV", &["csv"])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => {
                self.ortho_save(path);
                return Task::none();
            }
            Event::Saved(None) => return Task::none(),
            Event::Close => {
                self.map.ortho = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.ortho_run();
        Task::none()
    }

    pub(crate) fn ortho_view(&self) -> Element<'_, Message> {
        let s = &self.map.ortho;
        let form = Form::new()
            .label_width(130.0)
            .field(
                "Arazi noktaları",
                row![
                    container(fields::select_owned(s.layers.clone(), &s.known, |v| msg(Event::Known(v)))).width(230),
                    label::caption("Görüntüde okunan"),
                    container(fields::select_owned(s.layers.clone(), &s.observed, |v| msg(Event::Observed(v)))).width(230)
                ]
                .spacing(10)
                .align_y(Center),
            )
            .field(
                "Nokta adı alanı",
                row![
                    container(fields::input(None, "Ad", &s.field, false, false).on_input(|v| msg(Event::Field(v)))).width(120),
                    label::caption("Ölçek 1/"),
                    NumberInput::new(s.scale, |v| msg(Event::Scale(v))).step(500.0).decimals(0).width(110)
                ]
                .spacing(10)
                .align_y(Center),
            );
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        match &s.answer {
            Some(Ok(r)) => {
                let c = &r.check;
                lines.push(match r.passed_xy {
                    Some(ok) => words::text_line(
                        if ok { Kind::Ok } else { Kind::Error },
                        format!(
                            "{} denetim noktası: KOH_X,Y {} cm, sınır {} cm (BÖHHBÜY m. 91, 1/{}): {}. Ortalama hata doğu {} cm, kuzey {} cm; en büyük {} cm.",
                            c.n,
                            cm(c.koh_xy),
                            cm(r.limit_xy),
                            r.scale,
                            if ok { "sınırın içinde" } else { "sınırın dışında" },
                            cm(c.mean_x),
                            cm(c.mean_y),
                            cm(c.max_d)
                        ),
                    ),
                    None => words::text_line(Kind::Warn, "Bağımsız denetim noktası yok: doğruluk söylenemez."),
                });
                if let (Some(kz), Some(ok)) = (c.koh_z, r.passed_z) {
                    lines.push(words::text_line(
                        if ok { Kind::Ok } else { Kind::Error },
                        format!("KOH_Z {} cm ({} nokta), sınır {} cm.", cm(kz), c.n_z, cm(r.limit_z)),
                    ));
                }
                lines.push(match (r.gsd_limit, r.gsd_ok) {
                    (Some(l), Some(ok)) => words::text_line(
                        if ok { Kind::Ok } else { Kind::Warn },
                        format!("Yer örnekleme aralığı {} cm; 1/{} için en çok {} cm (m. 54, ±%10).", cm(r.gsd), r.scale, cm(l)),
                    ),
                    _ => words::text_line(Kind::Info, format!("Yer örnekleme aralığı {} cm.", cm(r.gsd))),
                });
                if let Some(f) = &r.fit {
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("{} uyum noktası (oturtmanın kontrol noktaları) ayrı: KOH_X,Y {} cm; doğruluğa sayılmaz.", f.n, cm(f.koh_xy)),
                    ));
                }
                if r.below_pixel {
                    lines.push(words::text_line(
                        Kind::Warn,
                        "KOH_X,Y yarım pikselden küçük: görüntüyü okuma inceliğinin altında, sonuç iyimser olabilir.",
                    ));
                }
                if !r.unmatched.is_empty() {
                    lines.push(words::text_line(Kind::Warn, format!("Eşi bulunmayan noktalar: {}.", r.unmatched.join(", "))));
                }
                for p in &r.points {
                    rows.push(vec![
                        p.name.clone(),
                        role_word(&p.role).to_owned(),
                        cm(p.dx),
                        cm(p.dy),
                        p.dz.map(cm).unwrap_or_default(),
                        cm(p.d),
                    ]);
                }
            }
            Some(Err(why)) => lines.push(words::text_line(Kind::Error, why.clone())),
            None => {}
        }
        let table = sheet(
            vec![col("Nokta", 100.0), col("Rol", 90.0), num("dx (cm)", 90.0), num("dy (cm)", 90.0), num("dz (cm)", 90.0), num("Yatay (cm)", 100.0)],
            rows,
            "Eşleşen nokta yok.",
            260.0,
        );
        let body = Column::new().spacing(12).push(form).push(words::summary(lines)).push(table);
        let ready = matches!(&s.answer, Some(Ok(r)) if !r.points.is_empty());
        Frame::new(TITLE)
            .push(body)
            .aside(words::secondary(SAVE, ready.then(|| msg(Event::Save))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(820.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn ortho_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
