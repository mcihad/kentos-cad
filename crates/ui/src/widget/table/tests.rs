//! The table as it is seen and pressed: real events on the real widget tree, read back from its pixels.

use iced::{Element, Point, Size, Theme};

use super::{Column, Row, SortOrder, Table, cell_pads, number, row_height, text};
use crate::snapshot::{Image, Input, Snapshot};
use crate::theme::{Tokens, metrics, typography};
use crate::widget::{Cell, bare_field};

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    Sort(usize),
    Width(usize, f32),
    Fit(usize),
    Press(usize),
}

#[derive(Default)]
struct Model {
    log: Vec<Msg>,
}

const WIDTHS: [f32; 3] = [100.0, 80.0, 60.0];

fn view(_: &Model) -> Element<'_, Msg> {
    Table::new([
        Column::new("Ad")
            .width(WIDTHS[0])
            .sortable(None, Msg::Sort(0))
            .resizable(|width| Msg::Width(0, width))
            .on_fit(Msg::Fit(0)),
        Column::new("Tür")
            .width(WIDTHS[1])
            .sortable(Some(SortOrder::Ascending), Msg::Sort(1)),
        Column::new("Alan").width(WIDTHS[2]).align_right(),
    ])
    .push(
        Row::new([
            text("Kadıköy").into(),
            text("Konut").into(),
            number("12.50").into(),
        ])
        .on_press(Msg::Press(0)),
    )
    .push(Row::new([
        text("Moda").into(),
        text("Park").into(),
        number("7.25").into(),
    ]))
    .into()
}

fn run(model: &mut Model, snapshot: &mut Snapshot, input: Input) {
    let mut update = |model: &mut Model, message: Msg| model.log.push(message);
    snapshot.input(model, view, &mut update, input);
}

fn pixel(image: &Image, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * image.width + x) * 4) as usize;
    [image.rgba[i], image.rgba[i + 1], image.rgba[i + 2]]
}

fn near(a: [u8; 3], b: iced::Color) -> bool {
    let b = [b.r, b.g, b.b].map(|c| (c * 255.0).round() as i32);
    a.iter().zip(b).all(|(a, b)| (i32::from(*a) - b).abs() <= 3)
}

/// The columns' right edges in pixels.
fn edges() -> [f32; 3] {
    let mut at = 0.0;
    let mut out = [0.0; 3];
    for (i, width) in WIDTHS.iter().enumerate() {
        let (left, right) = cell_pads(i, WIDTHS.len());
        at += typography::scaled(*width) + left + right;
        out[i] = at;
    }
    out
}

#[test]
fn the_head_is_a_band_of_its_own_over_a_heavier_line() {
    let mut snapshot = Snapshot::new(Size::new(480.0, 160.0)).expect("çizici kurulamadı");
    let theme = Theme::Dark;
    let t = Tokens::of(&theme);
    let image = snapshot.render(view(&Model::default()), &theme);
    let band = metrics::row() as u32;
    let x = 470;
    assert!(near(pixel(&image, x, band / 2), t.table_head()), "the band");
    assert!(near(pixel(&image, x, band), t.border_strong()), "its line");
    let first_row = band + 1 + band / 2;
    assert!(
        !near(pixel(&image, x, first_row), t.table_head()),
        "a row is not on the band"
    );
    assert_ne!(
        pixel(&image, x, band),
        pixel(&image, x, band + band),
        "the line under the head is heavier than a row's"
    );
}

#[test]
fn rows_are_as_high_as_the_head_with_a_line_under_each() {
    let mut snapshot = Snapshot::new(Size::new(480.0, 160.0)).expect("çizici kurulamadı");
    let theme = Theme::Dark;
    let image = snapshot.render(view(&Model::default()), &theme);
    let row = metrics::row() as u32;
    let x = 470;
    // The head and its line, then rows of `row` whose last pixel is their line.
    let line_one = pixel(&image, x, row + row);
    let line_two = pixel(&image, x, row + row + row);
    assert_eq!(line_one, line_two);
    assert_ne!(line_one, pixel(&image, x, row + row - 2));
    assert_eq!(row_height(), metrics::row());
}

