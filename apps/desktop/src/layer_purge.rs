//! Kullanılmayanları temizle (`layer.purge`, docs/adr/0177 §5; the web's
//! `app/layerPurge.ts` and `ui/layers/PurgeDialog.ts`): what the drawing and
//! the project library hold that nothing uses, by the shared rule
//! (`kentos_domain::layer_purge`).
//!
//! The window lists them by kind, each with its box, checked; a locked empty
//! layer is listed to be unlocked first and cannot be checked. Temizle removes
//! the checked ones nothing staying uses: the definitions (in rounds), the
//! layers and the groups (deepest first) in one undo step “Kullanılmayanları
//! temizle”, then the project library's symbols and assets, which keeps no
//! undo (docs/adr/0092). A project without `project.edit` may not change its
//! tree.

use std::collections::BTreeSet;

use iced::widget::{Column, button, container, row, scrollable, text};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{BlockId, Entity};
use kentos_domain::layer_purge::{
    self, PurgeBlock, PurgeFound, PurgeIds, PurgeItem, PurgeKind, PurgeObject, PurgeSource,
};
use kentos_interaction::Level;
use kentos_native_style::library::{ItemKind, Source};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog as Frame, Elided, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const PURGE_TITLE: &str = "Kullanılmayanları temizle";
const PURGE: &str = "Temizle";
const CANCEL: &str = "Vazgeç";

/// Each kind's heading in the list.
fn heading(kind: PurgeKind) -> &'static str {
    match kind {
        PurgeKind::Layers => "Boş katmanlar",
        PurgeKind::Groups => "Boş kalan gruplar",
        PurgeKind::Blocks => "Kullanılmayan bloklar",
        PurgeKind::Symbols => "Projenin kullanılmayan sembolleri",
        PurgeKind::Assets => "Projenin kullanılmayan varlıkları",
    }
}

fn key(kind: PurgeKind, id: &str) -> String {
    format!("{}:{id}", kind.key())
}

/// The window: what was found when it opened, and what is checked.
#[derive(Debug, Clone)]
pub struct Window {
    found: PurgeFound,
    checked: BTreeSet<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A row's box, by its kind and id.
    Check(String, bool),
    Purge,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::LayerPurge(event)
}

fn object_of(e: &Entity) -> PurgeObject {
    let base = e.base();
    PurgeObject {
        layer: base.layer_id.clone(),
        symbol: base.symbol.clone(),
        block: match e {
            Entity::Insert(i) => Some(i.block.to_text()),
            _ => None,
        },
        // A picture's image (docs/adr/0192 §2).
        asset: match e {
            Entity::Image(i) => i.image.asset.clone(),
            _ => None,
        },
    }
}

/// “2 katman, 1 grup, 3 sembol”: the counts that are not naught.
fn counts_text(removed: &PurgeIds) -> String {
    [
        (removed.layers.len(), "katman"),
        (removed.groups.len(), "grup"),
        (removed.blocks.len(), "blok"),
        (removed.symbols.len(), "sembol"),
        (removed.assets.len(), "varlık"),
    ]
    .iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, w)| format!("{n} {w}"))
    .collect::<Vec<_>>()
    .join(", ")
}

impl App {
    /// What the rule reads of the open drawing and the library.
    fn purge_found_now(&self) -> PurgeFound {
        let Some(doc) = &self.document else {
            return PurgeFound::default();
        };
        let model = &doc.model;
        let layers = model.layers();
        let src = PurgeSource {
            tree: layers.nodes(),
            active: layers.active(),
            objects: model.entities().map(object_of).collect(),
            blocks: model
                .blocks()
                .iter()
                .map(|b| PurgeBlock {
                    id: b.id.to_text(),
                    name: b.name.clone(),
                    objects: b.entities.iter().map(object_of).collect(),
                })
                .collect(),
            library: self.purge_library(),
        };
        layer_purge::found(&src)
    }

