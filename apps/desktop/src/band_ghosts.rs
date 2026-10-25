//! The rows round a bound layer's part (docs/adr/0578 §1). A layer that holds a part of its table (docs/adr/0510) has
//! only the part's objects; the label engine would see no obstacle just outside the area, and a label near the edge
//! would sit differently when the part moves. The rows of a band round the part (its area grown by 15 % of its
//! shorter side) that the layer holds no object for are read from the file the layer matches (the very version: the
//! file's SHA-256 is the binding's) on a thread of their own and given to the geometry store as obstacle-only
//! neighbours: they hold room as the layer's own objects do, and are neither drawn nor labelled nor picked.
//!
//! What was asked is remembered by what it depends on (the binding's revision and area), so the file is read again
//! only when the layer's source or part changed; what arrived is kept to put it back after the store was cleared.

use std::collections::{HashMap, HashSet};

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_interaction::Level;
use kentos_native_application::sources::{band_job, band_shown_key};

use crate::app::{App, Message};

/// What a band depends on: the binding's revision, the part's area and what the layer shows (its filter and time,
/// docs/adr/0636 §3).
type Key = (String, [u64; 4], String);

/// A neighbour as the store holds it: its shape and time.
type Ghost = kentos_native_application::sources::Ghost;

/// The answer of one band's read.
#[derive(Clone, Debug)]
pub(crate) struct Read {
    /// The drawing it was asked for (its session): another drawing's answer is not this one's.
    session: u64,
    layer: String,
    key: Key,
    result: Result<Vec<Ghost>, String>,
}

/// The bands asked for and arrived.
#[derive(Default)]
pub(crate) struct Bands {
    session: Option<u64>,
    asked: HashMap<String, Key>,
    shapes: HashMap<String, (Key, Vec<Ghost>)>,
    /// Why a band could not be read, by layer: said once.
    unread: HashMap<String, String>,
}

impl Bands {
    /// The bands of the open drawing's parts as the drawing is drawn with them, for `cad.labels.report`: the rows that
    /// arrived, or why there are none (not read yet, or could not be).
    pub(crate) fn given(
        &self,
        doc: &kentos_domain::Document,
    ) -> Vec<kentos_native_application::labels_report::GivenBand> {
        doc.settings()
            .sources
            .iter()
            .filter_map(|link| {
                let extent = link.extent?;
                let rows = match self.shapes.get(&link.layer) {
                    Some((_, shapes)) => Ok(shapes.clone()),
                    None => Err(self
                        .unread
                        .get(&link.layer)
                        .cloned()
                        .unwrap_or_else(|| "henüz okunmadı".to_owned())),
                };
                Some(kentos_native_application::labels_report::GivenBand {
                    layer: link.layer.clone(),
                    area: kentos_native_application::sources::band_of(&extent),
                    rows,
                })
            })
            .collect()
    }

    /// How many rows the layer's band holds room with; none when none arrived (yet, or it could not be read).
    #[cfg(test)]
    pub(crate) fn rows(&self, layer: &str) -> Option<usize> {
        self.shapes.get(layer).map(|(_, s)| s.len())
    }

    /// Forgets what arrived, not what was asked: the bands are not read again (a picture of the part alone).
    #[cfg(test)]
    pub(crate) fn forget(&mut self) {
        self.shapes.clear();
    }
}

