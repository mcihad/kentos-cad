//! Sanal bulut olarak kaydet (docs/adr/0207 §8): the selected cloud's files
//! written as a `.vpc` (QGIS's and PDAL wrench's STAC ItemCollection): each
//! member's path relative to the `.vpc`'s folder (an address as it is), its
//! points and 3D bounds in the cloud's system, and its plan's corners in WGS
//! 84 when the system has an EPSG code. An embedded file has no path to
//! write: such a cloud is refused, said why.

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_contracts::PointCloudFields;
use kentos_pointcloud::vpc::{Item, write};

use super::Event as Clouds;
use crate::app::{App, Message, Picker};

/// A member's href from the `.vpc` at `vpc`: a path beside or under its folder relative (`./a.laz`), else absolute.
fn href(file: &Path, vpc: &Path) -> String {
    let folder = vpc.parent().unwrap_or(Path::new(""));
    match file.strip_prefix(folder) {
        Ok(rest) => format!("./{}", rest.to_string_lossy().replace('\\', "/")),
        Err(_) => file.to_string_lossy().into_owned(),
    }
}

/// The `.vpc`'s text for `c`, written at `vpc` (paths beside a linked file resolved from `folder`).
pub fn text_of(c: &PointCloudFields, vpc: &Path, folder: Option<&Path>) -> Result<String, String> {
    let mut items = Vec::with_capacity(c.sources.len());
    for (i, s) in c.sources.iter().enumerate() {
        let href = match (&s.file, &s.url, &s.asset) {
            (Some(f), _, _) => href(&crate::pictures::resolve(f, folder), vpc),
            (None, Some(u), _) => u.clone(),
            _ => {
                return Err(format!(
                    "Bulutun {}. dosyası gömülü; sanal buluta yalnız bağlı dosyalar ve adresler yazılır. Bulutu bağlı dosyalardan ekleyip yeniden deneyin.",
                    i + 1
                ));
            }
        };
        items.push(Item {
            href,
            count: s.count,
            bounds: s.bounds,
            wgs84: kentos_processing::builtin::pointcloud::wgs84_corners(&s.bounds, c.srid),
        });
    }
    let epsg = (c.srid != 0).then_some(c.srid);
    Ok(write(&items, epsg, None))
}

impl App {
    /// `pointcloud.vpcSave`: where to write asked for, for the selected cloud.
    pub(crate) fn cloud_vpc_command(&mut self) -> Task<Message> {
        let clouds = self.selected_clouds();
        let Some((_, c, _)) = clouds.first() else {
            self.warn("Sanal bulut olarak kaydetmek için önce bir nokta bulutu seçin.");
            return Task::none();
        };
        let name = format!(
            "{}.vpc",
            c.sources
                .first()
                .map(|s| super::file_stem(&super::source_name(s, None)))
                .unwrap_or_else(|| "bulut".to_owned())
        );
        if let Picker::File(path) = &self.picker {
            return Task::done(Message::PointClouds(Clouds::VpcSave(Some(path.clone()))));
        }
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("Sanal bulut olarak kaydet")
                    .add_filter("Sanal nokta bulutu (VPC)", &["vpc"])
                    .set_file_name(name)
                    .save_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            |path| Message::PointClouds(Clouds::VpcSave(path)),
        )
    }

    /// The `.vpc` written at `path` off the thread.
    pub(crate) fn cloud_vpc_write(&mut self, path: Option<PathBuf>) -> Task<Message> {
        let Some(mut path) = path else {
            return Task::none();
        };
        if path.extension().is_none() {
            path.set_extension("vpc");
        }
        let Some((_, c, _)) = self.selected_clouds().into_iter().next() else {
            return Task::none();
        };
        let folder = self
            .document
            .as_ref()
            .and_then(|d| d.path.as_deref())
            .and_then(Path::parent)
            .map(Path::to_path_buf);
        crate::rasters::off_thread(move || {
            let result = text_of(&c, &path, folder.as_deref()).and_then(|text| {
                std::fs::write(&path, text)
                    .map(|()| path.clone())
                    .map_err(|e| format!("“{}” yazılamadı: {e}.", path.display()))
            });
            Message::PointClouds(Clouds::VpcSaved(result))
        })
    }
}
