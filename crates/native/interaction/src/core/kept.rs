//! The short texts and number lists the tools' [`crate::tool::Memory`] keeps, which is `Copy` (split from tool.rs):
//! an attribute's name, a network's id, Ara nokta's distances, Hizmet alanı's breaks.

/// A short text kept in [`crate::tool::Memory`], which is `Copy`: at most
/// [`Name::MAX_CHARS`] characters (an attribute's name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Name {
    len: u16,
    bytes: [u8; Name::MAX_BYTES],
}

impl Name {
    pub const MAX_CHARS: usize = 60;
    const MAX_BYTES: usize = Self::MAX_CHARS * 4;
    /// The empty text.
    pub const EMPTY: Self = Self {
        len: 0,
        bytes: [0; Self::MAX_BYTES],
    };

    /// The text, or `None` when it is longer than [`Name::MAX_CHARS`] characters.
    pub fn new(text: &str) -> Option<Self> {
        if text.chars().count() > Self::MAX_CHARS {
            return None;
        }
        let mut bytes = [0; Self::MAX_BYTES];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Some(Self {
            len: text.len() as u16,
            bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        // Written from a `&str` in `new`: always UTF-8.
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

/// A short list of numbers typed as one answer (Ara nokta's distances and
/// ratios), kept in [`crate::tool::Memory`], which is `Copy`: at most [`Values::MAX`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Values {
    len: u8,
    v: [f64; Values::MAX],
}

impl Values {
    pub const MAX: usize = 32;

    /// The list, or `None` when it is longer than [`Values::MAX`].
    pub fn from_slice(list: &[f64]) -> Option<Self> {
        if list.len() > Self::MAX {
            return None;
        }
        let mut v = [0.0; Self::MAX];
        v[..list.len()].copy_from_slice(list);
        Some(Self {
            len: list.len() as u8,
            v,
        })
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.v[..usize::from(self.len)]
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for Values {
    fn default() -> Self {
        Self {
            len: 0,
            v: [0.0; Self::MAX],
        }
    }
}
