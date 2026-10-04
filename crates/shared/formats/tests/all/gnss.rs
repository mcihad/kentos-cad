//! GNSS files (docs/adr/0169 §1, §6) against fixtures/gnss/v1/gpx.json and
//! nmea.json (scripts/fixtures/gnss_gpx_cases.py and gnss_nmea_cases.py,
//! from GPX 1.1's schema and NMEA 0183's sentences alone): the positions,
//! heights, fixes and times, the points and sentences not read with the
//! same words. The web runs the same files through WASM
//! (`apps/web/src/io/formats.wasm.test.ts`).

use kentos_contracts::GnssPoint;
use kentos_formats::gnss::{gpx, nmea, read};
use serde_json::Value;

/// Every case of a fixture read as the reference reads it.
fn agree(
    file: &str,
    format: &str,
    texts: &[(&str, &str)],
    reader: fn(&[u8]) -> kentos_contracts::GnssRead,
) {
    let file: Value = serde_json::from_str(file).expect("the cases read");
    assert_eq!(file["format"], format);
    for (key, text) in texts {
        assert_eq!(file["texts"][key], *text, "{key}");
    }
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 2);
    for case in cases {
        let name = case["name"].as_str().expect("a name");
        let text = case["text"].as_str().expect("a text").as_bytes();
        for got in [reader(text), read(text)] {
            let want: Vec<GnssPoint> =
                serde_json::from_value(case["expect"]["points"].clone()).expect("points");
            assert_eq!(got.points, want, "{name}");
            let problems: Vec<(u64, String)> = case["expect"]["problems"]
                .as_array()
                .expect("problems")
                .iter()
                .map(|p| {
                    (
                        p["line"].as_u64().expect("a line"),
                        p["problem"].as_str().expect("a text").to_owned(),
                    )
                })
                .collect();
            let got_problems: Vec<(u64, String)> = got
                .problems
                .iter()
                .map(|p| (u64::from(p.line), p.message.clone()))
                .collect();
            assert_eq!(got_problems, problems, "{name}");
            assert_eq!(got.encoding, "UTF-8", "{name}");
        }
    }
}

#[test]
fn a_gpx_file_reads_as_the_reference_does() {
    agree(
        include_str!("../../../../../fixtures/gnss/v1/gpx.json"),
        "kentos.gnss-gpx",
        &[
            ("xml", gpx::XML),
            ("deep", gpx::DEEP),
            ("root", gpx::ROOT),
            ("position", gpx::POSITION),
            ("value", gpx::VALUE),
            ("nofix", gpx::NOFIX),
        ],
        gpx::read,
    );
}

#[test]
fn an_nmea_log_reads_as_the_reference_does() {
    agree(
        include_str!("../../../../../fixtures/gnss/v1/nmea.json"),
        "kentos.gnss-nmea",
        &[
            ("checksum", nmea::CHECKSUM),
            ("position", nmea::POSITION),
            ("value", nmea::VALUE),
            ("unit", nmea::UNIT),
            ("nofix", nmea::NOFIX),
        ],
        nmea::read,
    );
}

/// The one entry tells GPX by its first character, anything else is NMEA.
#[test]
fn the_entry_reads_gpx_by_its_angle_bracket() {
    assert_eq!(read(b"  \n<gpx/>").format, "gpx");
    assert_eq!(read(b"\xef\xbb\xbf<gpx/>").format, "gpx");
    assert_eq!(read(b"$GPGGA,1").format, "nmea");
    assert_eq!(read(b"").format, "nmea");
}

/// Sentences and documents cut short, overlong or not ASCII never panic.
#[test]
fn odd_inputs_are_said_not_panicked_on() {
    let odd = [
        "$",
        "$*",
        "$GPGGA",
        "$GPGGA,,,,,,1",
        "$GPGGA,ğğğ,ğ,ğ,ğ,ğ,1,ğ,ğ,ğ,ğ,ğ,ğ*00",
        "$GPGGA,1,4045.2345,N,02923.1234,E,1,99999999999999,1,1,M,1,M",
        "$GPGGA,1,4045.234500000000000000000000000000001,N,02923.1234,E,1,1,1,1,M,1,M",
        "$GPGGA,1,4045.2345,N,02923.1234,E,1,1,1,1e999,M,1,M",
        "$GPRMC,1,A,1,N,1,E,1,1,999999",
        "$ÇÇÇÇÇ,1",
        "<gpx><wpt lat='1' lon='1'><sat>99999999999999999999</sat></wpt></gpx>",
        "<gpx><wpt lat='1.000000000000000000000000000000001' lon='1'/></gpx>",
        "<gpx><wpt lat='1' lon='1'><ele>1</ele><geoidheight>0.000000000000000000000000000001</geoidheight></wpt></gpx>",
        "<",
        "<gpx",
    ];
    for o in odd {
        let _ = read(o.as_bytes());
    }
    let deep = format!("<gpx>{}{}</gpx>", "<a>".repeat(5000), "</a>".repeat(5000));
    assert_eq!(read(deep.as_bytes()).problems.len(), 1);
}
