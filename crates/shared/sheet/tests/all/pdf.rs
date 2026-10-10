//! PDF and GeoPDF (design §9a): a system template's sheet becomes a page of
//! its paper's exact size with its faces embedded as subsets, its pictures
//! and its map's vectors in layers, a geographic viewport per map frame,
//! and the same bytes every time. The golden digest in
//! `fixtures/sheet/v1/pdf/ifraz.json` holds the file (`KENTOS_WRITE_SHEET=1`
//! rewrites it); `smoke.mjs` holds the WASM package to the same digest.
//! `pdf_files` (ignored) writes the files the acceptance tools read into
//! `.run/shots/sheet-pdf`.

use crate::common;

use common::*;
use kentos_sheet::display::{Prim, RenderInputs, display_list};
use kentos_sheet::pdf::*;
use kentos_sheet::template::*;
use kentos_sheet::*;
use sha2::{Digest, Sha256};

fn ifraz() -> SheetBook {
    let t = system_templates()
        .iter()
        .find(|t| t.meta.id == "sys:ifraz-paftasi")
        .expect("the ifraz template");
    let ids = InstanceIds {
        sheet: "s1".into(),
        master: t.master.as_ref().map(|_| "m1".into()),
        items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
        master_items: t
            .master
            .iter()
            .flat_map(|m| m.items.iter().map(|i| format!("m-{}", i.id)))
            .collect(),
    };
    let options = InstanceOptions {
        paper: None,
        name: None,
        center: Some(CENTER),
        scale: None,
        values: sample_values(),
    };
    let inst = instantiate(t, &ids, &options).expect("instantiate");
    SheetBook {
        sheets: vec![inst.sheet],
        masters: inst.master.into_iter().collect(),
        assets: inst.assets,
        ..SheetBook::default()
    }
}

fn tm33_crs() -> PdfCrs {
    PdfCrs {
        wkt: tm_wkt(&TmCrs {
            name: "TUREF / TM33".into(),
            datum: "TUREF".into(),
            ellipsoid: "GRS80".into(),
            epsg: Some(5255),
            tm: tm33(),
        }),
        epsg: Some(5255),
    }
}

/// What a host gives: the screen's inputs, the faces, the maps' vectors, the system.
fn inputs_for(book: &SheetBook) -> PdfInputs {
    let render: RenderInputs = sample_inputs(book, "s1", true);
    let list = display_list(book, "s1", &render).expect("the display list");
    let maps = list
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Map(m) => map_vector(m).map(|content| PdfMap {
                item: m.item.clone(),
                content,
                corners: None,
            }),
            _ => None,
        })
        .collect();
    PdfInputs {
        render,
        fonts: pdf_fonts(),
        assets: Vec::new(),
        maps,
        crs: Some(tm33_crs()),
    }
}

