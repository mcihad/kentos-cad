//! A host for the tests: the file's bytes in memory, a job run to its end.

use kentos_formats::raster::source::{Put, Reader, open_bytes};
use kentos_raster::job::{Finished, Job, READER_BUDGET, Spec};

pub const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/");

pub fn read(rel: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{rel}")).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The raster of `bytes`, only its header read: its blocks come when the job asks.
pub fn opened(bytes: &[u8]) -> Reader {
    open_bytes(bytes, None, READER_BUDGET).expect("the raster opens")
}

/// What a run gives: the result file's bytes, or the lines.
pub enum Ran {
    Raster(Vec<u8>),
    Lines(Vec<kentos_raster::contours::Line>),
}

/// `spec` run over the file `bytes` on `threads`, its blocks handed over as asked.
pub fn run(bytes: &[u8], spec: &Spec, threads: usize) -> Result<Ran, String> {
    let (mut job, header) = Job::new(opened(bytes), spec, threads)?;
    let mut file = header;
    let mut last = -1.0;
    while !job.done() {
        let needs = job.needs();
        // Half the runs hand the blocks over one by one, half all at once (decoded on the threads).
        if threads % 2 == 1 {
            for need in needs {
                let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
                match job.put(&need, &bytes[a..b])? {
                    Put::Done => {}
                    Put::Jpeg(_) => return Err("a JPEG block in a test".into()),
                }
            }
        } else {
            let blocks = needs
                .into_iter()
                .map(|n| {
                    (
                        n,
                        bytes[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                    )
                })
                .collect();
            if !job.put_all(blocks)?.is_empty() {
                return Err("a JPEG block in a test".into());
            }
        }
        file.extend(job.step()?);
        let share = job.share();
        assert!(share >= last, "the share goes up ({last} → {share})");
        last = share;
    }
    match job.finish()? {
        Finished::Raster { tail, header } => {
            file.extend(tail);
            file[..header.len()].copy_from_slice(&header);
            Ok(Ran::Raster(file))
        }
        Finished::Lines(lines) => Ok(Ran::Lines(lines)),
    }
}