#[test]
fn a_press_anywhere_on_a_head_cell_sorts_and_only_there() {
    let mut snapshot = Snapshot::new(Size::new(480.0, 160.0)).expect("çizici kurulamadı");
    let mut model = Model::default();
    let [first, second, third] = edges();
    let middle = metrics::row() / 2.0;
    // Far from the title, on the cell's empty end.
    run(
        &mut model,
        &mut snapshot,
        Input::Click(Point::new(first - 12.0, middle)),
    );
    assert_eq!(model.log, [Msg::Sort(0)]);
    model.log.clear();
    run(
        &mut model,
        &mut snapshot,
        Input::Click(Point::new(second - 12.0, middle)),
    );
    assert_eq!(model.log, [Msg::Sort(1)]);
    model.log.clear();
    // A column that does not sort; the band past the last column.
    run(
        &mut model,
        &mut snapshot,
        Input::Click(Point::new(third - 12.0, middle)),
    );
    run(
        &mut model,
        &mut snapshot,
        Input::Click(Point::new(third + 20.0, middle)),
    );
    assert_eq!(model.log, []);
}

#[test]
fn an_edge_dragged_gives_the_new_width_and_a_double_click_fits() {
    let mut snapshot = Snapshot::new(Size::new(480.0, 160.0)).expect("çizici kurulamadı");
    let mut model = Model::default();
    let [first, ..] = edges();
    let middle = metrics::row() / 2.0;
    let at = Point::new(first, middle);
    run(&mut model, &mut snapshot, Input::Move(at));
    run(&mut model, &mut snapshot, Input::Press(at));
    run(
        &mut model,
        &mut snapshot,
        Input::Move(Point::new(first + 30.0, middle + 40.0)),
    );
    run(
        &mut model,
        &mut snapshot,
        Input::Release(Point::new(first + 30.0, middle + 40.0)),
    );
    let width = typography::unscaled(typography::scaled(WIDTHS[0]) + 30.0);
    assert_eq!(model.log, [Msg::Width(0, width)], "no sort with the drag");
    assert_eq!(
        typography::scaled(width),
        typography::scaled(WIDTHS[0]) + 30.0,
        "the width given comes back to the pixel"
    );
    model.log.clear();
    // Too narrow is not taken: the column keeps room for its title's start.
    run(&mut model, &mut snapshot, Input::Press(at));
    run(
        &mut model,
        &mut snapshot,
        Input::Move(Point::new(first - 400.0, middle)),
    );
    run(
        &mut model,
        &mut snapshot,
        Input::Release(Point::new(first - 400.0, middle)),
    );
    assert_eq!(
        model.log,
        [Msg::Width(0, typography::unscaled(24.0))],
        "a column never goes below the least"
    );
    model.log.clear();
    run(&mut model, &mut snapshot, Input::Click(at));
    run(&mut model, &mut snapshot, Input::Click(at));
    assert_eq!(model.log, [Msg::Fit(0)]);
}

#[test]
fn a_right_aligned_title_stands_over_its_numbers_with_the_arrow_or_without() {
    fn table(order: Option<SortOrder>) -> Element<'static, Msg> {
        Table::new([Column::new("Alan")
            .width(100.0)
            .align_right()
            .sortable(order, Msg::Sort(0))])
        .push(Row::new([number("12.50").into()]))
        .into()
    }
    let last_ink = |image: &Image, ground: iced::Color, y: std::ops::Range<u32>| {
        (0..image.width)
            .rev()
            .find(|&x| y.clone().any(|y| !near(pixel(image, x, y), ground)))
    };
    let mut snapshot = Snapshot::new(Size::new(160.0, 80.0)).expect("çizici kurulamadı");
    let theme = Theme::Dark;
    let t = Tokens::of(&theme);
    let row = metrics::row() as u32;
    let band = t.table_head();
    let plain = snapshot.render(table(None), &theme);
    let sorted = snapshot.render(table(Some(SortOrder::Descending)), &theme);
    // Only on the title's lines, not the band's last pixels.
    let inner = 6..row - 6;
    assert_eq!(
        last_ink(&plain, band, inner.clone()),
        last_ink(&sorted, band, inner.clone()),
        "the arrow does not move the title"
    );
    // The arrow is all that differs; thin lines on the band never reach the accent itself, but they lean to it.
    let leaning: Vec<u32> = (0..plain.width)
        .filter(|&x| {
            inner.clone().any(|y| {
                let (a, b) = (pixel(&plain, x, y), pixel(&sorted, x, y));
                a != b && b[2] > b[0] + 30
            })
        })
        .collect();
    assert!(!leaning.is_empty(), "the sorted column's arrow in the accent");
}

