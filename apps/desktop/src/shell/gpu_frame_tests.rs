//! The overlay frame's tests (gpu_frame.rs): dashes, strokes, fills and texts as the glyph layer takes them.

use super::*;

/// A dash pattern cuts a line into its drawn pieces, the pattern going on round a corner.
#[test]
fn a_dash_pattern_cuts_a_polyline_into_its_pieces() {
    let line = [
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        Point::new(20.0, 10.0),
    ];
    let pieces = dashed(&line, &[9.0, 4.0], 0);
    let lengths: Vec<f32> = pieces
        .iter()
        .map(|p| p.windows(2).map(|w| w[0].distance(w[1])).sum::<f32>())
        .collect();
    // 9 on, 4 off, 9 on (7 along, 2 down), 4 off, the last 4 on.
    assert_eq!(lengths.len(), 3, "{lengths:?}");
    assert!(
        (lengths[0] - 9.0).abs() < 1e-4
            && (lengths[1] - 9.0).abs() < 1e-4
            && (lengths[2] - 4.0).abs() < 1e-4,
        "{lengths:?}"
    );
    assert_eq!(pieces[1].len(), 3, "the second piece turns the corner");
}

/// A stroked and a filled path, and a text, go anchored on the map where the canvas draws them.
#[test]
fn paths_and_texts_go_to_the_glyph_layer() {
    crate::drawing_fonts::load();
    let mut view = Camera::default();
    view.set_size(400.0, 300.0);
    let mut frame = GpuFrame::on_map(view, Size::new(400.0, 300.0));
    frame.stroke(
        &Path::line(Point::new(10.0, 10.0), Point::new(50.0, 10.0)),
        Stroke::default().with_width(2.0),
    );
    frame.fill(&Path::circle(Point::new(100.0, 100.0), 6.0), Color::BLACK);
    let font = crate::drawing_fonts::font(kentos_commands::DrawingFont::Arimo, 400, false);
    frame.fill_text(Text {
        content: "km 0+100".into(),
        position: Point::new(200.0, 150.0),
        font,
        ..Text::default()
    });
    let texts = frame.finish();
    assert_eq!(texts.len(), 3);
    assert_eq!(texts[0].under.len(), 1, "one segment");
    assert!(
        texts[1].under.iter().any(|q| q.triangle),
        "the circle's fan"
    );
    let run = crate::glyph_layer::text_run("km 0+100", font).expect("a drawing face's run");
    assert_eq!(
        texts[2].glyphs.len(),
        run.glyphs.len(),
        "every letter placed"
    );
    assert!(texts[2].glyphs.len() >= 7);
    let at = view.screen_to_map(10.0, 10.0);
    assert_eq!(texts[0].at, [at.x, at.y]);
}

/// A closed square's corners are joined as its stroke asks: a miter fills each corner whole (two triangles), a
/// bevel cuts it (one), a round join fans; a straight run of points needs none.
#[test]
fn a_strokes_corners_are_joined_as_it_asks() {
    let square = Path::rectangle(Point::new(10.0, 10.0), Size::new(40.0, 40.0));
    let quads = |join: LineJoin| {
        let mut frame = GpuFrame::on_screen(Size::new(100.0, 100.0));
        frame.stroke(
            &square,
            Stroke {
                line_join: join,
                ..Stroke::default().with_width(2.5)
            },
        );
        let texts = frame.finish();
        let under = &texts[0].under;
        (
            under.iter().filter(|q| !q.triangle).count(),
            under.iter().filter(|q| q.triangle).count(),
        )
    };
    assert_eq!(quads(LineJoin::Miter), (4, 8));
    assert_eq!(quads(LineJoin::Bevel), (4, 4));
    let (sides, fans) = quads(LineJoin::Round);
    assert_eq!(sides, 4);
    assert!(fans >= 8, "{fans}");
    // A miter's tip is the outer corner of the stroke: 1.25 px out each way from the square's corner.
    let mut frame = GpuFrame::on_screen(Size::new(100.0, 100.0));
    frame.stroke(&square, Stroke::default().with_width(2.5));
    let texts = frame.finish();
    let tips: Vec<[f64; 2]> = texts[0]
        .under
        .iter()
        .filter(|q| q.triangle)
        .map(|q| {
            [
                q.transform[4] + q.transform[0],
                q.transform[5] + q.transform[1],
            ]
        })
        .collect();
    // The anchor is the square's first corner (10, 10); y up from it. The corner opposite is (50, 50) on screen.
    assert!(
        tips.iter()
            .any(|t| (t[0] - 41.25).abs() < 1e-9 && (t[1] + 41.25).abs() < 1e-9),
        "{tips:?}"
    );
    let mut frame = GpuFrame::on_screen(Size::new(100.0, 100.0));
    let straight = Path::new(|p| {
        p.move_to(Point::new(0.0, 0.0));
        p.line_to(Point::new(10.0, 0.0));
        p.line_to(Point::new(20.0, 0.1));
    });
    frame.stroke(&straight, Stroke::default().with_width(2.0));
    assert!(
        frame.finish()[0].under.iter().all(|q| !q.triangle),
        "a turn of a hundredth: no join"
    );
}

/// The screen's marks are lettered in the interface's face: the glyph layer takes it (none of them is lost), and
/// they stay on the screen.
#[test]
fn the_marks_letters_in_the_interface_face_go_as_glyphs() {
    kentos_ui::theme::typography::load();
    let mut frame = GpuFrame::on_screen(Size::new(400.0, 300.0));
    frame.fill_text(Text {
        content: "100 m".into(),
        position: Point::new(300.0, 280.0),
        font: kentos_ui::theme::typography::ui_strong(),
        ..Text::default()
    });
    let texts = frame.finish();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].glyphs.len() >= 4, "{}", texts[0].glyphs.len());
    assert!(
        texts[0]
            .glyphs
            .iter()
            .all(|g| g.key.font.0 >= kentos_render_wgpu::glyphs::EXTRA_FACES),
        "an interface face"
    );
    assert_eq!(texts[0].unit, GlyphUnit::Screen);
    assert_eq!(texts[0].at, [300.0, 280.0]);
}

/// The 0,0 mark among the screen's marks moves with the drawing: anchored on the map, the others on the screen.
#[test]
fn a_mark_of_a_place_goes_on_the_map_among_the_screens() {
    let mut view = Camera::default();
    view.set_size(400.0, 300.0);
    let mut frame = GpuFrame::on_screen(Size::new(400.0, 300.0));
    let dot = Path::circle(Point::new(20.0, 20.0), 3.0);
    frame.fill(&dot, Color::BLACK);
    frame.on_map_of(&view, |f| f.fill(&dot, Color::BLACK));
    frame.fill(&dot, Color::BLACK);
    let units: Vec<GlyphUnit> = frame.finish().iter().map(|t| t.unit).collect();
    assert_eq!(
        units,
        [
            GlyphUnit::Screen,
            GlyphUnit::Px { turns: false },
            GlyphUnit::Screen
        ]
    );
}
