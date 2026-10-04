//! Karne editörü's controls (docs/adr/0169 §6): the file opened, a text
//! book's mapping, the station and its back sight, and Kutupsal alım'a
//! aktar, which fills Kutupsal alım from the station's reduction and opens it.

use iced::Task;
use kentos_interaction::Level;

use super::super::Window;
use super::super::read::{Known, resolve_point};
use super::{Event, LIMIT, exact, fb};
use crate::app::Message;

impl crate::app::App {
    /// Karne editörü's own controls; the station is reduced again after each.
    pub(crate) fn fieldbook_event(&mut self, e: Event) -> Task<Message> {
        let form = &mut self.calc.fieldbook;
        match e {
            Event::Open => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Karne aç")
                            .add_filter(
                                "Karne (.gsi, .txt, .csv, .dat)",
                                &["gsi", "GSI", "txt", "TXT", "csv", "CSV", "dat", "DAT"],
                            )
                            .add_filter("Bütün dosyalar", &["*"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| fb(Event::Picked(path)),
                );
            }
            Event::Picked(None) => {}
            Event::Picked(Some(path)) => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                match std::fs::metadata(&path) {
                    Ok(m) if m.len() > LIMIT => {
                        form.error = Some(format!(
                            "“{name}” karne için çok büyük ({} MiB); en çok 64 MiB okunur.",
                            m.len() >> 20
                        ));
                    }
                    _ => match std::fs::read(&path) {
                        Ok(bytes) => form.open(name, bytes),
                        Err(e) => {
                            form.error = Some(format!(
                                "“{name}” okunamadı ({e}). Dosyanın yerini ve izinlerini denetleyin."
                            ));
                        }
                    },
                }
            }
            Event::Map(i, col) => {
                if let Some(c) = form.mapping.columns.get_mut(i) {
                    *c = col;
                }
                form.read();
            }
            Event::Header(on) => {
                form.mapping.header = on;
                form.read();
            }
            Event::Unit(u) => form.mapping.unit = Some(u),
            Event::Station(i) => form.station = i,
            Event::Back(i) => {
                if let Some(b) = form.backs.get_mut(form.station) {
                    *b = i;
                }
            }
            Event::Transfer => return self.fieldbook_transfer(),
            Event::Fore(j) => form.fore = j,
            Event::TransferTraverse => return self.fieldbook_traverse(),
            Event::Height(t) => {
                if let Some(e) = form.edits.get_mut(form.station) {
                    e.height = t;
                }
            }
        }
        if let Some(doc) = &self.document {
            self.calc.fieldbook.sync(doc.settings());
        }
        Task::none()
    }

    /// Kutupsal alım'a aktar: its fields filled from the station shown (the
    /// station by its name when the drawing has it, else by the file's
    /// coordinates), then it opens; this window waits as it is.
    fn fieldbook_transfer(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let form = &self.calc.fieldbook;
        let Some(t) = form
            .transfer(doc.settings().angle_unit)
            .filter(|t| !t.shots.is_empty())
        else {
            return Task::none();
        };
        let Some((st, height)) = form.station_shown() else {
            return Task::none();
        };
        let station = match (resolve_point(&doc.model, &st.station), st.east, st.north) {
            (Known::Point { .. }, _, _) => st.station.clone(),
            (_, Some(e), Some(n)) => format!("{e},{n}"),
            _ => st.station.clone(),
        };
        let station_z = st.height.map(|h| exact(h, 6)).unwrap_or_default();
        let height = height.to_owned();
        let polar = &mut self.calc.polar;
        polar.station = station;
        polar.back = t.back.clone();
        polar.back_reading = exact(t.back_reading, 8);
        polar.station_z = station_z;
        polar.instrument_height = height;
        polar.rows = t
            .shots
            .iter()
            .map(|s| {
                [
                    s.name.clone(),
                    exact(s.reading, 8),
                    exact(s.slope, 6),
                    exact(s.zenith, 8),
                    s.target_height.map(|h| exact(h, 6)).unwrap_or_default(),
                ]
            })
            .collect();
        self.say(
            Level::Success,
            format!(
                "Karne editörü: {} nokta Kutupsal alım'a aktarıldı (geri bakış {}).",
                t.shots.len(),
                t.back
            ),
        );
        if !t.left.is_empty() {
            self.warn(format!(
                "Uzunluğu ya da başucu açısı olmayan {} doğrultu aktarılmadı: {}.",
                t.left.len(),
                t.left.join(", ")
            ));
        }
        self.calc_show(Window::Polar);
        Task::none()
    }

    /// Poligon hesabı'na aktar: a connected traverse through the stations,
    /// from the first (oriented on its back sight) to the last (on the fore
    /// sight chosen, if any), each station's angle and each leg's mean
    /// distance; the stations by their names when the drawing has them, else
    /// by the file's coordinates. Poligon hesabı opens; this window waits.
    fn fieldbook_traverse(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let form = &self.calc.fieldbook;
        let (Some(t), Some((back, fore)), Some(book)) =
            (form.traverse(), form.traverse_ends(), form.book.as_ref())
        else {
            return Task::none();
        };
        let n = t.stations.len();
        let place = |i: usize| {
            let st = &book.stations[i];
            match (resolve_point(&doc.model, &st.station), st.east, st.north) {
                (Known::Point { .. }, _, _) => st.station.clone(),
                (_, Some(e), Some(n)) => format!("{e},{n}"),
                _ => st.station.clone(),
            }
        };
        let angle = |i: usize| {
            t.angles
                .get(i)
                .copied()
                .flatten()
                .map(|a| exact(a, 8))
                .unwrap_or_default()
        };
        let leg = |i: usize| {
            t.legs
                .get(i)
                .and_then(|l| l.mean)
                .map(|d| exact(d, 6))
                .unwrap_or_default()
        };
        let (start, end) = (place(0), place(n - 1));
        let back = back.to_owned();
        let fore = fore.map(str::to_owned);
        let first = [String::new(), angle(0), leg(0)];
        let rows: Vec<[String; 3]> = (1..n - 1)
            .map(|i| [t.stations[i].clone(), angle(i), leg(i)])
            .collect();
        let last_angle = if fore.is_some() {
            angle(n - 1)
        } else {
            String::new()
        };
        let missing: Vec<String> = t
            .missing
            .iter()
            .map(|m| format!("{} istasyonunda {}", m.station, m.target))
            .collect();
        let traverse = &mut self.calc.traverse;
        traverse.kind = super::super::traverse::Kind::Connected;
        traverse.end_oriented = fore.is_some();
        traverse.start = start;
        traverse.back = back;
        traverse.end = end;
        traverse.fore = fore.unwrap_or_default();
        traverse.first = first;
        traverse.rows = rows;
        traverse.last = [String::new(), last_angle, String::new()];
        self.say(
            Level::Success,
            format!("Karne editörü: {n} istasyonlu poligon Poligon hesabı'na aktarıldı."),
        );
        if !missing.is_empty() {
            self.warn(format!(
                "Poligonda bulunamayan gözlemler: {}. Açısı ya da kenarı olmayan satırları Poligon hesabı'nda yazın.",
                missing.join("; ")
            ));
        }
        self.calc_show(Window::Traverse);
        Task::none()
    }
}
