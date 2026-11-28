//! SORTENTSTABLE: source handles are sort keys, not KentOS entity identities.
use super::entity::Parsed;
use super::lexer::Lexer;
use super::strings::Decoder;
use std::collections::HashMap;

/// What the drawing's objects say, read in one pass over the file.
#[derive(Debug, Default)]
pub struct Objects {
    /// SORTENTSTABLE: source handle → sort key.
    pub order: HashMap<u64, u64>,
    /// Whether its wipeouts show their frames (WIPEOUTVARIABLES' 70; none: no such object).
    pub wipeout_frames: Option<bool>,
    /// Each IMAGEDEF's file (its 1) by its handle (docs/adr/9227).
    pub images: HashMap<u64, String>,
}

/// The drawing's objects (see [`Objects`]).
pub fn mapping(bytes: &[u8], dec: Decoder) -> Result<Objects, String> {
    let mut lex = Lexer::new(bytes);
    let mut in_wipeouts = false;
    let mut frames = None;
    let mut in_image = false;
    let mut image: Option<u64> = None;
    let mut images = HashMap::new();
    let mut in_table = false;
    let mut subclass = false;
    let mut entity = None;
    let mut out = HashMap::new();
    let mut sort_var = false;
    let mut enabled = true;
    while let Some(p) = lex.next()? {
        if p.code == 9 {
            sort_var = p.text().trim() == "$SORTENTS";
        }
        if sort_var && p.code == 70 {
            enabled = p.text().trim().parse::<u32>().unwrap_or(16) & 16 != 0;
            sort_var = false;
        }
        if in_wipeouts && p.code == 70 {
            frames = Some(p.text().trim().parse::<i64>().unwrap_or(1) != 0);
        }
        if in_image && p.code == 5 {
            image = u64::from_str_radix(p.text().trim(), 16).ok();
        }
        if in_image
            && p.code == 1
            && let Some(h) = image.take()
        {
            // The file's name is the drawing's string: in its code page (docs/adr/9401 §3).
            images.insert(h, dec.string(p.value).trim().to_string());
        }
        if p.code == 0 {
            in_wipeouts = p.text().eq_ignore_ascii_case("WIPEOUTVARIABLES");
            in_image = p.text().eq_ignore_ascii_case("IMAGEDEF");
            image = None;
            in_table = p.text().eq_ignore_ascii_case("SORTENTSTABLE");
            subclass = false;
            entity = None;
        } else if in_table && p.code == 100 {
            subclass = p.text() == "AcDbSortentsTable";
        } else if in_table && subclass && p.code == 331 {
            entity = u64::from_str_radix(p.text().trim(), 16).ok();
        } else if in_table
            && subclass
            && p.code == 5
            && let (Some(entity), Ok(sort)) =
                (entity.take(), u64::from_str_radix(p.text().trim(), 16))
        {
            out.insert(entity, sort);
        }
    }
    if !enabled {
        out.clear();
    }
    Ok(Objects {
        order: out,
        wipeout_frames: frames,
        images,
    })
}

pub fn reorder(entities: &mut [Parsed], map: &HashMap<u64, u64>) {
    if entities
        .iter()
        .any(|e| e.handle.is_some_and(|h| map.contains_key(&h)))
    {
        // Stable sort: equal keys keep their source order; sort handle 0 is last.
        entities.sort_by_key(|e| {
            let handle = e.handle.unwrap_or(u64::MAX);
            let key = map.get(&handle).copied().unwrap_or(handle);
            if key == 0 { u64::MAX } else { key }
        });
    }
}
