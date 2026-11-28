//! ISO 10303-21 (STEP) writing for IFC files (docs/adr/0908 §8, docs/adr/8047): reals always with a point, strings
//! with their apostrophes and backslashes doubled and what is not printable ASCII as `\X2\…\X0\`, GlobalIds of
//! 128-bit numbers, the instances numbered as they are added, and the file's header and its end.
//!
//! **GlobalIds** (docs/adr/8117; plan/ifc-uyum.md §7 "Kimlik"; IFC 4.3, Software Identity: the IFC-GUID "has to be
//! persistent"): an object with a KentOS id is written with that id ([`guid`]); a part made without one (a storey, an
//! opening, a zone, a group, a property set, a relationship) with one derived from its role and its owners' GlobalIds
//! ([`Step::derived`], [`derived_id`]) — never from the file's name, its time stamp or the order of writing, so the
//! same drawing written another day or after an unrelated change keeps them.

/// A real as STEP writes it: always with a point.
pub(crate) fn real(v: f64) -> String {
    let v = if v == 0.0 { 0.0 } else { v };
    let t = format!("{v}");
    if t.contains('.') || t.contains('e') || t.contains("inf") || t.contains("NaN") {
        t
    } else {
        format!("{t}.")
    }
}

/// A string as STEP writes it: an apostrophe doubled, a backslash doubled, what is not printable ASCII as `\X2\…\X0\`.
pub(crate) fn text(s: &str) -> String {
    let mut out = String::from("'");
    let mut wide = String::new();
    let flush = |wide: &mut String, out: &mut String| {
        if !wide.is_empty() {
            out.push_str("\\X2\\");
            out.push_str(wide);
            out.push_str("\\X0\\");
            wide.clear();
        }
    };
    for c in s.chars() {
        if (' '..='~').contains(&c) {
            flush(&mut wide, &mut out);
            match c {
                '\'' => out.push_str("''"),
                '\\' => out.push_str("\\\\"),
                c => out.push(c),
            }
        } else {
            let mut units = [0u16; 2];
            for u in c.encode_utf16(&mut units) {
                wide.push_str(&format!("{u:04X}"));
            }
        }
    }
    flush(&mut wide, &mut out);
    out.push('\'');
    out
}

const GUID: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

/// An IFC GlobalId (22 characters) of a 128-bit number: the first character its top 2 bits, then 6 bits each.
pub fn guid(n: u128) -> String {
    let mut out = String::with_capacity(22);
    out.push(GUID[(n >> 126) as usize] as char);
    for i in (0..21).rev() {
        out.push(GUID[((n >> (6 * i)) & 63) as usize] as char);
    }
    out
}

/// The 128-bit number of an IFC GlobalId, the inverse of [`guid`]: KentOS writes an element's GlobalId from its
/// persistent id, so a file it wrote leads back to the drawing's objects. None for 22 characters that are not a
/// GlobalId's (another character, or a first one worth more than 2 bits).
pub fn unguid(s: &str) -> Option<u128> {
    let b = s.as_bytes();
    if b.len() != 22 {
        return None;
    }
    let digit = |c: u8| GUID.iter().position(|&g| g == c).map(|i| i as u128);
    let first = digit(b[0]).filter(|&d| d < 4)?;
    b[1..]
        .iter()
        .try_fold(first, |n, &c| Some((n << 6) | digit(c)?))
}

/// The GlobalId a root's line starts with (`IFCWALL('…',`): its first attribute, 22 characters in apostrophes.
fn root_id(body: &str) -> Option<&str> {
    let rest = &body[body.find('(')? + 1..];
    let b = rest.as_bytes();
    (b.len() > 23 && b[0] == b'\'' && b[23] == b'\'').then(|| &rest[1..23])
}

