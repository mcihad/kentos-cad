//! SORTENTSTABLE: source handles are sort keys, not KentOS entity identities.
use super::entity::Parsed;
use super::lexer::Lexer;
use std::collections::HashMap;

pub fn mapping(bytes: &[u8]) -> Result<HashMap<u64, u64>, String> {
    let mut lex = Lexer::new(bytes);
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
        if p.code == 0 {
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
    Ok(out)
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
