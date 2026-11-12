//! XYZ sor on the desktop (docs/adr/0207 §7): the tool's place read off the
//! interface's thread against the shown clouds under it: every node near the
//! place (`geom::pointcloud::near`) of every opened file read at full
//! resolution, the nearest point in plan within the tool's reach (8 pixels)
//! kept (`ops::query`), its record's facts said on the command line and the
//! point marked on the drawing.

use std::path::Path;
use std::sync::Arc;

use iced::Task;
use kentos_commands::Entity;
use kentos_geometry_core::geom::pointcloud::near;
use kentos_interaction::{Format, Level};
use kentos_pointcloud::copc::Key;
use kentos_pointcloud::ops::query::{Facts, nearest};

use super::Event as Clouds;
use super::service::{Entry, State, service};
use crate::app::{App, Message};

/// The nearest point of the cloud `entry` to `at` within `reach`: its squared distance and facts.
fn nearest_in(entry: &Entry, at: [f64; 2], reach: f64) -> Result<Option<(f64, Facts)>, String> {
    let mut best: Option<(f64, Facts)> = None;
    let mut waiting = false;
    let mut nodes = Vec::new();
    let mut records = Vec::new();
    for m in &entry.members {
        let o = match m.state() {
            State::Ready(o) => o,
            State::Failed(why) => return Err(why),
            State::Waiting | State::Indexing(_) | State::Stopped => {
                waiting = true;
                continue;
            }
        };
        // docs/adr/5100: a placed file is searched in its own coordinates about the place taken back (at the heights'
        // middle), its reach grown by how far a tilt moves a point across the heights; its points compared where the
        // drawing shows them.
        let (look_at, look_reach) = match &m.pose {
            Some(p) => {
                let zm = (entry.z[0] + entry.z[1]) / 2.0;
                let d = [at[0] - p[3], at[1] - p[7], zm - p[11]];
                let back = [0, 1].map(|a| p[a] * d[0] + p[4 + a] * d[1] + p[8 + a] * d[2]);
                let lean = (p[8] * p[8] + p[9] * p[9]).sqrt() * (entry.z[1] - entry.z[0]).abs();
                (back, reach + lean)
            }
            None => (at, reach),
        };
        near(&o.tree, look_at, look_reach, &mut nodes);
        for &n in &nodes {
            let k = o.tree.keys[n as usize];
            let Some(run) = o.cloud.node_run(Key {
                d: k[0],
                x: k[1],
                y: k[2],
                z: k[3],
            }) else {
                continue;
            };
            let bytes = o.bytes.read(run.need.offset, run.need.len)?;
            records.clear();
            o.cloud
                .records(&run, &bytes, &mut records)
                .map_err(|e| e.0)?;
            let (layout, scale, offset) =
                (&o.cloud.layout, o.cloud.head.scale, o.cloud.head.offset);
            if let Some(p) = &m.pose {
                let r2 = reach * reach;
                for r in records.chunks_exact(layout.len.max(1)) {
                    let mut f = Facts::of(layout, r, scale, offset);
                    f.xyz = kentos_commands::pose_apply(p, f.xyz);
                    let d = (f.xyz[0] - at[0]).powi(2) + (f.xyz[1] - at[1]).powi(2);
                    if d <= r2 && best.as_ref().is_none_or(|(b, _)| d < *b) {
                        best = Some((d, f));
                    }
                }
                continue;
            }
            if let Some((i, d)) = nearest(layout, &records, scale, offset, at, reach)
                && best.as_ref().is_none_or(|(b, _)| d < *b)
            {
                let r = &records[i * layout.len..(i + 1) * layout.len];
                best = Some((d, Facts::of(layout, r, scale, offset)));
            }
        }
    }
    if best.is_none() && waiting {
        return Err("bulutun dizini hazırlanıyor; hazır olunca yeniden deneyin.".into());
    }
    Ok(best)
}

