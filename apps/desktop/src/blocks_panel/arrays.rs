//! Read-only array search in the dock, explicit review and cancellable metadata-only adoption.
use crate::{
    app::{App, Dialog, Message},
    exchange::words::{self, Kind},
    traces::Control,
};
use iced::{
    Element, Fill, Subscription, Task,
    widget::{button, column, container, row, scrollable, text},
};
use kentos_commands::{
    ArrayFindKind, ArrayItem, ArrayLayout, ArraysAdopt, ArraysAdoptPlan, ArraysAdopted, ArraysFind,
    ArraysFound, CallEnvelope, CallOp, CommandResult, Entity,
};
use kentos_native_application::jobs::Job;
use kentos_ui::{
    label, style,
    theme::typography,
    widget::{Dialog as Frame, NumberInput, VirtualList, number::units, select::{Choice as SelectChoice, Select}},
};
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

pub(crate) const TITLE: &str = "Dizi örüntülerini bul";
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    uid: String,
    label: String,
}
impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.label.fmt(f)
    }
}
struct Review {
    index: usize,
    input: ArraysAdopt,
    plan: Option<ArraysAdoptPlan>,
    choices: Vec<Choice>,
    choice: Option<Choice>,
    place: f64,
    keep_options: bool,
}
pub(super) struct Window {
    source: Option<String>,
    name: String,
    session: u64,
    generation: u64,
    revision: String,
    hidden: bool,
    tolerance: f64,
    kind: ArrayFindKind,
    path: Option<Choice>,
    paths: Vec<Choice>,
    previous: Option<(bool, f64, ArrayFindKind, Option<Choice>)>,
    token: u64,
    cancel: Arc<AtomicBool>,
    busy: bool,
    output: Option<ArraysFound>,
    warnings: Vec<String>,
    refused: Option<String>,
    review: Option<Review>,
    job: Option<Job>,
}
impl std::fmt::Debug for Window {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArraySearch")
            .field("source", &self.source)
            .field("busy", &self.busy)
            .finish()
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(j) = &mut self.job {
            j.cancel();
        }
    }
}
impl Window {
    fn current(&self, app: &App) -> bool {
        app.document
            .as_ref()
            .is_some_and(|d| d.session == self.session && d.model.generation() == self.generation)
    }
    fn input(&self, doc: &kentos_domain::Document) -> Option<ArraysFind> {
        if !self.tolerance.is_finite() || self.tolerance <= 0. || self.tolerance > 0.001 {
            return None;
        }
        let source = self.source.as_ref()?;
        let path = if self.kind == ArrayFindKind::Path {
            Some(self.path.as_ref()?.uid.clone())
        } else { None };
        let Entity::Insert(seed) = doc.get(doc.slot_of(source.parse().ok()?)?)? else {
            return None;
        };
        let uids = doc
            .entities_with_uids()
            .filter(|(_, e)| {
                matches!(e,Entity::Insert(i) if i.block==seed.block)
                    && (self.hidden
                        || (doc.layers().is_visible(&e.base().layer_id)
                            && !doc.layers().is_frozen(&e.base().layer_id)))
            })
            .take(kentos_commands::ARRAY_FIND_SCOPE_MAX + 1)
            .map(|(u, _)| u.to_string())
            .collect();
        Some(ArraysFind {
            source: source.clone(),
            kind: (self.kind != ArrayFindKind::Line).then_some(self.kind),
            path,
            uids: Some(uids),
            tolerance: Some(self.tolerance),
            limit: Some(100),
            expected_revision: Some(self.revision.clone()),
        })
    }
    fn stop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(j) = &mut self.job {
            j.cancel();
        }
        self.job = None;
        self.busy = false;
        self.token = NEXT.fetch_add(1, Ordering::Relaxed);
    }
    fn fresh_work(&mut self) -> (u64, Arc<AtomicBool>) {
        self.stop();
        self.cancel = Arc::new(AtomicBool::new(false));
        self.busy = true;
        self.refused = None;
        (self.token, Arc::clone(&self.cancel))
    }
}

