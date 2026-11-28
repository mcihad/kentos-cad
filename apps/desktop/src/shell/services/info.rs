//! Servis bilgisi (docs/adr/0208 §11; the web's `ui/bottom/serviceInfoRun.ts`): a
//! point picked on the drawing, then every WMS layer shown is asked with
//! GetFeatureInfo and every ArcGIS layer with `identify`, each with its
//! connection's proof, off the window's thread. A small map of 101 pixels
//! around the point is asked for, in the service's own system, so the
//! answer does not depend on the view's size. The answers are listed in the
//! bottom panel's Servis bilgisi tab, layer by layer, field and value.

use iced::widget::{Column, column, container, row, scrollable};
use iced::{Element, Fill, Length, Task};
use kentos_commands::ServiceKind;
use kentos_interaction::Vec2;
use kentos_interaction::pick::PickPoint;
use kentos_services::info::{InfoRow, read, requests};
use kentos_ui::label;

use crate::app::{App, Message};

/// What the tab shows: the point asked about and each service's answer.
#[derive(Clone, Debug, Default)]
pub struct Info {
    /// A point is being picked for it.
    pub picking: bool,
    /// The point picked, waiting for its requests.
    pub wanted: Option<Vec2>,
    pub busy: bool,
    pub at: Option<Vec2>,
    pub answers: Vec<Answer>,
    /// The ask the answers are for (GIS-044): another ask, or another drawing, makes a late answer stale.
    pub asked: u64,
}

#[derive(Clone, Debug)]
pub struct Answer {
    /// The layer's name.
    pub layer: String,
    /// The layer's id in the drawing it was asked on.
    pub layer_id: String,
    pub rows: Result<Vec<InfoRow>, String>,
}

impl App {
    /// Servis bilgisi: a point is asked for on the drawing.
    pub fn start_service_info(&mut self) {
        // A program without the tab says where it is (module_surfaces.rs).
        if !self.open_bottom_tab(crate::bottom::BottomTab::ServiceInfo) {
            return;
        }
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let any = crate::style::scene::shown_service_layers(doc.model.layers().nodes())
            .iter()
            .any(|s| matches!(s.kind, ServiceKind::Wms | ServiceKind::Arcgis));
        if !any {
            self.output("Sorulabilecek servis yok: görünen bir WMS ya da ArcGIS katmanı ekleyin.");
            return;
        }
        self.service_info.picking = true;
        self.session.run(Box::new(PickPoint::new(
            "Servis bilgisi",
            "Sorulacak nokta",
        )));
    }

    /// The point picked for Servis bilgisi (none: given up); false when it was another window's.
    pub(crate) fn service_info_picked(&mut self, p: Option<Vec2>) -> bool {
        if !std::mem::replace(&mut self.service_info.picking, false) {
            return false;
        }
        self.service_info.wanted = p;
        true
    }

    /// The picked point's requests, off the window's thread.
    pub(crate) fn service_info_tasks(&mut self) -> Task<Message> {
        let Some(p) = self.service_info.wanted.take() else {
            return Task::none();
        };
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let settings = doc.model.settings();
        let project = super::systems::ProjectSystem::of(settings);
        // Each shown WMS or ArcGIS layer, its name, connection and the point in its system.
        let mut jobs = Vec::new();
        for n in crate::style::scene::shown_layers(doc.model.layers().nodes()) {
            let Some(s) = &n.service else {
                continue;
            };
            if !matches!(s.kind, ServiceKind::Wms | ServiceKind::Arcgis) {
                continue;
            }
            let srid = s.srid.or(s.grid.as_ref().map(|g| g.srid)).unwrap_or(3857);
            let Ok(pair) = super::systems::pair(&project, srid) else {
                continue;
            };
            let Some((x, y)) = (pair.to_grid)(p.x, p.y) else {
                continue;
            };
            // A screen pixel in the service's units, from two points a pixel apart.
            let px = 1.0 / self.viewport.camera.scale;
            let units = (pair.to_grid)(p.x + px, p.y)
                .map(|(x2, y2)| (x2 - x).hypot(y2 - y))
                .filter(|u| u.is_finite() && *u > 0.0)
                .unwrap_or(px);
            let conn = s
                .connection
                .as_ref()
                .and_then(|id| settings.connections.iter().find(|c| &c.id == id))
                .cloned();
            let secret = conn
                .as_ref()
                .and_then(|c| super::secrets::secrets().get(&c.origin, &c.id));
            jobs.push((
                (n.name.clone(), n.id.clone()),
                s.clone(),
                srid,
                Vec2::new(x, y),
                units,
                conn,
                secret,
            ));
        }
        self.service_info.busy = true;
        self.service_info.at = Some(p);
        self.service_info.answers.clear();
        self.service_info.asked += 1;
        let asked = self.service_info.asked;
        self.show_bottom(crate::bottom::BottomTab::ServiceInfo);
        super::app::off_thread(
            move || {
                jobs.into_iter()
                    .map(|((name, layer_id), s, srid, q, units, conn, secret)| {
                        let mut last = Err("Servis bu noktada bilgi vermedi.".to_owned());
                        for (media, req) in requests(&s, srid, q.x, q.y, units) {
                            match super::net::send_text(
                                req,
                                conn.as_ref().map(|c| (c, secret.as_ref())),
                                &s.url,
                                4,
                            ) {
                                Ok(body) => {
                                    last = Ok(read(&media, &body));
                                    break;
                                }
                                Err(e) => last = Err(e),
                            }
                        }
                        Answer {
                            layer: name,
                            layer_id,
                            rows: last,
                        }
                    })
                    .collect::<Vec<_>>()
            },
            move |answers| super::app::Event::Info(asked, answers),
        )
    }