fn text(pdf: &[u8]) -> String {
    String::from_utf8_lossy(pdf).into_owned()
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn the_ifraz_sheet_is_a_page_of_its_paper_with_its_faces_layers_and_place() {
    let book = ifraz();
    let inputs = inputs_for(&book);
    let pdf = to_pdf(&book, &inputs, &PdfOptions::default()).expect("a PDF");
    let t = text(&pdf);
    assert!(t.starts_with("%PDF-1.7"), "{}", &t[..20]);
    assert!(t.trim_end().ends_with("%%EOF"));
    // A3 landscape: 420 × 297 mm, 1 mm = 72/25.4 pt.
    assert!(
        t.contains("/MediaBox [0 0 1190.5511 841.8898]"),
        "the page's box"
    );
    assert_eq!(
        t.matches("/Type /Page\n").count() + t.matches("/Type /Page ").count(),
        1
    );
    // Faces as CID subsets with their Unicode.
    assert!(t.contains("/Subtype /CIDFontType2"));
    assert!(t.contains("/FontFile2"));
    assert!(t.contains("/ToUnicode"));
    assert!(t.contains("+Barlow-"), "a tagged subset name");
    // The map's layers and its place.
    assert!(t.contains("/OCProperties"));
    assert!(t.contains("(Parseller)"));
    assert!(t.contains("/Subtype /GEO"));
    assert!(t.contains("/EPSG 5255"));
    assert!(!t.contains("KENTOSGPTS"), "every coordinate set in");
    assert!(
        t.contains("/GPTS [39.8933201015 32.8482460680 "),
        "latitude first, ten decimals"
    );
    assert!(t.contains("/Producer (KentOS)"));
    // The same inputs, the same bytes.
    assert_eq!(to_pdf(&book, &inputs, &PdfOptions::default()).unwrap(), pdf);
}

/// The faces the sheet and its map write with: what `pdfFonts` tells the host to load.
#[test]
fn the_fonts_needed_are_the_sheet_s_and_the_map_s() {
    let book = ifraz();
    let mut inputs = inputs_for(&book);
    inputs.fonts.clear();
    let faces = fonts_needed(&book, &inputs, &PdfOptions::default()).expect("faces");
    assert!(
        faces
            .iter()
            .any(|f| f.font == "barlow" && f.weight == 500 && !f.italic)
    );
    assert!(
        faces.iter().any(|f| f.font == "barlow" && f.italic),
        "the blocks' names"
    );
    let e = to_pdf(&book, &inputs, &PdfOptions::default()).unwrap_err();
    assert_eq!(e.code, "pdf_font_missing");
    assert!(e.message.contains("barlow"), "{}", e.message);
}

#[test]
fn the_options_choose_sheets_layers_and_the_place() {
    let book = ifraz();
    let inputs = inputs_for(&book);
    let plain = PdfOptions {
        sheets: vec!["s1".into()],
        geo: false,
        layers: false,
    };
    let t = text(&to_pdf(&book, &inputs, &plain).unwrap());
    assert!(!t.contains("/OCProperties") && !t.contains("/OC /oc"));
    assert!(!t.contains("/VP"));
    let e = to_pdf(
        &book,
        &inputs,
        &PdfOptions {
            sheets: vec!["yok".into()],
            ..PdfOptions::default()
        },
    )
    .unwrap_err();
    assert_eq!(
        (e.code.as_str(), e.path.as_deref()),
        ("pdf_unknown_sheet", Some("yok"))
    );
    let e = to_pdf(&SheetBook::default(), &inputs, &PdfOptions::default()).unwrap_err();
    assert_eq!(e.code, "pdf_no_sheets");
}

/// A map's picture that is not a PNG is refused; a map without content is the “harita” box.
#[test]
fn a_map_s_picture_must_be_a_png_and_a_map_without_content_is_a_box() {
    let book = ifraz();
    let mut inputs = inputs_for(&book);
    let item = inputs.maps[0].item.clone();
    inputs.maps[0].content = MapContent::Raster(RasterMap {
        png: b"not a png".to_vec(),
    });
    let e = to_pdf(&book, &inputs, &PdfOptions::default()).unwrap_err();
    assert_eq!(
        (e.code.as_str(), e.path.as_deref()),
        ("pdf_map_bad", Some(item.as_str()))
    );
    inputs.maps.clear();
    let pdf = to_pdf(&book, &inputs, &PdfOptions::default()).expect("a PDF");
    assert!(!text(&pdf).contains("/OCProperties"));
}

/// A logo with transparency goes in with its soft mask, a JPEG as it is; a picture whose bytes
/// the host did not give is the light box.
#[test]
fn pictures_go_in_as_images_with_their_masks() {
    let mut book = ifraz();
    let mut png = Vec::new();
    {
        let mut e = png::Encoder::new(&mut png, 8, 8);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        let px: Vec<u8> = (0..64u32)
            .flat_map(|i| [(i * 4) as u8, 90, 200, if i % 2 == 0 { 255 } else { 40 }])
            .collect();
        w.write_image_data(&px).unwrap();
    }
    let mut jpeg = vec![0xff, 0xd8, 0xff, 0xc0, 0, 17, 8, 0, 12, 0, 16, 3];
    jpeg.extend_from_slice(&[1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]);
    jpeg.extend_from_slice(&[0xff, 0xda, 0, 2, 0xff, 0xd9]);
    let mut inputs = inputs_for(&book);
    for (n, (bytes, kind)) in [
        (&png, AssetKind::Png),
        (&jpeg, AssetKind::Jpeg),
        (&png, AssetKind::Png),
    ]
    .into_iter()
    .enumerate()
    {
        // The third names bytes nobody gave: a missing picture.
        let sha = if n == 2 { "f".repeat(64) } else { sha(bytes) };
        book.assets.push(AssetMeta {
            sha256: sha.clone(),
            kind,
            name: format!("resim{n}"),
            width: 8,
            height: 8,
            bytes: bytes.len() as u32,
            dpi: None,
        });
        if n < 2 {
            inputs.assets.push(PdfAsset {
                sha256: sha.clone(),
                data: bytes.clone(),
                raster: None,
            });
        }
        book.sheets[0].items.push(Item::new(
            &format!("resim{n}"),
            "Logo",
            RectUm::new(310_000 + n as i32 * 25_000, 230_000, 20_000, 20_000),
            ItemKind::Picture(PictureItem {
                asset: Some(sha),
                fit: PictureFit::Contain,
                clip: true,
            }),
        ));
    }
    let pdf = to_pdf(&book, &inputs, &PdfOptions::default()).expect("a PDF");
    let t = text(&pdf);
    assert_eq!(
        t.matches("/Subtype /Image").count(),
        3,
        "a picture, its mask, a JPEG"
    );
    assert!(t.contains("/SMask"));
    assert!(t.contains("/Filter /DCTDecode"));
    assert!(
        t.contains("/Im1 Do") || t.contains("/Im1 "),
        "drawn on the page"
    );
}

/// An SVG picture (design §9a): the core draws no SVG, so the host draws it as a PNG at the size
/// `svg_sizes` gives and passes it as the asset's `raster`; the PDF embeds that in its place and
/// `findings` says so. Without one it is the missing picture's box and `findings` warns.
#[test]
fn an_svg_picture_goes_in_as_the_png_the_host_drew() {
    use kentos_sheet::preflight::Severity;
    let mut book = ifraz();
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10" fill="#c00000"/></svg>"##.to_vec();
    let sha_svg = sha(&svg);
    book.assets.push(AssetMeta {
        sha256: sha_svg.clone(),
        kind: AssetKind::Svg,
        name: "logo.svg".into(),
        width: 24,
        height: 24,
        bytes: svg.len() as u32,
        dpi: None,
    });
    book.sheets[0].items.push(Item::new(
        "logo",
        "Kurum logosu",
        RectUm::new(310_000, 230_000, 20_000, 20_000),
        ItemKind::Picture(PictureItem {
            asset: Some(sha_svg.clone()),
            fit: PictureFit::Contain,
            clip: true,
        }),
    ));
    let mut inputs = inputs_for(&book);
    inputs.assets.push(PdfAsset {
        sha256: sha_svg.clone(),
        data: svg.clone(),
        raster: None,
    });
    let options = PdfOptions::default();
    // 20 mm at 150 dpi: 118.1 pixels, drawn at 119.
    assert_eq!(
        svg_sizes(&book, &inputs, &options, 150).unwrap(),
        [PdfSvgSize {
            sha256: sha_svg.clone(),
            width: 119,
            height: 119
        }]
    );
    // No PNG of it: the light box, and a warning (never a silent empty box).
    let pdf = to_pdf(&book, &inputs, &options).expect("a PDF");
    assert_eq!(text(&pdf).matches("/Subtype /Image").count(), 0);
    let warned = findings(&book, &inputs, &options).unwrap();
    assert_eq!(
        warned
            .iter()
            .map(|f| (f.severity, f.code.as_str(), f.item.as_deref()))
            .collect::<Vec<_>>(),
        [(Severity::Warning, "svg_not_in_pdf", Some("logo"))]
    );
    assert_eq!(
        warned[0].message,
        "“Kurum logosu” (Resim): SVG resim PDF'e çizilemedi; yerine boş resim kutusu basıldı."
    );
    // The host's PNG of it: embedded as a PNG is, and said.
    let mut png = Vec::new();
    {
        let mut e = png::Encoder::new(&mut png, 119, 119);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        w.write_image_data(&vec![200; 119 * 119 * 4]).unwrap();
    }
    inputs.assets.last_mut().unwrap().raster = Some(png);
    let pdf = to_pdf(&book, &inputs, &options).expect("a PDF");
    let t = text(&pdf);
    assert_eq!(
        t.matches("/Subtype /Image").count(),
        2,
        "the picture and its mask"
    );
    assert!(t.contains("/Width 119"));
    let said = findings(&book, &inputs, &options).unwrap();
    assert_eq!(
        (said.len(), said[0].severity, said[0].code.as_str()),
        (1, Severity::Info, "svg_as_picture")
    );
    assert_eq!(
        said[0].message,
        "“Kurum logosu” (Resim): SVG resim PDF'e resim olarak gömülür (119 × 119 piksel)."
    );
    // The asset in JSON: its raster as base64, absent when there is none.
    let json = serde_json::to_value(inputs.assets.last().unwrap()).unwrap();
    assert!(
        json["raster"]
            .as_str()
            .is_some_and(|r| r.starts_with("iVBOR"))
    );
    let back: PdfAsset = serde_json::from_value(json).unwrap();
    assert_eq!(
        back.raster.as_ref().map(Vec::len),
        inputs.assets.last().unwrap().raster.as_ref().map(Vec::len)
    );
    let plain = PdfAsset {
        sha256: sha_svg,
        data: svg,
        raster: None,
    };
    assert!(
        serde_json::to_value(&plain)
            .unwrap()
            .get("raster")
            .is_none()
    );
}

/// The golden file's digest: the same on every target (smoke.mjs reads the same record).
#[test]
fn the_ifraz_pdf_is_the_golden_one() {
    let book = ifraz();
    let inputs = inputs_for(&book);
    let pdf = to_pdf(&book, &inputs, &PdfOptions::default()).unwrap();
    let dir = fixtures().join("pdf");
    let record = serde_json::json!({
        "sha256": sha(&pdf),
        "size": pdf.len(),
    });
    let file = dir.join("ifraz.json");
    if writing() {
        std::fs::create_dir_all(&dir).unwrap();
        // What the WASM package is given: the book and the inputs without the faces' bytes.
        let mut light = inputs.clone();
        light.fonts.iter_mut().for_each(|f| f.data.clear());
        std::fs::write(
            dir.join("ifraz-book.json"),
            serde_json::to_string(&book).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("ifraz-inputs.json"),
            serde_json::to_string(&light).unwrap(),
        )
        .unwrap();
        std::fs::write(&file, serde_json::to_string_pretty(&record).unwrap() + "\n").unwrap();
    }
    let golden: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("the golden record")).unwrap();
    assert_eq!(
        golden, record,
        "the PDF changed: read the difference, then KENTOS_WRITE_SHEET=1"
    );
}