fn places(items: &[ArrayItem], limit: usize) -> String {
    let mut s = items
        .iter()
        .take(limit)
        .map(|i| {
            if i.kept {
                "◇"
            } else if i.uid.is_some() {
                "●"
            } else {
                "○"
            }
        })
        .collect::<Vec<_>>()
        .join("  ");
    if items.len() > limit {
        s.push_str("  …");
    }
    s
}
fn spacing(layout: &ArrayLayout) -> f64 {
    match *layout {
        ArrayLayout::Line { count, dx, dy, fit } => {
            dx.hypot(dy)
                / if fit {
                    count.saturating_sub(1).max(1) as f64
                } else {
                    1.
                }
        }
        _ => 0.,
    }
}
fn kind_name(kind: ArrayFindKind) -> &'static str {
    match kind {
        ArrayFindKind::Line => "Doğrusal",
        ArrayFindKind::Grid => "Dikdörtgen (X/Y)",
        ArrayFindKind::Lattice => "Dönük/eğik ızgara",
        ArrayFindKind::Polar => "Kutupsal (tam çember)",
        ArrayFindKind::Path => "Yol boyunca (sabit aralık)",
    }
}
fn layout_text(layout: &ArrayLayout, f: &kentos_interaction::Format) -> String {
    match *layout {
        ArrayLayout::Grid { rows, cols, dx, dy } => format!(
            "{rows} satır × {cols} sütun · Sütun {} · Satır {}",
            f.length(dx.abs()),
            f.length(dy.abs())
        ),
        ArrayLayout::Lattice {rows,cols,dx,dy,column_angle,row_angle} => format!(
            "{rows} satır × {cols} sütun\nSütun {} / {} · Satır {} / {}\nDünya doğusu 0°; öğelerin kendi yönü korunur.",
            f.length(dx.abs()),f.bearing(column_angle*400./360.),f.length(dy.abs()),f.bearing(row_angle*400./360.)
        ),
        ArrayLayout::Polar { center, count, fill, rotate } => format!(
            "{} yer · {} · Açısal aralık {}\nMerkez (dünya X/Y): {} / {}\n{}",
            count, f.bearing(fill * 400. / 360.), f.bearing(400. / count.max(1) as f64),
            f.length(center.x), f.length(center.y),
            if rotate { "Öğeler merkez etrafında döner" } else { "Öğelerin yönü korunur" }
        ),
        ArrayLayout::Path { count, spacing, align, .. } => format!(
            "{count} yer · Sabit aralık {}\n{} · Kılavuzun kayıtlı başlangıcından ve yönünde",
            f.length(spacing.unwrap_or(0.)),
            if align { "Öğeler teğeti izler" } else { "Öğelerin yönü korunur" }
        ),
        _ => format!("Aralık: {}", f.length(spacing(layout))),
    }
}
fn place_schema(layout: &ArrayLayout, items: &[ArrayItem]) -> String {
    if let ArrayLayout::Grid { cols, .. } | ArrayLayout::Lattice { cols, .. } = *layout {
        let mut rows = items
            .chunks(cols.max(1) as usize)
            .take(8)
            .map(|r| places(r, 8))
            .collect::<Vec<_>>();
        if items.len() / cols.max(1) as usize > 8 {
            rows.push("…".into());
        }
        rows.join("\n")
    } else if matches!(layout, ArrayLayout::Polar { .. }) {
        format!("↺ {}\nKaynak 1; yerler saat yönünün tersine ilerler.", places(items, 18))
    } else if matches!(layout, ArrayLayout::Path { .. }) {
        format!("→ {}\nKaynak 1; yerler kılavuzun kayıtlı yönünde ilerler.", places(items, 18))
    } else {
        places(items, 18)
    }
}
fn keep_position(r: &Review) -> Element<'_, Message> {
    if let ArrayLayout::Grid { rows, cols, .. } | ArrayLayout::Lattice { rows, cols, .. } = r.input.layout {
        let index = (r.place - 1.).max(0.) as u32;
        column![
            row![
                words::field(
                    "Satır",
                    NumberInput::new((index / cols + 1) as f64, |n| msg(Event::KeepRow(n)))
                        .range(1.0..=rows as f64)
                        .step(1.)
                        .decimals(0),
                    None
                ),
                words::field(
                    "Sütun",
                    NumberInput::new((index % cols + 1) as f64, |n| msg(Event::KeepCol(n)))
                        .range(1.0..=cols as f64)
                        .step(1.)
                        .decimals(0),
                    None
                ),
            ]
            .spacing(8),
            label::caption(
                "Satır ve sütunlar kaynak köşeden 1 ile başlar. Yalnız boş yere atama yapılır."
            )
        ]
        .spacing(6)
        .into()
    } else {
        words::field(
            "Dizideki yer",
            NumberInput::new(r.place, |n| msg(Event::KeepPlace(n)))
                .range(1.0..=r.input.items.len() as f64)
                .step(1.)
                .decimals(0),
            Some(if matches!(r.input.layout, ArrayLayout::Path { .. }) {
                "Kaynak 1'dir; yerler kılavuzun kayıtlı yönünde ilerler. Dolu yere yazılmaz."
            } else if matches!(r.input.layout, ArrayLayout::Polar { .. }) {
                "Kaynak 1'dir; yerler saat yönünün tersine ilerler. Dolu yere yazılmaz."
            } else {
                "Yerler soldan/ilk uçtan 1 ile başlar. Dolu yerin üzerine yazılmaz."
            }.into()),
        )
    }
}
fn available_choices(r: &Review) -> Vec<Choice> {
    let used: HashSet<_> = r
        .input
        .items
        .iter()
        .filter_map(|i| i.uid.map(|u| u.to_text()))
        .collect();
    r.choices
        .iter()
        .filter(|c| !used.contains(&c.uid))
        .cloned()
        .collect()
}
impl App {
    pub(crate) fn array_find_title(&self) -> &'static str {
        if self
            .blocks_panel
            .arrays
            .as_ref()
            .is_some_and(|w| w.review.is_some())
        {
            "Diziye bağlamayı incele"
        } else {
            TITLE
        }
    }
    pub(crate) fn array_find_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.blocks_panel.arrays else {
            return text("").into();
        };
        let current = w.current(self);
        let f = self.format();
        let mut body = column![label::body(format!("Kaynak: {}", w.name))].spacing(12);
        if let Some(r) = &w.review {
            let present = r.input.items.iter().filter(|i| i.uid.is_some()).count();
            let kept = r.input.items.iter().filter(|i| i.kept).count();
            let missing = r.input.items.len() - present;
            body = body.push(
                container(
                    column![
                        label::body(format!(
                            "{} yer · {present} mevcut blok · {missing} boş · {kept} korunan",
                            r.input.items.len()
                        )),
                        label::body(layout_text(&r.input.layout, &f)),
                        label::body(place_schema(&r.input.layout, &r.input.items)),
                        label::caption("Yerleşim şeması: ● Mevcut · ○ Boş · ◇ Korunan")
                    ]
                    .spacing(8),
                )
                .padding(12)
                .style(style::container::tile),
            );
            body=body.push(words::text_line(Kind::Info,"Bloklar taşınmaz veya yeniden oluşturulmaz. Boş yerler boş kalır; mevcut ölçü bağları ve proje birimi korunur."));
            if let ArrayLayout::Path { path, .. } = &r.input.layout
                && let Some(choice) = w.paths.iter().find(|c| &c.uid == path) {
                    body = body.push(label::caption(format!("Kılavuz: {}. Yol düzenlenince normal öğeler aynı kimliklerle izler.", choice.label)));
            }
            let choices = available_choices(r);
            if !r.choices.is_empty() {
                body = body.push(label::caption(format!(
                    "{} eşleşmeyen blok dışarıda kalır; çizimden silinmez.",
                    choices.len()
                )));
                body = body.push(words::secondary(
                    if r.keep_options {
                        "Korunan öğe ayarlarını gizle"
                    } else {
                        "Elle taşınmış öğeyi koru…"
                    },
                    (!w.busy).then(|| msg(Event::KeepOptions)),
                ));
            }
            if r.keep_options && !choices.is_empty() && !w.busy {
                let picked = r.choice.as_ref().and_then(|c| choices.iter().position(|v| v == c));
                let options = choices.iter().map(|c| SelectChoice::new(c.label.clone())).collect::<Vec<_>>();
                let select = Select::new(options, picked, move |i| msg(Event::KeepObject(choices[i].clone())));
                body=body.push(words::field("Eşleşmeyen blok",container(select).width(Fill),Some("Bu bloğu yalnız seçtiğiniz boş yere, mevcut konumunu koruyarak bağlayın.".into())))
                    .push(keep_position(r))
                    .push(row![words::secondary("Bloğu göster",r.choice.as_ref().map(|_|msg(Event::ShowKept))),words::secondary("Bu yere koru",r.choice.as_ref().map(|_|msg(Event::Assign)))].spacing(8));
            }
            for (k, item) in r.input.items.iter().enumerate().filter(|(_, i)| i.kept) {
                let name = item
                    .uid
                    .and_then(|u| r.choices.iter().find(|c| c.uid == u.to_text()))
                    .map_or("Mevcut blok", |c| c.label.as_str());
                body = body.push(
                    row![
                        label::caption(format!("Yer {} · {name}", k + 1)).width(Fill),
                        button(label::body(format!("Yer {} atamasını kaldır", k + 1)))
                            .on_press_maybe((!w.busy).then(|| msg(Event::Unassign(k))))
                            .padding(kentos_ui::theme::metrics::action())
                            .style(style::button::secondary)
                    ]
                    .spacing(8),
                );
            }
            if let Some(plan) = &r.plan {
                if let ArrayLayout::Polar { center, .. } = plan.array.layout {
                    body = body.push(label::caption(format!(
                        "Yarıçap (blok taban noktasından): {}",
                        f.length((plan.array.p.x - center.x).hypot(plan.array.p.y - center.y))
                    )));
                }
                body = body.push(label::caption(format!(
                    "Denetlenen en büyük sapma: {}. Bağlama tek geri alma adımıdır.",
                    f.length(plan.deviation)
                )));
            }
            if w.busy {
                body = body.push(label::body(if w.job.is_some() {
                    "Dizi bağlanıyor…"
                } else {
                    "Öğe yerleri ve bağlama koşulları inceleniyor…"
                }));
            }
        } else {
            if w.source.is_none() {
                body=body.push(words::text_line(Kind::Warn,"Çizimde tek bir blok yerleştirmesi seçip yeniden açın. Bir bloğun tanımını değil, çizimdeki örneğini seçin."));
            }
            let kinds = [ArrayFindKind::Line, ArrayFindKind::Grid, ArrayFindKind::Lattice, ArrayFindKind::Polar, ArrayFindKind::Path];
            let chosen = kinds.iter().position(|k| *k == w.kind);
            let kind_select = Select::new(kinds.iter().map(|k| SelectChoice::new(kind_name(*k))), chosen, move |i| msg(Event::PatternKind(kinds[i])));
            body=body.push(label::body("Aynı bloğun görünür yerleştirmelerinde seçilen düzen aranır. Sonuçlar çizimi değiştirmeden Bloklar panelinde açılır."))
                .push(words::field("Düzen türü", container(kind_select).width(Fill), Some("Doğrusal ve yol en az üç; dikdörtgen ve kutupsal en az dört mevcut öğe gerektirir.".into())))
                .push(words::field("Tolerans (mm)",NumberInput::new(w.tolerance,|n|msg(Event::Tolerance(n))).units(units::MILLIMETRE).range(0.0..=0.001).step(0.001).decimals(3),Some("Sıfırdan büyük, en çok 1 mm. Varsayılan 0,001 mm; çizim ölçeğini değiştirmez.".into())))
                .push(words::check(w.hidden,"Gizli ve donmuş katmanları da ara",Some(msg(Event::Hidden))))
                .push(label::caption(match w.kind {
                    ArrayFindKind::Line => "Gözlenen komşu aralıklar aranır. Eksik yerler doldurulmaz; başka tanım, ölçek veya öğe yönü birleştirilmez.",
                    ArrayFindKind::Grid => "Dikdörtgen arama dünya X/Y eksenlerindedir; dönük/eğik gridler bulunmaz. Bir köşe dolu olmalıdır. Dönük blok varyantları ve başka tanımlar birleştirilmez.",
                    ArrayFindKind::Lattice => "Gözlenen iki bağımsız eksen aranır; yönler ve aralıklar incelemede gösterilir. Öğelerin kendi yönü değişmez; dolu bir köşe ve iki yönde gerçek komşular gerekir.",
                    ArrayFindKind::Polar => "Tam çember yorumudur; kısmi yay niyeti çıkarılmaz. Öğe yönleri ve gözlenen açısal aralık doğrulanır. Eksik yerler boş kalır; kaynak 1'den saat yönünün tersine ilerlenir.",
                    ArrayFindKind::Path => "Kılavuzun kayıtlı yönü kullanılır; kaynak başlangıçta olmalıdır. Gözlenen eşit aralık korunur; son dolu öğeden ötesi varsayılmaz.",
                }));
            if w.kind == ArrayFindKind::Path {
                let paths = w.paths.clone();
                let picked = w.path.as_ref().and_then(|p| paths.iter().position(|v| v == p));
                let choices = paths.iter().map(|p| SelectChoice::new(p.label.clone())).collect::<Vec<_>>();
                let path_select = Select::new(choices, picked, move |i| msg(Event::Path(paths[i].clone())));
                body = body.push(words::field("Kılavuz yol", container(path_select).width(Fill),
                    Some("Kaynak bloğu ve bir kılavuzu birlikte seçip açabilirsiniz. Başlangıçtaki blok kaynak olmalı; yön değiştirilmez. İlk 2048 görünür kılavuz listelenir. Kılavuz çizimde kalır.".into())));
            }
        }
        let mut frame = Frame::new(self.array_find_title()).scroll(body);
        // Validation feedback must remain visible while the optional keep form scrolls.
        if !current {
            frame = frame.push(words::text_line(
                Kind::Warn,
                "Çizim değişti; eski aday uygulanamaz. Kapatıp yeniden arayın.",
            ));
        }
        if let Some(why) = &w.refused {
            frame = frame.push(words::text_line(Kind::Warn, why.clone()));
        }
        frame = frame.action(words::secondary("Vazgeç", Some(msg(Event::CloseDialog))));
        if w.busy {
            frame = frame.action(words::secondary("Durdur", Some(msg(Event::Stop))));
        } else if let Some(r) = &w.review {
            if r.plan.is_none() || w.refused.is_some() {
                frame = frame.action(words::secondary(
                    "Tekrar incele",
                    current.then(|| msg(Event::Refresh)),
                ));
            }
            frame = frame.action(words::primary(
                "Diziye bağla",
                (current && r.plan.is_some() && w.refused.is_none()).then(|| msg(Event::Apply)),
            ));
        } else {
            frame = frame.action(words::primary(
                "Bul",
                (current
                    && w.source.is_some()
                    && (w.kind != ArrayFindKind::Path || w.path.is_some())
                    && w.tolerance.is_finite()
                    && w.tolerance > 0.
                    && w.tolerance <= 0.001)
                    .then(|| msg(Event::Run)),
            ));
        }
        frame
            .width(620.)
            .max_height(700.)
            .modal(msg(Event::CloseDialog))
    }
    pub(crate) fn array_find_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.blocks_panel.arrays else {
            return Ok(None);
        };
        let event = match control {
            Control::Press("Vazgeç") => Event::CloseDialog,
            Control::Press(name)
                if name.starts_with("Yer ") && name.ends_with(" atamasını kaldır") =>
            {
                let Some(place) = name
                    .strip_prefix("Yer ")
                    .and_then(|s| s.strip_suffix(" atamasını kaldır"))
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|n| n.checked_sub(1))
                else {
                    return Err("Yer numarası okunamadı".into());
                };
                if w.busy
                    || !w.current(self)
                    || !w
                        .review
                        .as_ref()
                        .is_some_and(|r| r.input.items.get(place).is_some_and(|i| i.kept))
                {
                    return Ok(None);
                }
                Event::Unassign(place)
            }
            Control::Press("Durdur") => Event::Stop,
            Control::Press("Tekrar incele") if !w.busy && w.current(self) => Event::Refresh,
            Control::Press("Bul")
                if !w.busy
                    && w.current(self)
                    && w.source.is_some()
                    && w.tolerance.is_finite()
                    && w.tolerance > 0.
                    && w.tolerance <= 0.001 =>
            {
                Event::Run
            }
            Control::Press("Bul") => return Ok(None),
            Control::Press("Diziye bağla")
                if !w.busy
                    && w.current(self)
                    && w.refused.is_none()
                    && w.review.as_ref().is_some_and(|r| r.plan.is_some()) =>
            {
                Event::Apply
            }
            Control::Press("Diziye bağla") => return Ok(None),
            Control::Press("Elle taşınmış öğeyi koru…" | "Korunan öğe ayarlarını gizle") => {
                Event::KeepOptions
            }
            Control::Press("Bu yere koru") => Event::Assign,
            Control::Press("Bloğu göster") => Event::ShowKept,
            Control::Pick("Düzen türü", name) => Event::PatternKind(match name {
                "Doğrusal" => ArrayFindKind::Line,
                "Dikdörtgen (X/Y)" => ArrayFindKind::Grid, "Dönük/eğik ızgara" => ArrayFindKind::Lattice,
                "Kutupsal (tam çember)" => ArrayFindKind::Polar,
                "Yol boyunca (sabit aralık)" => ArrayFindKind::Path,
                _ => return Err("Düzen türü listede yok".into()),
            }),
            Control::Pick("Kılavuz yol", name) => Event::Path(w.paths.iter().find(|p|p.label==name).cloned().ok_or("Kılavuz listede yok")?),
            Control::Fill("Tolerans (mm)", s) => Event::Tolerance(
                kentos_ui::widget::number::evaluate(s, units::MILLIMETRE)
                    .map_err(|e| e.to_string())?,
            ),
            Control::Fill("Satır", s) => Event::KeepRow(s.parse().map_err(|_| "Satır okunamadı")?),
            Control::Fill("Sütun", s) => Event::KeepCol(s.parse().map_err(|_| "Sütun okunamadı")?),
            Control::Fill("Dizideki yer", s) => {
                Event::KeepPlace(s.parse().map_err(|_| "Yer numarası okunamadı")?)
            }
            Control::Check("Gizli ve donmuş katmanları da ara", on) => {
                if w.hidden == on {
                    return Ok(None);
                } else {
                    Event::Hidden
                }
            }
            Control::Pick("Eşleşmeyen blok", name) => {
                let Some(choice) = w
                    .review
                    .as_ref()
                    .and_then(|r| available_choices(r).into_iter().find(|c| c.label == name))
                else {
                    return Err("Eşleşmeyen blok listede yok".into());
                };
                Event::KeepObject(choice)
            }
            _ => return Err(format!("{TITLE}: {control} yok")),
        };
        Ok(Some(msg(event)))
    }
}

