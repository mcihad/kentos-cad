//! The tabs under the drawing and the status bar's cells while a sheet is in
//! front (design §11; the web's `SheetTabs` and `statusCells`): where the
//! pointer is on the paper (Sol, Üst in millimetres: X and Y are ground
//! coordinates in KentOS), the chosen items' size, the zoom, which sheet of
//! how many, and the preflight. They stand where the drawing's cells were.

use iced::Element;
use kentos_sheet::preflight::Severity;
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::widget::{Tab, Tabs, Tip};

use crate::designer::Designer;
use crate::icons;
use crate::message::{InspectorTab, Message};

/// Millimetres with one decimal, by the display rule (ADR 0149: a point, as the web's `fixed`).
fn mm1(um: f64) -> String {
    kentos_geometry_core::display::fixed(um / 1000.0, 1)
}

impl Designer {
    /// “Model | Pafta 1 | Pafta 2 | +”: the model first, never closed.
    pub fn tabs(&self) -> Element<'_, Message> {
        let active = self
            .open
            .as_deref()
            .and_then(|id| self.book.sheet_index(id))
            .map_or(0, |i| i + 1);
        let tabs = std::iter::once(Tab::new("Model").icon(Icon::Layers).closable(false)).chain(
            self.book
                .sheets
                .iter()
                // A dot where the sheet's template has a newer revision (the web's tab).
                .map(|s| {
                    Tab::new(s.name.as_str())
                        .icon(icons::LAYOUT)
                        .note(self.template_newer(s))
                }),
        );
        Tabs::new(tabs, active, Message::Tab)
            .on_close(Message::CloseTab)
            .on_reorder(Message::MoveTab)
            .on_new(Message::NewSheet)
            .bottom()
            .into()
    }

    /// The sheet's cells of the status bar, for the host's bar.
    pub fn status_cells(&self) -> Vec<Element<'_, Message>> {
        let Some(sheet) = self.open_sheet() else {
            return Vec::new();
        };
        let mut cells: Vec<Element<'_, Message>> = Vec::new();
        let (l, t) = self.hover.map_or(("—".to_owned(), "—".to_owned()), |p| {
            (mm1(p[0]), mm1(p[1]))
        });
        cells.push(
            Readout::new(label::mono(format!("Sol {l}  Üst {t} mm")))
                .icon(Icon::Crosshair)
                .width(150.0)
                .tip(
                    Tip::new("İmleç kâğıtta")
                        .body("Kâğıdın sol üst köşesinden milimetre: Sol sağa, Üst aşağı doğru."),
                )
                .into(),
        );
        if let Some(b) = self.chosen_bounds() {
            let n = self.selection.len();
            let text = format!(
                "{}{} × {} mm",
                if n > 1 {
                    format!("{n} öğe · ")
                } else {
                    "Seçim ".to_owned()
                },
                mm1(f64::from(b.width)),
                mm1(f64::from(b.height))
            );
            cells.push(
                Readout::new(label::mono(text))
                    .tip(Tip::new("Seçimin boyu").body(
                        "Seçili öğelerin çerçevelerini kapsayan kutunun genişliği ve yüksekliği.",
                    ))
                    .into(),
            );
        }
        cells.push(
            Readout::new(label::mono(format!(
                "%{}",
                self.view.percent(self.stage, sheet.page.size)
            )))
            .tip(
                Tip::new("Yakınlaştırma")
                    .body("Gerçek boya göre. Ctrl+0 sayfayı sığdırır, Ctrl+1 gerçek boyut."),
            )
            .on_press(Message::ZoomPage)
            .into(),
        );
        let at = self
            .open
            .as_deref()
            .and_then(|id| self.book.sheet_index(id))
            .map_or(0, |i| i + 1);
        cells.push(
            Readout::new(label::body(format!(
                "Sayfa {at}/{}",
                self.book.sheets.len()
            )))
            .tip(Tip::new("Pafta").body("Öndeki paftanın sırası ve paftaların sayısı."))
            .into(),
        );
        let errors = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let warnings = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        let (glyph, text) = if errors > 0 {
            (Icon::Error, format!("{errors} hata"))
        } else if warnings > 0 {
            (Icon::Warning, format!("{warnings} uyarı"))
        } else {
            (Icon::Success, "Ön denetim temiz".to_owned())
        };
        cells.push(
            Readout::new(label::body(text))
                .icon(glyph)
                .on_press(Message::InspectorTab(InspectorTab::Preflight))
                .tip(Tip::new("Ön denetim").body(format!(
                    "{} bulgu: denetçinin Ön denetim sekmesinde, çözümleriyle.",
                    self.findings.len()
                )))
                .into(),
        );
        if let Some(e) = &self.error {
            cells.push(
                Readout::new(label::caption(e.clone()))
                    .icon(Icon::Warning)
                    .into(),
            );
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn millimetres_have_a_point() {
        assert_eq!(super::mm1(24_000.0), "24.0");
        assert_eq!(super::mm1(292_050.0), "292.1");
        assert_eq!(super::mm1(-40.0), "0.0");
    }
}
