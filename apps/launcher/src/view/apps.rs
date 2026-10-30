//! Uygulamalar (docs/adr/0803): the plates of the apps that are out, the
//! recent drawings and the update centre, the apps still to come. While
//! something is searched, only what matches: the plates and the coming apps.
//!
//! Every part has a height of its own, so the page knows before it is laid
//! out whether it has to scroll: only then it leaves the scroll bar its room.

use iced::widget::{Column, Row, Space, column, container, row, scrollable};
use iced::{Element, Fill, Length, Size};
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::theme::typography;
use kentos_ui::widget::{EmptyState, Responsive};

use crate::app::{Launcher, Message, Page};
use crate::cards::{self, Status};
use crate::manifest::Manifest;
use crate::view::plates::{coming, coming_height, plate, plate_height};
use crate::view::style::{self, px};
use crate::view::tables;

/// The most recent drawings the page lists, and the most versions.
const RECENTS: usize = 4;
const VERSIONS: usize = 4;
/// The update centre's sentence under its versions: two lines of captions.
const NOTE_LINES: f32 = 2.0;

/// What the page is made of, for its height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parts {
    /// The apps on plates and those still to come.
    pub out: usize,
    pub coming: usize,
    /// The recent drawings' and the versions' rows; none while something is searched.
    pub tables: Option<(usize, usize)>,
}

/// How the page is laid out in `width` × `height`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    pub columns: usize,
    /// A plate's width.
    pub cell: f32,
    pub scrolls: bool,
    /// The width the plates fill, to the pixel.
    pub width: f32,
}

impl Plan {
    fn of(width: f32, scrolls: bool, columns: usize, cell: f32) -> Self {
        Self {
            columns,
            cell,
            scrolls,
            width,
        }
    }

    /// The gap before a column: the same between all plates, but for the
    /// pixels whole plates leave over, one each to the first gaps. So the last
    /// column ends at the page's edge, under the header's last control.
    fn gap_before(&self, column: usize) -> f32 {
        let gap = px(style::GAP);
        let n = self.columns as f32;
        let spare = self.width - n * self.cell - (n - 1.0) * gap;
        gap + if (column as f32) <= spare { 1.0 } else { 0.0 }
    }

    /// Where a column starts.
    fn left_of(&self, column: usize) -> f32 {
        (1..=column).map(|c| self.cell + self.gap_before(c)).sum()
    }
}

/// The update centre's height with `versions` rows.
fn centre_height(versions: usize) -> f32 {
    tables::lines_height(versions.max(1))
        + 1.0
        + px(6.0)
        + (NOTE_LINES * typography::caption() * 1.3).ceil()
}

/// The page's height with plates of `cell` pixels in `columns` columns.
fn tall(parts: Parts, columns: usize, cell: f32, gap: f32) -> f32 {
    let section = px(style::SECTION);
    let mut sections = Vec::new();
    if parts.out > 0 {
        let rows = parts.out.div_ceil(columns) as f32;
        sections.push(rows * plate_height(cell) + (rows - 1.0) * gap);
    }
    if let Some((recents, versions)) = parts.tables {
        let recents = px(style::HEAD) + tables::lines_height(recents.max(1));
        let versions = px(style::HEAD) + centre_height(versions);
        sections.push(if columns >= 3 {
            recents.max(versions)
        } else {
            recents + section + versions
        });
    }
    if parts.coming > 0 {
        let rows = parts.coming.div_ceil(columns) as f32;
        sections.push(px(style::HEAD) + rows * coming_height() + (rows - 1.0) * gap);
    }
    sections.iter().sum::<f32>() + section * sections.len().saturating_sub(1) as f32
}