/// FNV-1a over 128 bits.
pub(crate) fn fnv(seed: &str, k: u64) -> u128 {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for b in seed.bytes().chain(k.to_le_bytes()) {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    h
}

/// The 128 bits of a part's GlobalId made without a KentOS id: its `role` and the `key` that names it among its kind
/// (an owner's id, a name). The same for the same role and key in every file and every revision. Shaped as a UUID of
/// version 8 (RFC 9562's custom UUIDs): KentOS's own ids are version 7 (version 5 for an old file's), so
/// [`is_derived`] tells a part the export made from an object of the drawing.
pub fn derived_id(role: &str, key: &str) -> u128 {
    let h = fnv(&format!("kentos|{role}|{key}"), 0);
    let h = (h & !(0xF_u128 << 76)) | (0x8_u128 << 76);
    (h & !(0x3_u128 << 62)) | (0x2_u128 << 62)
}

/// Whether `id` is one [`derived_id`] made (a UUID of version 8).
pub fn is_derived(id: u128) -> bool {
    (id >> 76) & 0xF == 0x8 && (id >> 62) & 0x3 == 0x2
}

/// The file's instances as they are added (#1, #2, …), and its GlobalIds' seed for a writer that still numbers them
/// ([`Step::gid`], the road's alignment file).
pub(crate) struct Step {
    pub(crate) lines: Vec<String>,
    pub(crate) seed: String,
    pub(crate) ids: u64,
    /// The GlobalIds of the roots added, by their instance numbers: a derived part names its owners by them.
    roots: std::collections::HashMap<usize, String>,
    /// The derived keys given: one asked again (two parts of the same role and owners) takes a mark more.
    keys: std::collections::HashSet<String>,
    /// The building the file is (its GlobalId): in every derived key, so two buildings' own files of one project share
    /// no part but their project and site (docs/adr/8072 §2, 8117).
    pub(crate) scope: String,
}

impl Step {
    pub(crate) fn new(seed: String) -> Step {
        Step {
            lines: Vec::new(),
            seed,
            ids: 0,
            roots: std::collections::HashMap::new(),
            keys: std::collections::HashSet::new(),
            scope: String::new(),
        }
    }
    pub(crate) fn add(&mut self, body: impl Into<String>) -> usize {
        let body = body.into();
        let n = self.lines.len() + 1;
        if let Some(g) = root_id(&body) {
            self.roots.insert(n, g.to_owned());
        }
        self.lines.push(body);
        n
    }

    /// A derived part's GlobalId (quoted): its `role`, the GlobalIds of its `owners` (by instance number) and `name`.
    /// An owner without one is named by its number; a key already given takes a `+` more, so no two parts share one.
    pub(crate) fn derived(&mut self, role: &str, owners: &[usize], name: &str) -> String {
        let mut key = format!("{}|", self.scope);
        for o in owners {
            match self.roots.get(o) {
                Some(g) => key.push_str(g),
                None => key.push_str(&format!("#{o}")),
            }
            key.push('|');
        }
        key.push_str(name);
        while !self.keys.insert(format!("{role}|{key}")) {
            key.push('+');
        }
        text(&guid(derived_id(role, &key)))
    }
    pub(crate) fn gid(&mut self) -> String {
        self.ids += 1;
        text(&guid(fnv(&self.seed, self.ids)))
    }
    pub(crate) fn refs(ids: &[usize]) -> String {
        ids.iter()
            .map(|i| format!("#{i}"))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The whole file: its header (the view definition `view`, the file's `name`, its time `stamp`, the application
    /// as author and as system), its instances and its end.
    pub(crate) fn file(&self, view: &str, name: &str, stamp: &str, application: &str) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(({}),'2;1');\n",
            text(view)
        ));
        out.push_str(&format!(
            "FILE_NAME({},{},(''),(''),{},{},'');\nFILE_SCHEMA(('IFC4X3_ADD2'));\nENDSEC;\nDATA;\n",
            text(name),
            text(stamp),
            text(application),
            text(application)
        ));
        for (i, l) in self.lines.iter().enumerate() {
            out.push_str(&format!("#{}={};\n", i + 1, l));
        }
        out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
        out
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_global_id_reads_back_to_its_number() {
        for n in [
            0u128,
            1,
            7,
            u128::MAX,
            0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
            1 << 126,
        ] {
            assert_eq!(unguid(&guid(n)), Some(n), "{n:x}");
        }
        assert_eq!(unguid("0000000000000000000000"), Some(0));
        assert_eq!(
            unguid("4000000000000000000000"),
            None,
            "the first character holds 2 bits"
        );
        assert_eq!(unguid("000000000000000000000"), None, "21 characters");
        assert_eq!(
            unguid("00000000000000000000-0"),
            None,
            "not a GlobalId's character"
        );
    }
    use super::*;

    #[test]
    fn a_guid_has_22_characters_of_its_alphabet() {
        let g = guid(u128::MAX);
        assert_eq!(g.len(), 22);
        assert_eq!(g, "3$$$$$$$$$$$$$$$$$$$$$");
        assert_eq!(guid(0), "0000000000000000000000");
    }

    #[test]
    fn strings_and_reals_are_written_as_step_writes_them() {
        assert_eq!(text("Ana aks'ı"), "'Ana aks''\\X2\\0131\\X0\\'");
        assert_eq!(real(150.0), "150.");
        assert_eq!(real(-0.0), "0.");
        assert_eq!(real(0.25), "0.25");
    }
}