#[test]
fn a_row_lights_under_the_pointer_even_if_it_cannot_be_pressed() {
    let mut snapshot = Snapshot::new(Size::new(480.0, 160.0)).expect("çizici kurulamadı");
    let mut model = Model::default();
    let theme = Theme::Dark;
    let row = metrics::row();
    let at = |line: f32| Point::new(460.0, row + 1.0 + row * line + row / 2.0);
    let before = snapshot.render(view(&model), &theme);
    run(&mut model, &mut snapshot, Input::Move(at(1.0)));
    let after = snapshot.render(view(&model), &theme);
    let px = |image: &Image, line: f32| {
        let p = at(line);
        pixel(image, p.x as u32, p.y as u32)
    };
    assert_ne!(px(&before, 1.0), px(&after, 1.0), "the row under the pointer");
    assert_eq!(px(&before, 0.0), px(&after, 0.0), "not the other one");
    // A press on it says nothing: it has no message.
    run(&mut model, &mut snapshot, Input::Click(at(1.0)));
    assert_eq!(model.log, []);
    run(&mut model, &mut snapshot, Input::Click(at(0.0)));
    assert_eq!(model.log, [Msg::Press(0)]);
}

#[test]
fn a_cell_fills_its_column_and_its_text_stays_where_the_cells_stands() {
    fn table(editing: bool) -> Element<'static, Msg> {
        let pads = cell_pads(1, 2);
        let name: Element<'static, Msg> = if editing {
            Cell::new(
                bare_field::<Msg>("", "Ada")
                    .on_input(|_| Msg::Fit(0))
                    .padding(
                        metrics::padding(row_height(), 0.0)
                            .left(pads.0)
                            .right(pads.1),
                    ),
            )
            .into()
        } else {
            text("Ada").into()
        };
        Table::new([Column::new("Sıra").width(40.0), Column::new("Ad").width(120.0)])
            .grid()
            .push(Row::new([text("1").into(), name]))
            .into()
    }
    let mut snapshot = Snapshot::new(Size::new(320.0, 80.0)).expect("çizici kurulamadı");
    let theme = Theme::Dark;
    let t = Tokens::of(&theme);
    let row = metrics::row() as u32;
    let y = row + 1 + row / 2;
    let first = cell_pads(0, 2);
    let start = (typography::scaled(40.0) + first.0 + first.1) as u32;
    let plain = snapshot.render(table(false), &theme);
    let cell = snapshot.render(table(true), &theme);
    assert!(
        near(pixel(&cell, start + 1, y - 10), t.cell()),
        "a writable cell's ground from the column's start"
    );
    let end = start
        + (typography::scaled(120.0) + cell_pads(1, 2).0 + cell_pads(1, 2).1) as u32;
    assert!(
        near(pixel(&cell, end - 2, y - 10), t.cell()),
        "to the column's end but for the line's pixel"
    );
    let ink = |image: &Image| {
        let ground = pixel(image, start + 2, y - 10);
        (start..end).find(|&x| {
            (y - 6..y + 6).any(|y| {
                let p = pixel(image, x, y);
                p.iter()
                    .zip(ground)
                    .any(|(a, b)| (i32::from(*a) - i32::from(b)).abs() > 40)
            })
        })
    };
    // Anti-aliased on two grounds: their first inked pixels may differ by one.
    let (plain_at, cell_at) = (ink(&plain).expect("its text"), ink(&cell).expect("its text"));
    assert!(
        plain_at.abs_diff(cell_at) <= 1,
        "the text does not move when it turns into a field: {plain_at} and {cell_at}"
    );
}

#[test]
fn a_wide_table_is_as_wide_as_its_columns_and_their_pads() {
    let table = Table::<'_, Msg>::new([
        Column::new("A").width(100.0),
        Column::new("B").width(50.0),
    ])
    .horizontal();
    assert_eq!(
        table.content_width(),
        typography::scaled(100.0)
            + typography::scaled(50.0)
            + super::PADDING_X * 2.0
            + super::PAD * 2.0
    );
    let table = table.min_width(4000.0);
    assert_eq!(table.content_width(), 4000.0, "up to the panel's edge");
}
