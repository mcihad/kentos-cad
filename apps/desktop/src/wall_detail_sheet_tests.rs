//! The walls on paper at each map's own detail (docs/adr/0696's 29 October addendum): an L of KentOS's Dış duvar 25
//! (plaster, brick hatched ANSI31, insulation hatched ANSI37, plaster; filled #5A5A5A) on four sheets whose maps stand
//! at 1:20, 1:50, 1:100 and 1:200, printed through the sheet mode's Dışa aktar › PDF and through the headless book
//! (`kentos_plot::book`), each page drawn back by poppler's `pdftoppm` across the wall's middle. Ink that fills a row
//! of the band from end to end is a line along the wall (a face or a component's side) or, thick, the schematic body:
//! detailed shows five lines, standard three, schematic one band as thick as the wall. The model's camera close in (the
//! store drawing detailed) leaves every page as it was; a project's own detail is every map's. `KENTOS_PLOT_OUT=dir`
//! keeps the PDFs and the crops there (`duvar-ayrinti-*.pdf`, `duvar-ayrinti-*-N.png`).

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};
use crate::viewport::Event;

/// The maps' scales, one sheet each, in the book's order.
const SCALES: [f64; 4] = [20.0, 50.0, 100.0, 200.0];
/// Pixels per millimetre of the crops (600 dpi).
const DPI: f64 = 600.0;
/// The map's centre on the page: its frame's middle (A3 landscape, 20 mm left and 10 mm top margins), mm.
const CENTRE: [f64; 2] = [20.0 + 390.0 / 2.0, 10.0 + 277.0 / 2.0];

fn out_dir() -> std::path::PathBuf {
    match std::env::var_os("KENTOS_PLOT_OUT").map(std::path::PathBuf::from) {
        Some(dir) => {
            std::fs::create_dir_all(&dir).expect("the output directory");
            dir
        }
        None => scratch("duvar-ayrinti-pafta"),
    }
}

fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    app.follow_sheets();
    answer
}

/// A sheet whose one map stands at 1:`scale` on the horizontal wall's middle, (3, 0).
fn sheet(scale: f64) -> Value {
    let id = format!("pafta-{scale}");
    json!({ "op": "addSheet", "sheet": {
        "id": id, "name": format!("Duvar {scale}"),
        "page": { "paper": "a3", "orientation": "landscape", "size": { "width": 420000, "height": 297000 },
            "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } },
        "items": [{ "id": format!("{id}-harita"), "name": "Harita",
            "frame": { "left": 20000, "top": 10000, "width": 390000, "height": 277000 },
            "kind": { "type": "map", "view": { "type": "fixed", "center": { "x": 3.0, "y": 0.0 },
                "scale": scale as u32, "rotation": 0 } } }]
    }})
}

/// A CAD project at 1:100 whose walls' detail follows the scale: the L of Dış duvar 25 and the four sheets, open in the
/// desktop with its area 1100 × 700 px.
fn walls_on_sheets(dir: &std::path::Path) -> App {
    let project = json!({ "name": "Duvar", "workspace": "cad", "coords": "local", "drawingUnit": "m", "plotScale": 100 });
    let choices = kentos_headless::new_drawing(&project).expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    let path = dir.join("duvar.kcad");
    std::fs::write(&path, made.to_bytes().expect("its bytes")).expect("written");
    let (mut app, _) = App::boot(None);
    let doc = Document::read(&path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    let layer = app
        .document
        .as_ref()
        .map(|d| d.model.layers().active().to_owned())
        .expect("a layer");
    command(
        &mut app,
        "cad.walls.create",
        json!({ "layerId": layer, "walls": [{ "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 6.0, "y": 0.0 },
            { "x": 6.0, "y": 4.0 }], "justify": "center", "style": "kentos-dis-duvar-25" }] }),
    );
    command(
        &mut app,
        "cad.sheets.edit",
        json!({ "ops": SCALES.iter().map(|&s| sheet(s)).collect::<Vec<_>>() }),
    );
    let _ = app.update(Message::Viewport(Event::Resized(iced::Rectangle::new(
        iced::Point::ORIGIN,
        iced::Size::new(1100.0, 700.0),
    ))));
    app
}

fn print(app: &mut App, path: &std::path::Path) {
    use kentos_sheet_ui::message::{PdfMessage, PdfScope};
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Export(
        kentos_sheet_ui::ExportKind::Pdf,
    )));
    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Pdf(
        PdfMessage::Scope(PdfScope::All),
    )));
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(path.to_path_buf()));
    assert!(path.exists(), "the PDF written: {}", path.display());
}