#[test]
#[ignore = "files for the acceptance tools, run by hand"]
fn pdf_files() {
    let out = root().join(".run/shots/sheet-pdf");
    std::fs::create_dir_all(&out).unwrap();
    let book = ifraz();
    let inputs = inputs_for(&book);
    let pdf = to_pdf(&book, &inputs, &PdfOptions::default()).unwrap();
    let file = out.join("cekirdek-ifraz.pdf");
    std::fs::write(&file, &pdf).unwrap();
    // The map frames as the sheet places them, for the corner check against `gdalinfo`.
    let list = display_list(&book, "s1", &inputs.render).unwrap();
    let maps: Vec<&kentos_sheet::display::MapPrim> = list
        .prims
        .iter()
        .filter_map(|p| match p {
            Prim::Map(m) => Some(m),
            _ => None,
        })
        .collect();
    std::fs::write(
        out.join("cekirdek-ifraz-haritalar.json"),
        serde_json::to_string_pretty(&maps).unwrap(),
    )
    .unwrap();
    println!("{}", file.display());
    // The missing-glyph sheet (fixtures `glyphs/sheet.json`): a borrowed arrow, a “?”, a lira sign.
    let fx: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("glyphs/sheet.json")).unwrap(),
    )
    .unwrap();
    let book = kentos_sheet::validate::read_book(&fx["book"].to_string()).unwrap();
    let inputs = PdfInputs {
        render: serde_json::from_value(fx["inputs"].clone()).unwrap(),
        fonts: pdf_fonts(),
        assets: Vec::new(),
        maps: Vec::new(),
        crs: None,
    };
    let file = out.join("eksik-karakter.pdf");
    std::fs::write(
        &file,
        to_pdf(&book, &inputs, &PdfOptions::default()).unwrap(),
    )
    .unwrap();
    println!("{}", file.display());
}