impl App {
    /// After every message: the bands the open drawing's parts need are asked for, the ones that arrived are put
    /// back after the store was cleared, the ones of a layer no longer a part are dropped.
    pub(crate) fn follow_bands(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            if self.bands.session.is_some() {
                self.bands = Bands::default();
                self.spatial.clear_all_ghosts();
            }
            return Task::none();
        };
        if self.bands.session != Some(doc.session) {
            self.bands = Bands {
                session: Some(doc.session),
                ..Bands::default()
            };
            self.spatial.clear_all_ghosts();
        }
        let session = doc.session;
        let mut tasks: Vec<Task<Message>> = Vec::new();
        let mut parts: HashSet<String> = HashSet::new();
        for link in &doc.settings().sources {
            let Some(extent) = link.extent else {
                continue;
            };
            parts.insert(link.layer.clone());
            let key: Key = (
                link.revision.clone(),
                extent.map(f64::to_bits),
                band_shown_key(&doc.model, link),
            );
            if self.bands.asked.get(&link.layer) != Some(&key) {
                self.bands.asked.insert(link.layer.clone(), key.clone());
                self.bands.unread.remove(&link.layer);
                let Some(job) = band_job(&doc.model, link) else {
                    continue;
                };
                let (layer, asked) = (link.layer.clone(), key);
                let (done, wait) = oneshot::channel();
                std::thread::spawn(move || {
                    let _ = done.send(job.read());
                });
                tasks.push(Task::perform(wait, move |result| {
                    Message::BandRead(Read {
                        session,
                        layer: layer.clone(),
                        key: asked.clone(),
                        result: result.unwrap_or_else(|_| Err("okuma yarıda kaldı".to_owned())),
                    })
                }));
            }
            // A store that was cleared (the drawing reloaded) has none of them: they are given again.
            if let Some((_, shapes)) = self.bands.shapes.get(&link.layer)
                && self.spatial.ghost_count(&link.layer) != shapes.len()
            {
                self.spatial.set_ghosts(&link.layer, shapes.clone());
            }
        }
        // A layer that is no part any more has no band.
        let gone: Vec<String> = self
            .bands
            .asked
            .keys()
            .filter(|l| !parts.contains(*l))
            .cloned()
            .collect();
        for layer in gone {
            self.bands.asked.remove(&layer);
            self.bands.unread.remove(&layer);
            if self.bands.shapes.remove(&layer).is_some() {
                self.spatial.set_ghosts(&layer, Vec::new());
            }
        }
        Task::batch(tasks)
    }

    /// A band arrived: its rows hold room for the labels now. An answer to a question that is not the current one (the
    /// drawing, the source or the part changed since) is let go.
    pub(crate) fn band_read(&mut self, read: Read) {
        if self.bands.session != Some(read.session)
            || self.bands.asked.get(&read.layer) != Some(&read.key)
        {
            return;
        }
        match read.result {
            Ok(shapes) => {
                self.spatial.set_ghosts(&read.layer, shapes.clone());
                self.bands.shapes.insert(read.layer, (read.key, shapes));
            }
            Err(why) => {
                self.spatial.set_ghosts(&read.layer, Vec::new());
                self.bands.shapes.remove(&read.layer);
                if self
                    .bands
                    .unread
                    .insert(read.layer.clone(), why.clone())
                    .is_none()
                {
                    self.say(
                        Level::Info,
                        format!(
                            "{}: parçanın çevresindeki komşular okunamadı ({why}); kenardaki etiketler yalnız parçanın nesnelerine göre yerleşir.",
                            read.layer
                        ),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::band_scenes::{open, sides};
    use crate::files_testing::app_with_drawing;

    /// The rows beside a part are read as obstacles only: the layer holds the first parcel, the store holds the second
    /// as a ghost, and a survey point's name just inside the part's east edge keeps off it (docs/adr/0578 §1).
    #[test]
    fn the_rows_round_a_part_hold_room_for_the_names_at_its_edge() {
        let mut app = app_with_drawing();
        open(&mut app, "bant-test");
        let layer = "import-parsel";
        assert_eq!(
            app.bands.rows(layer),
            Some(1),
            "the second parcel is the band's row"
        );
        assert_eq!(app.spatial.ghost_count(layer), 1);
        // Ghosts are no objects: the store holds the first parcel, the five points and the neighbour the scene draws.
        assert_eq!(app.spatial.len(), 7);
        let with = sides(&app);
        // Without the band: the part alone, as before.
        app.bands.forget();
        app.spatial.clear_all_ghosts();
        let without = sides(&app);
        // The three names beside the neighbour went east over it; with it, west.
        let east = |v: &[(i64, i64)]| v.iter().map(|d| d.0 > 0).collect::<Vec<_>>();
        assert_eq!(east(&without), [true; 5]);
        assert_eq!(east(&with), [true, true, false, false, false]);
        assert_ne!(with, without, "the neighbours move a name at the edge");
        // A drawing that is another one has none of them.
        let other = app_with_drawing();
        assert_eq!(other.spatial.ghost_count(layer), 0);
    }

    /// A part that does not hold the pinned parcel any more parks its pins in the binding (docs/adr/0578 §3), the band
    /// is asked for again with the new area, and Denetle says both.
    #[test]
    fn a_narrowed_part_parks_the_pins_and_asks_for_its_band_again() {
        let mut app = app_with_drawing();
        crate::band_scenes::park(&mut app, "bant-park-test");
        let doc = app.document.as_ref().expect("a drawing");
        let link = &doc.model.settings().sources[0];
        assert_eq!(link.parked.len(), 1);
        assert_eq!(link.parked[0].pins.len(), 1);
        assert_eq!(
            doc.model.by_layer_with_uids("import-parsel").count(),
            0,
            "the part holds nothing"
        );
        // The new band reaches no row: asked again, nothing in it.
        assert_eq!(app.bands.rows("import-parsel"), Some(0));
        assert_eq!(app.spatial.ghost_count("import-parsel"), 0);
        app.open_labelling(Some("import-parsel".to_owned()));
        let _ = app.update(crate::app::Message::Labelling(
            crate::labelling::Event::Report,
        ));
        let lines = &app.labelling.as_ref().expect("the window").lines;
        assert!(
            lines
                .iter()
                .any(|(_, t)| t.contains("1 nesnenin 1 etiket iğnesi park edilmiş")),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|(_, t)| t.contains("çevresindeki 0 satır engel")),
            "{lines:?}"
        );
    }
}