    /// The library's items as the rule reads them.
    fn purge_library(&self) -> Vec<PurgeItem> {
        self.styles
            .library
            .items(None)
            .into_iter()
            .map(|(it, source)| PurgeItem {
                id: it.id().to_owned(),
                kind: match it.kind() {
                    ItemKind::Symbol => "symbol",
                    ItemKind::Asset => "asset",
                    ItemKind::Template => "template",
                }
                .to_owned(),
                source: source.key().to_owned(),
                name: it.name().to_owned(),
                symbol: it.symbol().cloned(),
                template: it.template().cloned(),
            })
            .collect()
    }

    /// Kullanılmayanları temizle (`layer.purge`): the window, everything
    /// found checked but a locked layer.
    pub(crate) fn open_layer_purge(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return;
        }
        let found = self.purge_found_now();
        let checked = PurgeKind::ALL
            .iter()
            .flat_map(|k| {
                found
                    .of(*k)
                    .iter()
                    .filter(|f| !f.locked)
                    .map(|f| key(*k, &f.id))
            })
            .collect();
        self.layer_purge = Some(Window { found, checked });
        self.dialog = Some(Dialog::LayerPurge);
    }

    /// Removes the checked ones nothing staying uses, and says what went.
    /// Whether anything went.
    pub(crate) fn purge_unused(&mut self, checked: &PurgeIds) -> bool {
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return false;
        }
        let library = self.purge_library();
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let (removed, kept) = {
            let model = &doc.model;
            let layers = model.layers();
            let src = PurgeSource {
                tree: layers.nodes(),
                active: layers.active(),
                objects: model.entities().map(object_of).collect(),
                blocks: model
                    .blocks()
                    .iter()
                    .map(|b| PurgeBlock {
                        id: b.id.to_text(),
                        name: b.name.clone(),
                        objects: b.entities.iter().map(object_of).collect(),
                    })
                    .collect(),
                library,
            };
            layer_purge::removed(&src, checked)
        };
        let model = &mut doc.model;
        if !(removed.blocks.is_empty() && removed.layers.is_empty() && removed.groups.is_empty()) {
            let group = model.begin_group("Kullanılmayanları temizle");
            let mut refused = None;
            for id in removed.blocks.iter().filter_map(|t| BlockId::parse(t)) {
                if let Err(r) = model.remove_block(id) {
                    refused = Some(r.to_string());
                    break;
                }
            }
            for id in removed.layers.iter().chain(&removed.groups) {
                if refused.is_some() {
                    break;
                }
                if let Err(r) = model.remove_layer(id) {
                    refused = Some(r.to_string());
                }
            }
            if let Some(why) = refused {
                model.cancel_group(group);
                self.warn(why);
                return false;
            }
            model.end_group(group);
        }
        if !(removed.symbols.is_empty() && removed.assets.is_empty()) {
            for id in removed.symbols.iter().chain(&removed.assets) {
                let _ = self.styles.library.remove(id);
            }
            if let Some(problem) = self.styles.changed(
                Source::Project,
                self.document.as_mut().map(|d| &mut d.model),
            ) {
                self.warn(problem);
            }
        }
        let gone = counts_text(&removed);
        if gone.is_empty() {
            self.say(
                Level::Info,
                "Silinecek bir şey kalmadı: işaretlenenleri kalanlar kullanıyor.",
            );
            return false;
        }
        self.say(
            Level::Success,
            format!("Kullanılmayanlar temizlendi: {gone}."),
        );
        if kept > 0 {
            self.say(
                Level::Info,
                format!("İşaretlenen {kept} öğe, kalanlar kullandığı için silinmedi."),
            );
        }
        true
    }

    pub(crate) fn layer_purge_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Check(k, on) => {
                if let Some(w) = self.layer_purge.as_mut() {
                    if on {
                        w.checked.insert(k);
                    } else {
                        w.checked.remove(&k);
                    }
                }
            }
            Event::Purge => {
                let Some(w) = &self.layer_purge else {
                    return Task::none();
                };
                let mut ids = PurgeIds::default();
                for kind in PurgeKind::ALL {
                    let prefix = format!("{}:", kind.key());
                    *ids.of_mut(kind) = w
                        .checked
                        .iter()
                        .filter_map(|k| k.strip_prefix(&prefix).map(str::to_owned))
                        .collect();
                }
                if self.purge_unused(&ids) {
                    self.layer_purge = None;
                    self.dialog = None;
                }
            }
            Event::Cancel => {
                self.layer_purge = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn layer_purge_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.layer_purge else {
            return text("").into();
        };
        let any = !w.found.is_empty();
        let lines = if any {
            vec![
                words::text_line(
                    Kind::Info,
                    "İşaretlenenler silinir. Katmanlar, gruplar ve bloklar tek adımda geri alınır; projenin kitaplığından silinen semboller ve varlıklar geri alınamaz.",
                ),
                if w.checked.is_empty() {
                    words::text_line(Kind::Info, "Silinecekleri işaretleyin.")
                } else {
                    words::text_line(Kind::Ok, format!("{} öğe işaretli.", w.checked.len()))
                },
            ]
        } else {
            vec![words::text_line(
                Kind::Ok,
                "Temizlenecek bir şey yok: her katmanda nesne var; bloklar, projenin sembolleri ve varlıkları kullanılıyor.",
            )]
        };
        let mut body = Column::new().spacing(12).push(words::summary(lines));
        if any {
            let mut list = Column::new().spacing(0).padding([4, 0]);
            for kind in PurgeKind::ALL {
                let found = w.found.of(kind);
                if found.is_empty() {
                    continue;
                }
                list = list.push(
                    container(label::caption(format!(
                        "{} ({})",
                        heading(kind),
                        found.len()
                    )))
                    .padding([6, 10]),
                );
                for f in found {
                    let k = key(kind, &f.id);
                    let on = !f.locked && w.checked.contains(&k);
                    let toggle = (!f.locked).then(|| msg(Event::Check(k.clone(), !on)));
                    let boxed = check_box(
                        if on { Check::Checked } else { Check::Unchecked },
                        toggle.clone(),
                    );
                    let quiet = f.locked;
                    let words_el = Elided::new(f.text.clone())
                        .size(typography::caption())
                        .font(typography::ui())
                        .style(move |theme: &iced::Theme| {
                            let t = Tokens::of(theme);
                            iced::widget::text::Style {
                                color: Some(if quiet { t.muted } else { t.text }),
                            }
                        })
                        .width(Fill);
                    let mut face = row![boxed].spacing(6).align_y(Center);
                    if f.locked {
                        face = face.push(icon(Icon::Lock).size(12.0).tone(Tone::Muted));
                    }
                    face = face.push(words_el);
                    if f.locked {
                        face = face.push(label::caption("kilitli").style(|theme: &iced::Theme| {
                            iced::widget::text::Style {
                                color: Some(Tokens::of(theme).muted),
                            }
                        }));
                    }
                    list = list.push(
                        button(face)
                            .on_press_maybe(toggle)
                            // Room on the right for the list's scroll bar.
                            .padding(iced::Padding {
                                top: 3.0,
                                right: 18.0,
                                bottom: 3.0,
                                left: 10.0,
                            })
                            .width(Fill)
                            .style(style::button::ghost),
                    );
                }
            }
            body = body.push(
                container(scrollable(list).height(typography::scaled(300.0)))
                    .style(style::container::field_box)
                    .width(Fill),
            );
        }
        overlay::modal(
            Frame::new(PURGE_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(
                    PURGE,
                    (!w.checked.is_empty()).then(|| msg(Event::Purge)),
                ))
                .width(560.0),
            msg(Event::Cancel),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): a
    /// row's box by its path or name, Temizle and Vazgeç.
    pub(crate) fn layer_purge_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.layer_purge else {
            return Err(format!("{PURGE_TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Check(words, on) => {
                let Some((kind, f)) = PurgeKind::ALL
                    .iter()
                    .flat_map(|k| w.found.of(*k).iter().map(move |f| (*k, f)))
                    .find(|(_, f)| f.text == words)
                else {
                    return Err(format!("“{PURGE_TITLE}” penceresinde “{words}” kutusu yok"));
                };
                let k = key(kind, &f.id);
                let now = w.checked.contains(&k);
                (!f.locked && now != on).then(|| msg(Event::Check(k, on)))
            }
            Control::Press(PURGE) => (!w.checked.is_empty()).then(|| msg(Event::Purge)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{PURGE_TITLE}” penceresinde {other} yok")),
        })
    }
}