pub(super) fn panel(app: &App) -> Element<'_, Message> {
    iced::widget::responsive(move |size| panel_height(app, size.height)).into()
}
fn panel_height(app: &App, height: f32) -> Element<'_, Message> {
    let Some(w) = &app.blocks_panel.arrays else {
        return text("").into();
    };
    let current = w.current(app);
    let f = app.format();
    // The default split dock has room for one summary line above a candidate.
    // Source details belong to the settings/review window; Back is already in the footer.
    let heading = w.output.as_ref().map_or_else(
        || "Dizi örüntüleri".to_owned(),
        |o| {
            if o.truncated {
                format!("Kısmi sonuç: {} aday", o.matches.len())
            } else {
                format!("{} aday · {} uygun blok", o.matches.len(), o.eligible.len())
            }
        },
    );
    let mut top = column![label::caption(heading)].spacing(6);
    if !current {
        top = top.push(words::text_line(
            Kind::Warn,
            "Çizim değişti. Yeni kaynak seçip aramayı yeniden açın.",
        ));
    }
    if let Some(why) = &w.refused {
        top = top.push(words::text_line(Kind::Warn, why.clone()));
    }
    for why in &w.warnings {
        top = top.push(words::text_line(Kind::Warn, why.clone()));
    }
    if w.busy {
        return container(
            column![
                scrollable(top.push(label::body(
                    "Örüntüler aranıyor… Çizimi kullanmaya devam edebilirsiniz."
                )))
                .height(Fill),
                words::secondary("Durdur", Some(msg(Event::Stop)))
            ]
            .spacing(8),
        )
        .padding(8)
        .height(Fill)
        .into();
    }
    let Some(output) = &w.output else {
        return container(
            column![
                scrollable(top).height(Fill),
                row![
                    words::secondary("Yeni kaynak", Some(msg(Event::Back))),
                    words::secondary("Ayarlar", current.then(||msg(Event::Settings)))
                ].spacing(6)
            ]
            .spacing(8),
        )
        .padding(8)
        .height(Fill)
        .into();
    };
    if output.matches.is_empty() {
        top=top.push(label::body("Seçilen düzene uyan aday bulunamadı. Ayarlardaki kapsam ve düzen koşullarını gözden geçirin."));
    }
    let row_height = typography::scaled(if w.kind == ArrayFindKind::Lattice { 128. } else { 96. });
    let list = VirtualList::new(output.matches.len(), row_height, move |index| {
        let candidate = &output.matches[index];
        let title = if let ArrayLayout::Grid { rows, cols, .. } | ArrayLayout::Lattice { rows, cols, .. } = candidate.adoption.layout {
            format!(
                "{}{rows} × {cols} · {} dolu · {} boş",
                if candidate.overlaps {
                    "Ortak öğe · "
                } else {
                    ""
                },
                candidate.present,
                candidate.missing
            )
        } else if let ArrayLayout::Polar { count, .. } = candidate.adoption.layout {
            format!("Çember · {count} yer · {} dolu · {} boş", candidate.present, candidate.missing)
        } else if let ArrayLayout::Path { count, .. } = candidate.adoption.layout {
            format!("Yol · {count} yer · {} dolu · {} boş", candidate.present, candidate.missing)
        } else {
            format!(
                "Sıra {} · {} dolu · {} boş",
                index + 1,
                candidate.present,
                candidate.missing
            )
        };
        let detail: Element<'_, Message> =
            if let ArrayLayout::Grid { dx, dy, .. } = candidate.adoption.layout {
                label::caption(format!(
                    "Sütun {} · Satır {}",
                    f.length(dx.abs()),
                    f.length(dy.abs())
                ))
                .into()
            } else if let ArrayLayout::Lattice {dx,dy,column_angle,row_angle,..}=candidate.adoption.layout {
                label::caption(format!("Sütun {} / {}\nSatır {} / {} · Yön korunur",f.length(dx.abs()),f.bearing(column_angle*400./360.),f.length(dy.abs()),f.bearing(row_angle*400./360.))).into()
            } else if let ArrayLayout::Polar { count, rotate, .. } = candidate.adoption.layout {
                label::caption(format!("{} aralık · {}{}", f.bearing(400. / count.max(1) as f64),
                    if rotate { "Öğeler döner" } else { "Yön korunur" },
                    if candidate.overlaps { " · Ortak öğe" } else { "" })).into()
            } else if let ArrayLayout::Path { spacing, align, .. } = candidate.adoption.layout {
                label::caption(format!("{} · {}{}", f.length(spacing.unwrap_or(0.)),
                    if align { "Teğeti izler" } else { "Yön korunur" },
                    if candidate.overlaps { " · Ortak öğe" } else { "" })).into()
            } else {
                row![
                    label::caption(format!(
                        "Aralık {}{}",
                        f.length(spacing(&candidate.adoption.layout)),
                        if candidate.overlaps {
                            " · Ortak öğe"
                        } else {
                            ""
                        }
                    ))
                    .width(Fill),
                    label::caption(places(&candidate.adoption.items, 8))
                ]
                .spacing(6)
                .into()
            };
        container(
            column![
                label::body(title),
                detail,
                row![
                    words::secondary("Göster", current.then(|| msg(Event::Show(index)))),
                    words::primary("İncele…", current.then(|| msg(Event::Review(index))))
                ]
                .spacing(6)
            ]
            .spacing(5),
        )
        .padding(8)
        .width(Fill)
        .style(style::container::tile)
        .into()
    })
    .height(Fill);
    let header = container(scrollable(top)).max_height((height - row_height - 64.).max(24.));
    container(
        column![
            header,
            list,
            row![
                words::secondary("Yeni kaynak", Some(msg(Event::Back))),
                words::secondary("Ayarlar", current.then(|| msg(Event::Settings)))
            ]
            .spacing(6)
        ]
        .spacing(8),
    )
    .padding(8)
    .height(Fill)
    .into()
}