/// The words of a point's facts, the coordinates in the project's order.
pub fn words(f: &Facts, format: &Format) -> String {
    let [x, y, z] = f.xyz;
    let mut out = format!(
        "{}={}, {}={}, Z={}; sınıf {} ({}); yoğunluk {}; dönüş {}/{}",
        format.east_label(),
        format.coord(x),
        format.north_label(),
        format.coord(y),
        format.length_bare(z),
        f.class,
        kentos_pointcloud::classes::name(f.class),
        f.intensity,
        f.returns.0,
        f.returns.1,
    );
    if let Some([r, g, b]) = f.rgb {
        out.push_str(&format!("; renk {r}, {g}, {b}"));
    }
    if let Some(t) = f.gps_time {
        out.push_str(&format!("; GPS zamanı {}", crate::crs::js_number(t)));
    }
    out.push_str(&format!("; kaynak kimliği {}", f.source_id));
    out
}

impl App {
    /// XYZ sor's places: the shown clouds under each read off the thread.
    pub(crate) fn cloud_query_tasks(&mut self) -> Task<Message> {
        let wanted = std::mem::take(&mut self.cloud_query_wanted);
        Task::batch(
            wanted
                .into_iter()
                .map(|(p, reach)| self.cloud_query_at([p.x, p.y], reach))
                .collect::<Vec<_>>(),
        )
    }

    fn cloud_query_at(&mut self, at: [f64; 2], reach: f64) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let format = Format::of(doc.settings());
        let layers = doc.model.layers();
        let folder = doc.path.as_deref().and_then(Path::parent);
        let mut clouds: Vec<(String, Arc<Entry>)> = Vec::new();
        for e in doc.model.entities() {
            let Entity::PointCloud(c) = e else {
                continue;
            };
            if !layers.is_visible(&c.base.layer_id) {
                continue;
            }
            let [x1, y1, x2, y2] = c.cloud.rect();
            if at[0] < x1 - reach || at[0] > x2 + reach || at[1] < y1 - reach || at[1] > y2 + reach
            {
                continue;
            }
            let key = super::key_of(&c.cloud);
            if service().entry(&key).is_none() {
                let z = [c.cloud.bounds[2], c.cloud.bounds[5]];
                service().register(&key, || super::files_of(&c.cloud, &doc.model, folder), z);
            }
            if let Some(entry) = service().entry(&key) {
                clouds.push((self.layer_name(&c.base.layer_id), entry));
            }
        }
        if clouds.is_empty() {
            self.say(Level::Info, "Burada gösterilen nokta bulutu yok.");
            return Task::none();
        }
        let task = super::super::rasters::off_thread(move || {
            let mut best: Option<(f64, String, Facts)> = None;
            let mut lines = Vec::new();
            for (layer, entry) in clouds {
                match nearest_in(&entry, at, reach) {
                    Ok(Some((d, f))) => {
                        if best.as_ref().is_none_or(|(b, ..)| d < *b) {
                            best = Some((d, layer, f));
                        }
                    }
                    Ok(None) => {}
                    Err(why) => lines.push(format!("Nokta bulutu {layer}: {why}")),
                }
            }
            let found = best.as_ref().map(|(_, _, f)| f.xyz);
            match best {
                Some((_, layer, f)) => {
                    lines.insert(0, format!("Nokta bulutu {layer}: {}", words(&f, &format)))
                }
                None if lines.is_empty() => {
                    lines.push("Tıklanan yerin 8 piksel yakınında nokta yok.".to_owned())
                }
                None => {}
            }
            Message::PointClouds(Clouds::Queried(lines, found))
        });
        self.tool_reply(task)
    }

    /// XYZ sor's answer: its lines on the command line, the point marked.
    pub(crate) fn cloud_queried(&mut self, lines: Vec<String>, found: Option<[f64; 3]>) {
        for l in lines {
            self.say(Level::Info, l);
        }
        self.with_tool(|s, cx| s.cloud_found(found, cx));
    }
}