/// The page's layout: its columns, a plate's width and whether it scrolls
/// (and leaves the scroll bar its room).
pub fn plan(width: f32, height: f32, parts: Parts) -> Plan {
    let gap = px(style::GAP);
    let lay = |width: f32| {
        let columns = cards::columns(
            width,
            parts.out,
            px(style::PLATE_COMFORTABLE),
            px(style::PLATE_LEAST),
            gap,
        );
        (columns, cards::card_width(width, columns, gap))
    };
    let (columns, cell) = lay(width);
    if tall(parts, columns, cell, gap) <= height {
        return Plan::of(width, false, columns, cell);
    }
    let width = width - px(style::SCROLLBAR);
    let (columns, cell) = lay(width);
    Plan::of(width, true, columns, cell)
}

pub fn page(launcher: &Launcher) -> Element<'_, Message> {
    if launcher.visible().is_empty() {
        return container(
            EmptyState::new(
                Icon::Search,
                format!("“{}” ile eşleşen uygulama yok", launcher.query.trim()),
            )
            .description("Aramayı kısaltın ya da başka bir sözcük deneyin; ad ve açıklama aranır.")
            .secondary("Aramayı temizle", Message::Search(String::new())),
        )
        .width(Fill)
        .height(Fill)
        .into();
    }
    Responsive::new(move |size: Size| {
        let (out, soon) = launcher.shelves();
        let whole = launcher.query.trim().is_empty();
        let versions = installed(launcher);
        let parts = Parts {
            out: out.len(),
            coming: soon.len(),
            tables: whole.then_some((
                launcher.recent.len().min(RECENTS),
                versions.len().min(VERSIONS),
            )),
        };
        let plan = plan(size.width, size.height, parts);
        launcher.columns.set(plan.columns);
        let mut sections = Column::new().spacing(px(style::SECTION));
        if !out.is_empty() {
            sections = sections.push(grid(
                out.iter().map(|app| plate(launcher, app, plan.cell)),
                plan,
            ));
        }
        if whole {
            sections = sections.push(work(launcher, &versions, plan));
        }
        if !soon.is_empty() {
            sections = sections.push(tables::section(
                "Yakında",
                None,
                grid(
                    soon.iter().map(|app| coming(launcher, app, plan.cell)),
                    plan,
                ),
            ));
        }
        if plan.scrolls {
            scrollable(sections)
                .direction(kentos_ui::style::field::body_scrollbar())
                .style(style::scroller)
                .width(Fill)
                .height(Fill)
                .into()
        } else {
            container(sections).width(Fill).height(Fill).into()
        }
    })
    .into()
}

/// The apps that have a version to say: those installed or running.
fn installed(launcher: &Launcher) -> Vec<&Manifest> {
    launcher
        .registry
        .apps
        .iter()
        .filter(|a| matches!(launcher.status(a), Status::Installed | Status::Running(_)))
        .collect()
}

/// `items` in the plan's rows.
fn grid<'a>(items: impl Iterator<Item = Element<'a, Message>>, plan: Plan) -> Element<'a, Message> {
    let mut rows = Column::new().spacing(px(style::GAP));
    let mut items = items.peekable();
    while items.peek().is_some() {
        let mut line = Row::new();
        for (column, item) in items.by_ref().take(plan.columns).enumerate() {
            if column > 0 {
                line = line.push(Space::new().width(plan.gap_before(column)));
            }
            line = line.push(item);
        }
        rows = rows.push(line);
    }
    rows.into()
}

/// The recent drawings and the update centre: side by side where the page has
/// three columns or more (the centre over the last plate), one under the other else.
fn work<'a>(launcher: &'a Launcher, versions: &[&'a Manifest], plan: Plan) -> Element<'a, Message> {
    if plan.columns >= 3 {
        let last = plan.columns - 1;
        let gap = plan.gap_before(last);
        let recents_width = plan.left_of(last) - gap;
        row![
            container(recents(launcher, recents_width)).width(Length::Fixed(recents_width)),
            Space::new().width(gap),
            container(centre(launcher, versions)).width(Length::Fixed(plan.cell)),
        ]
        .into()
    } else {
        column![recents(launcher, plan.width), centre(launcher, versions)]
            .spacing(px(style::SECTION))
            .into()
    }
}

/// The last drawings: the quickest way back to work.
fn recents(launcher: &Launcher, width: f32) -> Element<'_, Message> {
    let body = if launcher.recent.is_empty() {
        tables::empty_line("Henüz açılan çizim yok. Bir uygulamayı başlatıp çizim açın.")
    } else {
        tables::lines(
            launcher
                .recent
                .iter()
                .enumerate()
                .take(RECENTS)
                .map(|(i, row)| tables::recent_row(launcher, i, row, width)),
        )
    };
    // An empty list has nothing more to show.
    let whole = (!launcher.recent.is_empty()).then_some(Page::Recent);
    tables::section("Son dosyalar", whole, body)
}

