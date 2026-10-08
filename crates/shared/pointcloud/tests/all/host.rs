//! A host for the tests: a file's bytes in memory, handed over as the readers ask.

use std::path::{Path, PathBuf};

use kentos_pointcloud::source::{Cloud, Opening};
use kentos_pointcloud::{PcError, Step};

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/pointcloud/v1")
}

pub fn cases() -> serde_json::Value {
    let text = std::fs::read_to_string(fixtures().join("cases.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// A cloud opened from bytes, every run asked for handed over at once.
pub fn open(bytes: &[u8]) -> Result<Cloud, PcError> {
    let mut o = Opening::new(bytes.len() as u64);
    for _ in 0..10_000 {
        match o.step()? {
            Step::Done(c) => return Ok(c),
            Step::Need(n) => {
                let a = n.offset as usize;
                let b = (n.offset + n.len) as usize;
                o.put(n.offset, bytes[a..b.min(bytes.len())].to_vec());
            }
        }
    }
    panic!("opening never ended");
}

/// Every record of a cloud, in its order.
pub fn records(cloud: &Cloud, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for run in cloud.runs() {
        let a = run.need.offset as usize;
        let b = (run.need.offset + run.need.len) as usize;
        cloud.records(&run, &bytes[a..b], &mut out).unwrap();
    }
    out
}

pub fn fnv(data: &[u8]) -> String {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{h:016x}")
}