#[derive(Clone, Debug)]
pub enum Event {
    Back,
    CloseDialog,
    Settings,
    Run,
    Stop,
    Hidden,
    Tolerance(f64),
    PatternKind(ArrayFindKind),
    Path(Choice),
    Ready(u64, Box<CommandResult<ArraysFound>>),
    Show(usize),
    Review(usize),
    Reviewed(u64, Box<CommandResult<ArraysAdoptPlan>>),
    KeepOptions,
    KeepObject(Choice),
    KeepPlace(f64),
    KeepRow(f64),
    KeepCol(f64),
    Assign,
    Unassign(usize),
    ShowKept,
    Apply,
    Refresh,
    Tick,
}
fn msg(e: Event) -> Message {
    super::msg(super::Event::Arrays(e))
}

impl App {
    pub(crate) fn close_array_find(&mut self) {
        self.blocks_panel.arrays = None;
        if self.dialog == Some(Dialog::ArrayFind) {
            self.dialog = None;
        }
    }
    pub(crate) fn open_array_find(&mut self) {
        self.close_block_find();
        self.show_blocks_panel();
        let Some(doc) = &self.document else {
            return;
        };
        let selection = self.selection.ids();
        let inserts: Vec<_> = selection.iter().copied().filter(|s|matches!(doc.model.get(*s),Some(Entity::Insert(_)))).collect();
        let picked = inserts.first().filter(|_|inserts.len()==1).copied();
        let paths:Vec<_> = doc.model.entities_with_uids().filter_map(|(uid,e)|{
            let kind = match e {Entity::Line(_)=>"Çizgi",Entity::Arc(_)=>"Yay",Entity::Circle(_)=>"Çember",Entity::Polyline(_)=>"Çoklu çizgi",_=>return None};
            if !doc.model.layers().is_visible(&e.base().layer_id) || doc.model.layers().is_frozen(&e.base().layer_id) {return None;}
            Some(Choice {uid:uid.to_string(),label:format!("{kind} #{} · Katman {}",e.base().id,e.base().layer_id)})
        }).take(2048).collect();
        let selected_paths:Vec<_> = paths.iter().filter(|p|p.uid.parse().ok().and_then(|u|doc.model.slot_of(u)).is_some_and(|s|selection.contains(&s))).cloned().collect();
        let path = selected_paths.first().filter(|_|selected_paths.len()==1).cloned();
        let kind = if path.is_some() {ArrayFindKind::Path} else {ArrayFindKind::Line};
        let (source, name) = picked
            .and_then(|s| {
                let Entity::Insert(i) = doc.model.get(s)? else {
                    return None;
                };
                Some((
                    doc.model.uid(s)?.to_string(),
                    doc.model.block(i.block)?.name.clone(),
                ))
            })
            .map_or((None, "Kaynak seçilmedi".into()), |(u, n)| (Some(u), n));
        self.blocks_panel.arrays = Some(Window {
            source,
            name,
            session: doc.session,
            generation: doc.model.generation(),
            revision: doc.model.revision().to_string(),
            hidden: false,
            tolerance: 1e-6,
            kind,
            path,
            paths,
            previous: None,
            token: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            busy: false,
            output: None,
            warnings: Vec::new(),
            refused: None,
            review: None,
            job: None,
        });
        self.dialog = Some(Dialog::ArrayFind);
    }
    fn array_find_review_start(&mut self) -> Task<Message> {
        let (Some(doc), Some(w)) = (&self.document, self.blocks_panel.arrays.as_mut()) else {
            return Task::none();
        };
        let Some(review) = &mut w.review else {
            return Task::none();
        };
        review.plan = None;
        let input = review.input.clone();
        let model = doc.model.clone();
        let (token, cancel) = w.fresh_work();
        crate::db_connections::off_thread(
            move || {
                kentos_native_application::arrays_adopt::inspect(&model, &input, || {
                    cancel.load(Ordering::Relaxed)
                })
            },
            move |answer| msg(Event::Reviewed(token, Box::new(answer))),
        )
    }
    pub(crate) fn array_find_close_dialog(&mut self) {
        self.dialog = None;
        if let Some(w) = &mut self.blocks_panel.arrays {
            w.stop();
            w.review = None;
            if let Some((hidden, tolerance, kind, path)) = w.previous.take() {
                w.hidden = hidden;
                w.tolerance = tolerance;
                w.kind = kind;
                w.path = path;
            }
            if w.output.is_none() {
                self.blocks_panel.arrays = None;
            }
        }
    }
    pub(crate) fn array_find_subscription(&self) -> Subscription<Message> {
        if self
            .blocks_panel
            .arrays
            .as_ref()
            .is_some_and(|w| w.job.is_some())
        {
            iced::window::frames().map(|_| msg(Event::Tick))
        } else {
            Subscription::none()
        }
    }
    pub(super) fn array_find_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Back => {
                self.close_array_find();
                return Task::none();
            }
            Event::CloseDialog => {
                self.array_find_close_dialog();
                return Task::none();
            }
            _ => {}
        }
        let current = self
            .blocks_panel
            .arrays
            .as_ref()
            .is_some_and(|w| w.current(self));
        let (Some(doc), Some(w)) = (self.document.as_mut(), self.blocks_panel.arrays.as_mut())
        else {
            return Task::none();
        };
        if !current {
            w.stop();
            if let Some(r) = &mut w.review {
                r.plan = None;
            }
            w.refused =
                Some("Çizim değişti. Eski aday uygulanmadı; yeni kaynakla aramayı açın.".into());
            return Task::none();
        }
        match event {
            Event::Settings if !w.busy => {
                w.review = None;
                w.previous = Some((w.hidden, w.tolerance, w.kind, w.path.clone()));
                self.dialog = Some(Dialog::ArrayFind);
            }
            Event::PatternKind(kind) if !w.busy => {
                w.kind = kind;
                w.refused = None;
            }
            Event::Path(path) if !w.busy => {
                if w.paths.contains(&path) {w.path=Some(path);w.refused=None;}
            }
            Event::Tolerance(t) if !w.busy => {
                w.tolerance = t;
                w.refused = None;
            }
            Event::Hidden if !w.busy => w.hidden = !w.hidden,
            Event::Run if !w.busy => {
                let Some(input) = w.input(&doc.model) else {
                    return Task::none();
                };
                let model = doc.model.clone();
                let (token, cancel) = w.fresh_work();
                w.previous = None;
                w.output = None;
                w.review = None;
                w.warnings.clear();
                self.dialog = None;
                let echo = self.echo_query(
                    kentos_commands::CAD_ARRAYS_FIND,
                    &serde_json::to_value(&input).unwrap_or_default(),
                );
                return Task::batch([
                    echo,
                    crate::db_connections::off_thread(
                        move || {
                            kentos_native_application::arrays_find::inspect(&model, &input, || {
                                cancel.load(Ordering::Relaxed)
                            })
                        },
                        move |answer| msg(Event::Ready(token, Box::new(answer))),
                    ),
                ]);
            }
            Event::Stop => {
                w.stop();
                if let Some(r) = &mut w.review {
                    r.plan = None;
                }
                w.refused = Some("İşlem durduruldu; çizim değiştirilmedi.".into());
            }
            Event::Ready(token, answer)
                if token == w.token && !w.cancel.load(Ordering::Relaxed) =>
            {
                w.busy = false;
                match *answer {
                    CommandResult::Completed { output, warnings } => {
                        w.output = Some(output);
                        w.warnings = warnings.into_iter().map(|w| w.message).collect();
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => w.refused = Some(error.message),
                    _ => w.refused = Some("Arama tamamlanmadı; çizim değiştirilmedi.".into()),
                }
            }
            Event::Show(index) => {
                let Some(candidate) = w.output.as_ref().and_then(|o| o.matches.get(index)) else {
                    return Task::none();
                };
                let slots: Vec<_> = candidate
                    .adoption
                    .items
                    .iter()
                    .filter_map(|i| i.uid)
                    .filter_map(|u| doc.model.slot_of(kentos_domain::Uuid::from_bytes(u.0)))
                    .collect();
                self.selection.set(slots);
                self.dialog = None;
                self.zoom_selection();
            }
            Event::Review(index) if !w.busy => {
                if w.review.as_ref().is_some_and(|r| r.index == index) {
                    self.dialog = Some(Dialog::ArrayFind);
                    return Task::none();
                }
                let Some(output) = &w.output else {
                    return Task::none();
                };
                let Some(candidate) = output.matches.get(index) else {
                    return Task::none();
                };
                let used: HashSet<_> = candidate
                    .adoption
                    .items
                    .iter()
                    .filter_map(|i| i.uid.map(|u| u.to_text()))
                    .collect();
                let format = kentos_interaction::Format::of(doc.settings());
                let choices: Vec<_> = output
                    .eligible
                    .iter()
                    .filter(|u| !used.contains(*u))
                    .filter_map(|u| {
                        let Entity::Insert(i) =
                            doc.model.get(doc.model.slot_of(u.parse().ok()?)?)?
                        else {
                            return None;
                        };
                        Some(Choice {
                            uid: u.clone(),
                            label: format!(
                                "Blok #{} · X {} · Y {}",
                                i.base.id,
                                format.length(i.p.x),
                                format.length(i.p.y)
                            ),
                        })
                    })
                    .collect();
                let place = candidate
                    .adoption
                    .items
                    .iter()
                    .position(|i| i.uid.is_none())
                    .map_or(1., |k| (k + 1) as f64);
                w.review = Some(Review {
                    index,
                    input: candidate.adoption.clone(),
                    plan: None,
                    choice: choices.first().cloned(),
                    choices,
                    place,
                    keep_options: false,
                });
                self.dialog = Some(Dialog::ArrayFind);
                return self.array_find_review_start();
            }
            Event::Reviewed(token, answer)
                if token == w.token && !w.cancel.load(Ordering::Relaxed) =>
            {
                w.busy = false;
                match *answer {
                    CommandResult::Completed { output, .. } => {
                        if let Some(r) = &mut w.review {
                            r.plan = Some(output);
                        }
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => w.refused = Some(error.message),
                    _ => w.refused = Some("İnceleme durduruldu; çizim değiştirilmedi.".into()),
                }
            }
            Event::Refresh if !w.busy && w.review.is_some() => {
                return self.array_find_review_start();
            }
            Event::KeepOptions if !w.busy => {
                if let Some(r) = &mut w.review {
                    r.keep_options = !r.keep_options;
                }
            }
            Event::KeepObject(choice) if !w.busy => {
                if let Some(r) = &mut w.review {
                    r.choice = Some(choice);
                }
            }
            Event::KeepPlace(place) if !w.busy => {
                if let Some(r) = &mut w.review {
                    r.place = place;
                }
            }
            Event::KeepRow(value) | Event::KeepCol(value) if !w.busy => {
                let row = matches!(event, Event::KeepRow(_));
                if let Some(r) = &mut w.review
                    && let ArrayLayout::Grid { rows, cols, .. } | ArrayLayout::Lattice { rows, cols, .. } = r.input.layout
                {
                    let max = if row { rows } else { cols };
                    if !value.is_finite() || value.fract() != 0. || value < 1. || value > max as f64
                    {
                        w.refused = Some("Satır ve sütun geçerli tam sayı olmalı.".into());
                        return Task::none();
                    }
                    let old = (r.place - 1.).max(0.) as u32;
                    let (r0, c0) = if row {
                        (value as u32 - 1, old % cols)
                    } else {
                        (old / cols, value as u32 - 1)
                    };
                    r.place = (r0 * cols + c0 + 1) as f64;
                }
            }
            Event::Assign if !w.busy => {
                let Some(r) = &mut w.review else {
                    return Task::none();
                };
                let Some(choice) = &r.choice else {
                    return Task::none();
                };
                if !r.place.is_finite()
                    || r.place.fract() != 0.
                    || r.place < 2.
                    || r.place > r.input.items.len() as f64
                {
                    w.refused = Some("Boş bir yer numarası seçin; kaynak yer korunamaz.".into());
                    return Task::none();
                }
                let index = r.place as usize - 1;
                if r.input.items[index].uid.is_some()
                    || r.input
                        .items
                        .iter()
                        .any(|i| i.uid.is_some_and(|u| u.to_text() == choice.uid))
                {
                    w.refused=Some("Bu yer dolu veya blok zaten bağlı. Yalnız boş bir yere eşleşmeyen blok eklenebilir.".into());
                    return Task::none();
                }
                let Ok(uid) = choice.uid.parse::<kentos_domain::Uuid>() else {
                    return Task::none();
                };
                r.input.items[index] = ArrayItem {
                    uid: Some(kentos_commands::EntityId(uid.into_bytes())),
                    kept: true,
                };
                r.choice = r
                    .choices
                    .iter()
                    .find(|c| {
                        !r.input
                            .items
                            .iter()
                            .any(|i| i.uid.is_some_and(|u| u.to_text() == c.uid))
                    })
                    .cloned();
                r.place = r
                    .input
                    .items
                    .iter()
                    .position(|i| i.uid.is_none())
                    .map_or(1., |k| (k + 1) as f64);
                return self.array_find_review_start();
            }
            Event::Unassign(index) if !w.busy => {
                let Some(r) = &mut w.review else {
                    return Task::none();
                };
                if let Some(item) = r.input.items.get_mut(index).filter(|i| i.kept) {
                    *item = ArrayItem {
                        uid: None,
                        kept: false,
                    };
                    r.place = (index + 1) as f64;
                    r.choice = r
                        .choices
                        .iter()
                        .find(|c| {
                            !r.input
                                .items
                                .iter()
                                .any(|i| i.uid.is_some_and(|u| u.to_text() == c.uid))
                        })
                        .cloned();
                    return self.array_find_review_start();
                }
            }
            Event::ShowKept => {
                let Some(choice) = w.review.as_ref().and_then(|r| r.choice.as_ref()) else {
                    return Task::none();
                };
                let Some(slot) = choice.uid.parse().ok().and_then(|u| doc.model.slot_of(u)) else {
                    return Task::none();
                };
                self.selection.set([slot]);
                self.dialog = None;
                self.zoom_selection();
            }
            Event::Apply if !w.busy && w.refused.is_none() => {
                let Some(r) = w.review.as_ref().filter(|r| r.plan.is_some()) else {
                    return Task::none();
                };
                w.busy = true;
                w.refused = None;
                w.job = Some(Job::start(
                    format!("array-adopt-{}", w.token),
                    &doc.model,
                    CallEnvelope {
                        command: kentos_commands::CAD_ARRAYS_ADOPT.into(),
                        version: None,
                        op: CallOp::Execute,
                        input: serde_json::to_value(&r.input).unwrap_or_default(),
                        request_id: None,
                        expected: None,
                        relax_constraints: false,
                    },
                ));
            }
            Event::Tick => {
                let Some(job) = &mut w.job else {
                    return Task::none();
                };
                let status = job.status(&mut doc.model);
                if !status.phase.finished() {
                    return Task::none();
                }
                w.job = None;
                w.busy = false;
                let Some(answer) = status.answer else {
                    w.refused = Some("İşlem sonucu alınamadı; geçmişi kontrol edin.".into());
                    return Task::none();
                };
                match answer.result {
                    CommandResult::Completed { output, .. } => {
                        let Ok(output) = serde_json::from_value::<ArraysAdopted>(output) else {
                            w.refused = Some("Dizi cevabı okunamadı; geçmişi kontrol edin.".into());
                            return Task::none();
                        };
                        let slots: Vec<_> = output
                            .retained
                            .iter()
                            .filter_map(|u| u.parse().ok())
                            .filter_map(|u| doc.model.slot_of(u))
                            .collect();
                        self.selection.set(slots);
                        self.close_array_find();
                        self.say(kentos_interaction::Level::Success,"Mevcut bloklar diziye bağlandı. Kimlikler ve boş yerler korundu; tek adımda geri alınır.");
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => w.refused = Some(error.message),
                    _ => w.refused = Some("Bağlama durduruldu; çizim değiştirilmedi.".into()),
                }
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
