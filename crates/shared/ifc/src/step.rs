//! ISO 10303-21 (STEP) writing for IFC files (docs/adr/0908 §8, docs/adr/8047): reals always with a point, strings
//! with their apostrophes and backslashes doubled and what is not printable ASCII as `\X2\…\X0\`, GlobalIds of
//! 128-bit numbers (FNV-1a of a seed: the same for the same file), the instances numbered as they are added, and the
//! file's header and its end.

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

/// FNV-1a over 128 bits: the GlobalIds are the same for the same file.
pub(crate) fn fnv(seed: &str, k: u64) -> u128 {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for b in seed.bytes().chain(k.to_le_bytes()) {
        h ^= b as u128;
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    h
}

/// The file's instances as they are added (#1, #2, …), and its GlobalIds' seed.
pub(crate) struct Step {
    pub(crate) lines: Vec<String>,
    pub(crate) seed: String,
    pub(crate) ids: u64,
}

impl Step {
    pub(crate) fn new(seed: String) -> Step {
        Step {
            lines: Vec::new(),
            seed,
            ids: 0,
        }
    }
    pub(crate) fn add(&mut self, body: impl Into<String>) -> usize {
        self.lines.push(body.into());
        self.lines.len()
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