    /// The answers of the ask `asked`: kept only when it is still the one asked (GIS-044): one that comes after
    /// another drawing was opened, or after another point was asked, is dropped.
    pub(crate) fn service_info_answered(&mut self, asked: u64, answers: Vec<Answer>) {
        if asked != self.service_info.asked {
            return;
        }
        self.service_info.busy = false;
        // Each answer of a layer the drawing still has (an id is the drawing's own, a name may be another's).
        let held = |a: &Answer| {
            a.layer_id.is_empty()
                || self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.model.layers().get(&a.layer_id).is_some())
        };
        self.service_info.answers = answers.into_iter().filter(|a| held(a)).collect();
    }

    /// Another drawing (GIS-044): the point, its answers and a pick going on are the last drawing's, never shown on
    /// this one; an answer still on its way is dropped when it comes.
    pub(crate) fn leave_service_info(&mut self) {
        let asked = self.service_info.asked + 1;
        self.service_info = Info {
            asked,
            ..Info::default()
        };
    }

    /// The bottom panel's Servis bilgisi tab.
    pub(crate) fn service_info_tab(&self) -> Element<'_, Message> {
        let i = &self.service_info;
        let mut list = Column::new().spacing(10).padding([8, 12]);
        match (&i.at, i.busy) {
            (None, _) => {
                return container(label::muted(
                    "Harita › Altlık › Servis bilgisi ile çizimde bir noktaya tıklayın: görünen WMS ve ArcGIS katmanlarının o noktadaki kayıtları burada listelenir.",
                ))
                .padding(12)
                .into();
            }
            (Some(_), true) => list = list.push(label::muted("Servislere soruluyor…")),
            _ => {}
        }
        if let Some(at) = i.at
            && let Some(doc) = &self.document
        {
            let f = kentos_interaction::Format::of(doc.settings());
            list = list.push(label::caption(format!("Nokta: {}", f.point(at))));
        }
        for a in &i.answers {
            let mut block = Column::new()
                .spacing(4)
                .push(label::strong(a.layer.clone()));
            match &a.rows {
                Err(e) => block = block.push(label::muted(e.clone())),
                Ok(rows) if rows.is_empty() => {
                    block = block.push(label::muted("Bu noktada kayıt yok."))
                }
                Ok(rows) => {
                    for r in rows {
                        block = block.push(label::caption(r.layer.clone()));
                        for (k, v) in &r.fields {
                            block = block.push(
                                row![
                                    container(label::body(k.clone())).width(Length::Fixed(180.0)),
                                    label::body(v.clone()),
                                ]
                                .spacing(12),
                            );
                        }
                    }
                }
            }
            list = list.push(block);
        }
        scrollable(column![list].width(Fill)).height(Fill).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Message;

    fn answer(layer: &str) -> Answer {
        Answer {
            layer: layer.into(),
            layer_id: String::new(),
            rows: Ok(Vec::new()),
        }
    }

    /// GIS-044 (docs/adr/1101): Servis bilgisi's point and answers are the drawing's they were asked on. Another
    /// drawing opened clears the tab; an answer that comes after it, or after another point was asked, is dropped,
    /// even when both drawings have a layer of the same name.
    #[test]
    fn another_drawing_never_shows_the_last_ones_answers() {
        let mut app = crate::files_testing::app_with_drawing();
        app.service_info.at = Some(Vec2::new(487_434.292, 4_421_766.999));
        app.service_info.asked = 7;
        app.service_info.busy = true;
        app.service_info_answered(7, vec![answer("Parseller")]);
        assert_eq!(
            app.service_info.answers.len(),
            1,
            "the ask's own answer is shown"
        );
        // Another point asked meanwhile: the first one's late answer is dropped.
        app.service_info.asked = 8;
        app.service_info_answered(7, vec![answer("Parseller")]);
        assert_eq!(app.service_info.answers.len(), 1);
        app.service_info.busy = true;
        // Another drawing opened while the answer is on its way.
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
            "../../../../../fixtures/interaction/v1/empty.kcad"
        ))
        .expect("reads");
        let doc = crate::document::Document::new(snapshot, None).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        assert!(
            app.service_info.at.is_none() && app.service_info.answers.is_empty(),
            "cleared"
        );
        assert!(!app.service_info.busy);
        app.service_info_answered(8, vec![answer("Parseller")]);
        assert!(
            app.service_info.answers.is_empty(),
            "the late answer is not the new drawing's"
        );
    }
}
