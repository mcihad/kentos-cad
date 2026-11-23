//! GIS-035 (docs/adr/1092): a point cloud's result never takes an earlier one's file by itself — two runs with no
//! output asked, the forward and the inverse, get two files beside the scan. A file the user picks that is there is
//! said under Çıktı dosyası before the run writes over it: Harita's Kayıt window shows it (the map package's
//! register_overwrite_tests.rs, docs/adr/9199).

use std::collections::HashMap;

use kentos_processing::files::{Beside, Files};

/// The scan beside which both automatic results go: forward then inverse, each its own file, neither the scan's.
#[test]
fn the_forward_and_the_inverse_take_two_files_beside_the_scan() {
    let dir = crate::files_testing::scratch("bulut-otomatik-ad");
    let scan = dir.join("Tarama-2.laz");
    std::fs::write(&scan, b"scan").expect("a scan");
    let files = crate::pointclouds::files::DesktopFiles::new(Some(dir.clone()), HashMap::new());
    let file = scan.to_string_lossy().into_owned();
    let beside = || Beside {
        file: Some(&file),
        url: None,
        asset: None,
        name: None,
        fallback: "bulut",
    };
    let forward = files
        .output_path("", Some(beside()), "-donusmus", ".laz")
        .expect("a path");
    let inverse = files
        .output_path("", Some(beside()), "-donusmus", ".laz")
        .expect("a path");
    assert_ne!(forward, inverse, "two runs, two files");
    assert_ne!(forward, file);
    assert!(forward.ends_with("Tarama-2-donusmus.laz"), "{forward}");
    assert!(inverse.ends_with("Tarama-2-donusmus-2.laz"), "{inverse}");
}
