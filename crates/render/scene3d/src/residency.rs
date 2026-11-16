//! Which contents live on the GPU (plan/3d.md SCENE-STREAM-10, 11): a frame asks for contents by priority; this
//! decides, without the GPU, which to upload now and which to let go. Two budgets: the bytes resident at once and the
//! bytes uploaded in one frame (so a large scene arriving at once does not stall a frame; the first content of a frame
//! always goes, so nothing starves). To make room it lets go of the contents used longest ago, never one this frame
//! draws or has just uploaded; what does not fit waits for a later frame.

use std::collections::HashMap;

/// The two budgets, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budgets {
    pub resident: u64,
    pub upload: u64,
}

impl Default for Budgets {
    /// 1 GiB resident and 64 MiB a frame: what an integrated GPU of today keeps for a view without crowding the rest
    /// of the screen, and about a frame's worth of bus at 60 frames a second. A host with a measured device sets its
    /// own (plan/3d.md SCENE-DEC-06).
    fn default() -> Self {
        Budgets { resident: 1 << 30, upload: 64 << 20 }
    }
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    bytes: u64,
    used: u64,
}

/// What a frame decided.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// The contents to upload now, in the asked order.
    pub upload: Vec<u64>,
    /// The contents to let go.
    pub evict: Vec<u64>,
    /// The contents asked for that wait for a later frame.
    pub waiting: Vec<u64>,
}

/// The contents resident and when each was last used.
#[derive(Clone, Debug, Default)]
pub struct Residency {
    entries: HashMap<u64, Entry>,
    bytes: u64,
    frame: u64,
}

impl Residency {
    pub fn is_resident(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    /// The bytes resident and how many contents.
    pub fn resident(&self) -> (u64, usize) {
        (self.bytes, self.entries.len())
    }

    /// A new frame: what it marks used is kept this frame.
    pub fn begin(&mut self) {
        self.frame += 1;
    }

    /// The content is drawn this frame.
    pub fn touch(&mut self, id: u64) {
        let frame = self.frame;
        if let Some(e) = self.entries.get_mut(&id) {
            e.used = frame;
        }
    }

    /// The plan for the contents asked for (`(id, bytes)`, the most needed first) within `budgets`; the plan is taken
    /// as done (the uploaded become resident, used this frame; the evicted go).
    pub fn plan(&mut self, asked: &[(u64, u64)], budgets: Budgets) -> Plan {
        let mut out = Plan::default();
        let mut sent: u64 = 0;
        for &(id, bytes) in asked {
            if self.entries.contains_key(&id) {
                self.touch(id);
                continue;
            }
            if !out.upload.is_empty() && sent + bytes > budgets.upload {
                out.waiting.push(id);
                continue;
            }
            if self.bytes + bytes > budgets.resident {
                // Let go of the longest unused, never one of this frame.
                let mut old: Vec<(u64, u64)> =
                    self.entries.iter().filter(|(_, e)| e.used < self.frame).map(|(k, e)| (e.used, *k)).collect();
                old.sort_unstable();
                let mut freed = Vec::new();
                let mut room = budgets.resident.saturating_sub(self.bytes);
                for (_, k) in old {
                    if room >= bytes {
                        break;
                    }
                    room += self.entries[&k].bytes;
                    freed.push(k);
                }
                if room < bytes {
                    out.waiting.push(id);
                    continue;
                }
                for k in freed {
                    if let Some(e) = self.entries.remove(&k) {
                        self.bytes -= e.bytes;
                    }
                    out.evict.push(k);
                }
            }
            self.entries.insert(id, Entry { bytes, used: self.frame });
            self.bytes += bytes;
            sent += bytes;
            out.upload.push(id);
        }
        out
    }

    /// Forgets a content (its source changed: it is uploaded again when asked).
    pub fn forget(&mut self, id: u64) -> bool {
        match self.entries.remove(&id) {
            Some(e) => {
                self.bytes -= e.bytes;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: Budgets = Budgets { resident: 100, upload: 50 };

    #[test]
    fn a_frame_uploads_within_its_budget_and_the_rest_waits() {
        let mut r = Residency::default();
        r.begin();
        let p = r.plan(&[(1, 30), (2, 30), (3, 10)], B);
        assert_eq!(p.upload, vec![1, 3]);
        assert_eq!(p.waiting, vec![2]);
        r.begin();
        let p = r.plan(&[(1, 30), (2, 30), (3, 10)], B);
        assert_eq!(p.upload, vec![2]);
        assert_eq!(r.resident(), (70, 3));
    }

    #[test]
    fn the_first_content_goes_even_above_the_upload_budget() {
        let mut r = Residency::default();
        r.begin();
        let p = r.plan(&[(1, 80), (2, 5)], B);
        assert_eq!(p.upload, vec![1]);
        assert_eq!(p.waiting, vec![2]);
    }

    #[test]
    fn the_longest_unused_go_first_never_this_frames() {
        let mut r = Residency::default();
        for (id, bytes) in [(1, 40), (2, 40)] {
            r.begin();
            r.plan(&[(id, bytes)], B);
        }
        r.begin();
        r.touch(1);
        // 1 is drawn this frame, 2 not: 2 goes for 3.
        let p = r.plan(&[(1, 40), (3, 40)], B);
        assert_eq!(p.evict, vec![2]);
        assert_eq!(p.upload, vec![3]);
        // A content larger than all that may go waits.
        r.begin();
        let p = r.plan(&[(1, 40), (3, 40), (4, 90)], Budgets { resident: 100, upload: 200 });
        assert!(p.evict.is_empty());
        assert_eq!(p.waiting, vec![4]);
        assert!(r.forget(3));
        assert_eq!(r.resident(), (40, 1));
    }
}