/// A grey PGM's width, height and pixels.
fn pgm(bytes: &[u8]) -> (usize, usize, &[u8]) {
    // "P5", width, height, maximum, each after white space; the pixels after the last one's single white space.
    let mut fields = Vec::new();
    let mut i = 0;
    while fields.len() < 4 {
        while bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        fields.push(String::from_utf8_lossy(&bytes[start..i]).into_owned());
    }
    let w: usize = fields[1].parse().expect("a width");
    let h: usize = fields[2].parse().expect("a height");
    (w, h, &bytes[i + 1..i + 1 + w * h])
}

/// The runs of rows a page draws across the wall, top to bottom, in rows: `lines` those ink fills from end to end (a
/// face, a component's side), `ink` those any ink covers from end to end (the schematic body's translucent fill too).
#[derive(Debug)]
struct Across {
    lines: Vec<usize>,
    ink: Vec<usize>,
}

/// The lengths of the runs of rows whose pixels darker than `below` cover nine tenths of the row.
fn runs(pixels: &[u8], width: usize, below: u8) -> Vec<usize> {
    let mut out = Vec::new();
    let mut run = 0;
    for row in pixels.chunks(width) {
        if row.iter().filter(|&&v| v < below).count() * 10 >= width * 9 {
            run += 1;
        } else if run > 0 {
            out.push(run);
            run = 0;
        }
    }
    if run > 0 {
        out.push(run);
    }
    out
}

/// What page `page` draws across the wall at 1:`scale` in a 4 mm wide band across the wall's middle, three walls'
/// thickness high. The crop is kept as a PNG beside the PDF.
fn across(pdf: &std::path::Path, page: usize, scale: f64) -> Across {
    let px = DPI / 25.4;
    let wall = 0.25 * 1000.0 / scale * px;
    let (w, h) = ((4.0 * px).round() as usize, (wall * 3.0).round() as usize);
    let x = (CENTRE[0] * px - w as f64 / 2.0).round() as usize;
    let y = (CENTRE[1] * px - h as f64 / 2.0).round() as usize;
    let stem = pdf.with_extension("");
    let prefix = format!("{}-{page}", stem.display());
    let crop = |format: &str| {
        let ran = std::process::Command::new("pdftoppm")
            .args([
                "-r",
                &DPI.to_string(),
                format,
                "-singlefile",
                "-f",
                &page.to_string(),
                "-l",
                &page.to_string(),
            ])
            .args([
                "-x",
                &x.to_string(),
                "-y",
                &y.to_string(),
                "-W",
                &w.to_string(),
                "-H",
                &h.to_string(),
            ])
            .arg(pdf)
            .arg(&prefix)
            .status()
            .expect("pdftoppm runs");
        assert!(ran.success(), "pdftoppm {format} {}", pdf.display());
    };
    crop("-png");
    crop("-gray");
    let bytes = std::fs::read(format!("{prefix}.pgm")).expect("the crop");
    let (cw, ch, pixels) = pgm(&bytes);
    assert_eq!((cw, ch), (w, h), "the crop's size");
    // Ink is black on paper; the body's fill #5A5A5A at 45 % reads 180 (docs/adr/0339 §4).
    Across {
        lines: runs(pixels, cw, 160),
        ink: runs(pixels, cw, 230),
    }
}

/// The detail a page shows across the wall: schematic its two faces round one band of ink at least nine tenths of
/// the wall's thickness; standard three lines (faces, brick and insulation), detailed five (the plasters' too), with no
/// ink across between them; anything else is said as it is.
fn detail_of(a: &Across, scale: f64) -> &'static str {
    let wall = 0.25 * 1000.0 / scale * DPI / 25.4;
    // Lines of half a millimetre at most, and ink only on them (a row of a line's soft edge at most).
    let bare = a.lines.iter().all(|&r| (r as f64) <= 0.5 * DPI / 25.4)
        && a.ink.len() == a.lines.len()
        && a.ink.iter().zip(&a.lines).all(|(&i, &l)| i <= l + 2);
    match (a.lines.len(), a.ink.as_slice()) {
        (2, [band]) if *band as f64 >= wall * 0.9 => "schematic",
        (3, _) if bare => "standard",
        (5, _) if bare => "detailed",
        _ => "other",
    }
}

/// Each page's detail in the PDF at `path`, with the runs it was read from.
fn details(path: &std::path::Path) -> Vec<(&'static str, Across)> {
    SCALES
        .iter()
        .enumerate()
        .map(|(i, &scale)| {
            let a = across(path, i + 1, scale);
            (detail_of(&a, scale), a)
        })
        .collect()
}

