//! The builder's positions are UTF-16 code units (the web's text fields
//! count so); the desktop's editor counts bytes within lines. These convert
//! both ways, clamped to the text; a position inside a character counts as
//! the character's start.

/// The UTF-16 position of a byte offset.
pub fn from_byte(s: &str, byte: usize) -> usize {
    s.char_indices()
        .take_while(|(b, _)| *b < byte)
        .map(|(_, c)| c.len_utf16())
        .sum()
}

/// The byte offset of a UTF-16 position.
pub fn to_byte(s: &str, unit: usize) -> usize {
    let mut units = 0;
    for (b, c) in s.char_indices() {
        if units + c.len_utf16() > unit {
            return b;
        }
        units += c.len_utf16();
    }
    s.len()
}

/// The UTF-16 position of a line (from 0, lines end at `\n`) and a byte
/// column in it.
pub fn from_line_byte(s: &str, line: usize, column: usize) -> usize {
    let mut start = 0;
    for _ in 0..line {
        match s[start..].find('\n') {
            Some(n) => start += n + 1,
            None => return from_byte(s, s.len()),
        }
    }
    let end = s[start..].find('\n').map_or(s.len(), |n| start + n);
    from_byte(s, (start + column).min(end))
}

/// The line and byte column of a UTF-16 position.
pub fn to_line_byte(s: &str, unit: usize) -> (usize, usize) {
    let byte = to_byte(s, unit);
    let line = s[..byte].matches('\n').count();
    let start = s[..byte].rfind('\n').map_or(0, |n| n + 1);
    (line, byte - start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_bytes_and_lines_agree() {
        let s = "ağ\n𝒜 + x";
        // a=1 byte/1 unit, ğ=2/1, \n=1/1, 𝒜=4/2, space…
        assert_eq!(from_byte(s, 3), 2);
        assert_eq!(from_byte(s, 8), 5);
        assert_eq!(to_byte(s, 5), 8);
        assert_eq!(to_byte(s, 4), 4, "inside 𝒜: its start");
        assert_eq!(to_byte(s, 99), s.len());
        assert_eq!(from_line_byte(s, 1, 4), 5);
        assert_eq!(from_line_byte(s, 1, 99), 9);
        assert_eq!(to_line_byte(s, 5), (1, 4));
        assert_eq!(to_line_byte(s, 2), (0, 3));
    }
}