/// A sheet's export file name (`display::export_name`, WASM `exportName`): the default template
/// writes the sheet's name, a template reads the sheet's values, a missing one is its mark, a
/// fixed name is kept, an empty one is the sheet's name.
#[test]
fn a_sheet_s_export_name_is_written_in_its_scope() {
    use kentos_sheet::display::export_name;
    let mut book = ifraz();
    let inputs = sample_inputs(&book, "s1", true);
    let name = book.sheets[0].name.clone();
    assert_eq!(export_name(&book, "s1", &inputs).unwrap(), name);
    book.sheets[0].export.file_name = "Ada [% @ada %]  [% @yok %]".into();
    assert_eq!(
        export_name(&book, "s1", &inputs).unwrap(),
        "Ada 1234 ‹yok?›"
    );
    book.sheets[0].export.file_name = "İfraz 12".into();
    assert_eq!(export_name(&book, "s1", &inputs).unwrap(), "İfraz 12");
    book.sheets[0].export.file_name = "  ".into();
    assert_eq!(export_name(&book, "s1", &inputs).unwrap(), name);
    assert_eq!(
        export_name(&book, "yok", &inputs).unwrap_err().code,
        "unknown_sheet"
    );
}

/// The drawing's own variables (Proje ayarları › Değişkenler, docs/adr/0214 §2.3) are read
/// after the book's: a name the book has keeps the book's value, one only the drawing has
/// is the drawing's, looked for with Turkish letters and case aside.
#[test]
fn the_drawing_s_variables_are_read_after_the_book_s() {
    use kentos_sheet::display::export_name;
    use kentos_sheet::model::{VarKind, VarValue, Variable};
    let mut book = ifraz();
    let mut inputs = sample_inputs(&book, "s1", true);
    let var = |name: &str, value: &str| Variable {
        name: name.into(),
        label: String::new(),
        kind: VarKind::Text,
        value: VarValue::Text(value.into()),
    };
    book.sheets[0].export.file_name = "[% @İs_No %] Ada [% @ada %]".into();
    assert_eq!(
        export_name(&book, "s1", &inputs).unwrap(),
        "‹İs_No?› Ada 1234"
    );
    inputs.project.variables = vec![var("is_no", "2026/41"), var("ada", "9999")];
    assert_eq!(
        export_name(&book, "s1", &inputs).unwrap(),
        "2026/41 Ada 1234"
    );
}