fn assert_details(
    path: &std::path::Path,
    want: [&str; 4],
    what: &str,
) -> Vec<(&'static str, Across)> {
    let got = details(path);
    let names: Vec<&str> = got.iter().map(|(d, _)| *d).collect();
    assert_eq!(names, want, "{what}: {} → {got:?}", path.display());
    got
}

fn set_detail(app: &mut App, detail: Option<kentos_commands::WallDetail>) {
    let model = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = model.settings().clone();
    let mut arch = settings.arch.clone().unwrap_or_default();
    arch.detail = detail;
    settings.arch = Some(arch);
    model.set_settings(settings);
    app.follow_sheets();
}

fn headless(app: &App, path: &std::path::Path) {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let pdf = kentos_plot::book::book_pdf(doc, &[], "Duvar", "2026-10-29").expect("the book's PDF");
    std::fs::write(path, pdf.bytes).expect("written");
}

#[cfg(unix)]
#[test]
fn each_maps_walls_are_drawn_at_its_own_scales_detail_on_paper() {
    if std::process::Command::new("pdftoppm")
        .arg("-v")
        .output()
        .is_err()
    {
        eprintln!("pdftoppm yok: duvar ayrıntısının paftası denetlenmedi (poppler kurun).");
        return;
    }
    let dir = out_dir();
    let mut app = walls_on_sheets(&dir);
    let auto = ["detailed", "standard", "schematic", "schematic"];

    // The model far out: each map at its own scale's detail, on the desktop and in the headless book alike.
    app.viewport.camera.zoom_to_screen_scale(500.0);
    app.follow_wall_view();
    assert_eq!(
        app.spatial.store().wall_view().detail,
        kentos_geometry_core::geom::wall::Detail::Schematic
    );
    let far = dir.join("duvar-ayrinti-uzak.pdf");
    print(&mut app, &far);
    assert_details(&far, auto, "model 1:500");
    let book = dir.join("duvar-ayrinti-basliksiz.pdf");
    headless(&app, &book);
    assert_details(&book, auto, "headless book");

    // The model close in draws the walls detailed; the paper does not follow, the 1:100 map (the project's own scale)
    // included.
    app.viewport.camera.zoom_to_screen_scale(15.0);
    app.follow_wall_view();
    assert_eq!(
        app.spatial.store().wall_view().detail,
        kentos_geometry_core::geom::wall::Detail::Detailed
    );
    let near = dir.join("duvar-ayrinti-yakin.pdf");
    print(&mut app, &near);
    assert_details(&near, auto, "model 1:15");

    // A project's own detail is every map's, on both paths.
    set_detail(&mut app, Some(kentos_commands::WallDetail::Standard));
    let standard = dir.join("duvar-ayrinti-uygulama.pdf");
    print(&mut app, &standard);
    let three = assert_details(&standard, ["standard"; 4], "Uygulama");
    let book = dir.join("duvar-ayrinti-basliksiz-uygulama.pdf");
    headless(&app, &book);
    assert_details(&book, ["standard"; 4], "headless book, Uygulama");
    set_detail(&mut app, Some(kentos_commands::WallDetail::Detailed));
    let detailed = dir.join("duvar-ayrinti-ayrintili.pdf");
    print(&mut app, &detailed);
    let got = details(&detailed);
    // At 1:20 and 1:50 the five lines print apart. At 1:100 and 1:200 a plaster's side stands within a line's width
    // of its face (2 and 3 cm: 0.2 and 0.3 mm at 1:100) and prints on it: no more runs than Uygulama's three lines but
    // more ink along the wall, still no body.
    assert_eq!(
        [got[0].0, got[1].0],
        ["detailed"; 2],
        "Ayrıntılı: {} → {got:?}",
        detailed.display()
    );
    for page in [2, 3] {
        let ((_, a), (_, s)) = (&got[page], &three[page]);
        let ink = |x: &Across| x.lines.iter().sum::<usize>();
        assert!(
            a.lines.len() <= s.lines.len() && a.ink.len() == a.lines.len() && ink(a) >= ink(s) + 4,
            "Ayrıntılı, page {}: {a:?} against Uygulama's {s:?}",
            page + 1
        );
    }
    set_detail(&mut app, Some(kentos_commands::WallDetail::Schematic));
    let schematic = dir.join("duvar-ayrinti-sematik.pdf");
    print(&mut app, &schematic);
    assert_details(&schematic, ["schematic"; 4], "Şematik");

    // Back to the scale's: as at first.
    set_detail(&mut app, None);
    let again = dir.join("duvar-ayrinti-yeniden.pdf");
    print(&mut app, &again);
    assert_details(&again, auto, "otomatik again");
}
