//! The pyramid file (docs/adr/0204 §3): a raster's levels from
//! `PYRAMID_FIRST` on, made in one pass over level 0 with rows pushed in
//! uneven bands, are sample for sample the levels the reader works out
//! itself; an unsigned 16-bit raster with nodata blocks and a float DEM with
//! NaN holes, both with odd edges.

use kentos_contracts::RasterSample;
use kentos_formats::raster::pyramid::Builder;
use kentos_formats::raster::source::{PYRAMID_FIRST, Put, Reader};
use kentos_formats::raster::{ByteStore, Samples, Step, tiff};

/// A raster's samples: a DEM-like float surface with holes, or 16-bit counts with nodata blocks.
fn pattern(w: u32, h: u32, kind: RasterSample) -> Samples {
    let mut s = Samples::filled(kind, w as usize * h as usize, 0.0);
    for j in 0..h {
        for i in 0..w {
            let k = (j * w + i) as usize;
            let v = match kind {
                RasterSample::F32 if (i / 40 + j / 30) % 11 == 0 => f64::NAN,
                RasterSample::F32 => {
                    1000.0
                        + f64::from(i) * 0.37
                        + f64::from(j) * 0.11
                        + f64::from((i * j) % 113) * 0.013
                }
                _ if (i / 50 + j / 70) % 9 == 0 => 65535.0,
                _ => f64::from((i * 37 + j * 101 + (i * j) % 997) % 60000),
            };
            s.set(k, v);
        }
    }
    s
}

/// The pyramid file of `reader`'s level 0, `rows` rows a push, as the host writes it.
fn pyramid_of(reader: &mut Reader, rows: u32) -> Vec<u8> {
    let info = &reader.info;
    let (w, h) = (info.width, info.height);
    let (mut b, header) =
        Builder::new(w, h, info.bands, info.sample, reader.nodata, false).expect("a pass");
    let mut file = header;
    let mut y = 0;
    while y < h {
        let n = rows.min(h - y);
        let region = reader.region(0, 0, i64::from(y), w, n).expect("level 0");
        file.extend(b.push(&region).expect("pushed"));
        y += n;
    }
    assert_eq!(b.rows_done(), h);
    let (dirs, head) = b.finish().expect("finished");
    file.extend(dirs);
    file[..head.len()].copy_from_slice(&head);
    file
}

#[test]
fn a_pyramid_file_holds_the_levels_the_reader_works_out() {
    for (kind, nodata) in [
        (RasterSample::U16, Some(65535.0)),
        (RasterSample::F32, None),
    ] {
        let (w, h) = (2101, 1099);
        let samples = pattern(w, h, kind);
        let mut plain =
            Reader::image(w, h, 1, samples.clone(), nodata, None, 1 << 30).expect("a reader");
        let file = pyramid_of(&mut plain, 173);
        let mut store = ByteStore::new();
        store.put(0, file.clone());
        let Step::Done(t) =
            tiff::parse(&store, file.len() as u64).expect("the pyramid file parses")
        else {
            panic!("the whole file was given");
        };
        assert_eq!(
            t.ifds.len(),
            plain.levels.len() - PYRAMID_FIRST,
            "{kind:?}: a directory a level"
        );
        let mut with = Reader::image(w, h, 1, samples, nodata, None, 1 << 30).expect("a reader");
        with.attach_pyramid(&t).expect("attached");
        for level in PYRAMID_FIRST..plain.levels.len() {
            let (lw, lh) = (plain.levels[level].width, plain.levels[level].height);
            for need in with.needs(level, 0, 0, lw, lh) {
                assert_eq!(
                    need.file, 1,
                    "{kind:?}: level {level} comes from the pyramid file"
                );
                let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
                assert_eq!(
                    with.put_block(&need, &file[a..b]).expect("a block"),
                    Put::Done
                );
            }
            let worked = plain.region(level, 0, 0, lw, lh).expect("worked out");
            let written = with.region(level, 0, 0, lw, lh).expect("from the file");
            assert!(
                worked.samples.to_bytes(true) == written.samples.to_bytes(true),
                "{kind:?}: level {level} ({lw} × {lh}) differs"
            );
        }
    }
}

#[test]
fn rows_out_of_order_are_refused() {
    let (w, h) = (1100, 600);
    let mut reader = Reader::image(
        w,
        h,
        1,
        pattern(w, h, RasterSample::U16),
        Some(65535.0),
        None,
        1 << 30,
    )
    .expect("a reader");
    let (mut b, _) =
        Builder::new(w, h, 1, RasterSample::U16, Some(65535.0), false).expect("a pass");
    let skipped = reader.region(0, 0, 10, w, 10).expect("level 0");
    assert!(b.push(&skipped).is_err());
    let narrow = reader.region(0, 0, 0, w - 1, 10).expect("level 0");
    assert!(b.push(&narrow).is_err());
    assert!(b.finish().is_err(), "an unfinished pass gives no file");
    assert!(
        Builder::new(900, 500, 1, RasterSample::U8, None, false).is_err(),
        "too small for a pyramid"
    );
}
