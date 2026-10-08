//! Reading LAS and LAZ (docs/adr/0207 §2): what laspy and LASzip read from the fixtures, KentOS reads too.

use kentos_pointcloud::record::{Layout, wide_format, widen};

use crate::host::{cases, fixtures, fnv, open, records};

#[test]
fn files_read_as_laspy_and_laszip_read_them() {
    let cases = cases();
    for c in cases["files"].as_array().unwrap() {
        let name = c["file"].as_str().unwrap();
        let bytes = std::fs::read(fixtures().join("files").join(name)).unwrap();
        let cloud = open(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let info = cloud.info();
        assert_eq!(info.kind.name(), c["kind"].as_str().unwrap(), "{name}");
        assert_eq!(info.version, c["version"].as_str().unwrap(), "{name}");
        assert_eq!(
            u64::from(info.format),
            c["format"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            u64::from(info.record_len),
            c["recordLen"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            u64::from(info.extra),
            c["extra"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(info.count, c["count"].as_u64().unwrap(), "{name}");
        for k in 0..3 {
            assert_eq!(info.scale[k], c["scale"][k].as_f64().unwrap(), "{name}");
            assert_eq!(info.offset[k], c["offset"][k].as_f64().unwrap(), "{name}");
        }
        for k in 0..6 {
            assert_eq!(
                info.bounds[k],
                c["bounds"][k].as_f64().unwrap(),
                "{name} bounds {k}"
            );
        }
        assert_eq!(
            info.crs.epsg.map(u64::from),
            c["epsg"].as_u64(),
            "{name}: system"
        );
        let raw = records(&cloud, &bytes);
        assert_eq!(fnv(&raw), c["rawHash"].as_str().unwrap(), "{name}: records");
        let from = cloud.layout;
        let to_format = wide_format(from.format);
        assert_eq!(u64::from(to_format), c["wideFormat"].as_u64().unwrap());
        let to = Layout::new(
            to_format,
            kentos_pointcloud::record::standard_len(to_format) + from.extra(),
        );
        let mut wide = vec![0u8; to.len * cloud.head.count as usize];
        for (i, r) in raw.chunks_exact(from.len).enumerate() {
            widen(&from, r, &to, &mut wide[i * to.len..(i + 1) * to.len]);
        }
        assert_eq!(
            fnv(&wide),
            c["wideHash"].as_str().unwrap(),
            "{name}: widened"
        );
        for (i, f) in c["first"].as_array().unwrap().iter().enumerate() {
            let r = &raw[i * from.len..(i + 1) * from.len];
            assert_eq!(i64::from(from.x(r)), f["x"].as_i64().unwrap());
            assert_eq!(i64::from(from.y(r)), f["y"].as_i64().unwrap());
            assert_eq!(i64::from(from.z(r)), f["z"].as_i64().unwrap());
            assert_eq!(
                u64::from(from.intensity(r)),
                f["intensity"].as_u64().unwrap()
            );
            let (ret, n) = from.returns(r);
            assert_eq!(
                u64::from(ret),
                f["returnNumber"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                u64::from(n),
                f["numberOfReturns"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                u64::from(from.class(r)),
                f["class"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                u64::from(from.user_data(r)),
                f["userData"].as_u64().unwrap(),
                "{name}"
            );
            assert_eq!(
                u64::from(from.source_id(r)),
                f["sourceId"].as_u64().unwrap(),
                "{name}"
            );
            if let Some(g) = f.get("gpsTime") {
                assert_eq!(from.gps_time(r), g.as_f64().unwrap());
            }
            if let Some(c) = f.get("rgb") {
                let rgb = from.rgb(r).unwrap();
                for k in 0..3 {
                    assert_eq!(u64::from(rgb[k]), c[k].as_u64().unwrap());
                }
            }
            if let Some(v) = f.get("nir") {
                assert_eq!(u64::from(from.nir(r).unwrap()), v.as_u64().unwrap());
            }
            if let Some(v) = f.get("scanAngle") {
                assert_eq!(i64::from(from.scan_angle(r)), v.as_i64().unwrap());
            }
        }
    }
}

#[test]
fn a_laz_of_three_chunks_reads_them_apart() {
    let bytes = std::fs::read(fixtures().join("files/many-chunks.laz")).unwrap();
    let cloud = open(&bytes).unwrap();
    assert_eq!(cloud.chunks.len(), 3);
    assert_eq!(
        cloud.chunks.iter().map(|c| c.count).collect::<Vec<_>>(),
        vec![50_000, 50_000, 20_000]
    );
    // The chunks stand alone: read in reverse they give the same records.
    let mut back = Vec::new();
    for run in cloud.runs().iter().rev() {
        let a = run.need.offset as usize;
        let b = (run.need.offset + run.need.len) as usize;
        let mut one = Vec::new();
        cloud.records(run, &bytes[a..b], &mut one).unwrap();
        back.insert(0, one);
    }
    assert_eq!(back.concat(), records(&cloud, &bytes));
}

#[test]
fn broken_files_say_what_is_wrong() {
    let cases = cases();
    for c in cases["broken"].as_array().unwrap() {
        let name = c["file"].as_str().unwrap();
        let bytes = std::fs::read(fixtures().join("files").join(name)).unwrap();
        let err = match open(&bytes) {
            Ok(_) => panic!("{name} opened"),
            Err(e) => e.0,
        };
        let says = c["says"].as_str().unwrap();
        assert!(err.contains(says), "{name}: “{err}” does not say “{says}”");
    }
}

#[test]
fn text_clouds_read_as_the_reference_reads_them() {
    use kentos_pointcloud::text::{Parse, Scan};
    let cases = cases();
    for c in cases["text"].as_array().unwrap() {
        let name = c["file"].as_str().unwrap();
        let bytes = std::fs::read(fixtures().join("files").join(name)).unwrap();
        let mut scan = Scan::new();
        // In pieces, a line cut between them.
        for piece in bytes.chunks(97) {
            scan.feed(piece).unwrap();
        }
        let plan = scan.finish().unwrap();
        assert_eq!(plan.count, c["count"].as_u64().unwrap(), "{name}");
        assert_eq!(
            u64::from(plan.decimals),
            c["decimals"].as_u64().unwrap(),
            "{name}"
        );
        for k in 0..3 {
            assert_eq!(plan.offset[k], c["offset"][k].as_f64().unwrap(), "{name}");
        }
        for k in 0..6 {
            assert_eq!(plan.bounds[k], c["bounds"][k].as_f64().unwrap(), "{name}");
        }
        let mut parse = Parse::new(plan.clone());
        let mut out = Vec::new();
        for piece in bytes.chunks(61) {
            parse.feed(piece, &mut out).unwrap();
        }
        parse.finish(&mut out).unwrap();
        let len = parse.record_len();
        assert_eq!(out.len() / len, plan.count as usize, "{name}");
        let layout = Layout::new(plan.columns.format(), len);
        let r = &out[..len];
        let ints = [layout.x(r), layout.y(r), layout.z(r)];
        for k in 0..3 {
            assert_eq!(
                i64::from(ints[k]),
                c["firstInts"][k].as_i64().unwrap(),
                "{name}"
            );
        }
    }
}