/// The update centre: the versions installed, and plainly that nothing checks
/// for updates yet (docs/adr/0800 §6).
fn centre<'a>(launcher: &'a Launcher, versions: &[&'a Manifest]) -> Element<'a, Message> {
    let list = if versions.is_empty() {
        tables::empty_line("Kurulu uygulama yok.")
    } else {
        tables::lines(
            versions
                .iter()
                .take(VERSIONS)
                .map(|app| tables::version_row(launcher, app)),
        )
    };
    let note = container(
        label::caption("Denetim henüz yok; yeni sürümler kurulum paketiyle gelir.")
            .style(style::soft),
    )
    .padding(iced::Padding {
        top: px(6.0),
        left: px(style::INSET),
        right: px(style::INSET),
        bottom: 0.0,
    });
    tables::section(
        "Güncelleme merkezi",
        Some(Page::Updates),
        column![list, tables::rule(), note],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIX: Parts = Parts {
        out: 3,
        coming: 3,
        tables: Some((4, 3)),
    };

    #[test]
    fn the_page_fits_at_1440_and_scrolls_at_the_smallest_window() {
        // At 1440 × 900 and 13 px the page is 1168 wide and about 730 high:
        // the three plates on a row, nothing scrolls.
        let wide = plan(1168.0, 730.0, SIX);
        assert_eq!(wide.columns, 3);
        assert_eq!(wide.cell, 384.0);
        assert!(!wide.scrolls, "needs {}", tall(SIX, 3, 384.0, 8.0));
        // At 1100 × 650 still the three on a row: the page scrolls, and leaves the bar its room.
        let small = plan(828.0, 480.0, SIX);
        assert_eq!((small.columns, small.cell), (3, 266.0));
        assert!(small.scrolls);
        // Searching leaves the plates alone: nothing to scroll for.
        let found = Parts {
            out: 1,
            coming: 0,
            tables: None,
        };
        assert!(!plan(1168.0, 730.0, found).scrolls);
    }

    #[test]
    fn the_last_plate_ends_at_the_pages_edge() {
        // 1168 divides evenly: every gap alike.
        let even = plan(1168.0, 800.0, SIX);
        assert_eq!(even.left_of(2) + even.cell, 1168.0);
        assert_eq!(even.gap_before(1), even.gap_before(2));
        // Two pixels over: one to each gap, and still the edge.
        let over = plan(1170.0, 800.0, SIX);
        assert_eq!(over.cell, 384.0);
        assert_eq!((over.gap_before(1), over.gap_before(2)), (9.0, 9.0));
        assert_eq!(over.left_of(2) + over.cell, 1170.0);
        // One over: the first gap only.
        let one = plan(1169.0, 800.0, SIX);
        assert_eq!((one.gap_before(1), one.gap_before(2)), (9.0, 8.0));
        assert_eq!(one.left_of(2) + one.cell, 1169.0);
    }

    #[test]
    fn a_plate_is_its_parts_and_its_picture_is_16_to_9() {
        // 368 wide on the plate's ground of 384: the picture 207 high.
        assert_eq!(
            plate_height(384.0),
            16.0 + 207.0 + 12.0 + 40.0 + 10.0 + 28.0
        );
        assert_eq!(plate_height(384.0), plate_height(384.0));
    }
}
