//! The file's head (ISO 10303-21 §8): `ISO-10303-21;`, then the `HEADER` section's FILE_DESCRIPTION, FILE_NAME and
//! FILE_SCHEMA. Each header statement is cut out by our own quote-aware walk, bounded to the head's first megabyte,
//! and handed to the engine's tokenizer as if it were a record, so its strings are decoded as the records' are.

use ifc_lite_core::{AttributeValue, SchemaVersion};

use super::{Family, Header, ReadError};

/// How far the header may reach.
const HEAD: usize = 1 << 20;

/// Whether the bytes start as a STEP file: `ISO-10303-21;` after an optional byte order mark and white space.
pub(super) fn starts_as_step(bytes: &[u8]) -> bool {
    let b = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let at = b
        .iter()
        .position(|c| !c.is_ascii_whitespace())
        .unwrap_or(b.len());
    b[at..].starts_with(b"ISO-10303-21;")
}

/// The family of a declared schema; none for a schema the engine does not know.
pub(super) fn family(schema: Option<&str>) -> Option<Family> {
    Some(match SchemaVersion::from_file_schema(schema?)? {
        SchemaVersion::Ifc2x3 => Family::Ifc2x3,
        SchemaVersion::Ifc4 => Family::Ifc4,
        SchemaVersion::Ifc4x1 => Family::Ifc4x1,
        SchemaVersion::Ifc4x2 => Family::Ifc4x2,
        SchemaVersion::Ifc4x3 => Family::Ifc4x3,
        _ => return None,
    })
}

/// The engine's schema for a family.
pub(super) fn version(family: Family) -> SchemaVersion {
    match family {
        Family::Ifc2x3 => SchemaVersion::Ifc2x3,
        Family::Ifc4 => SchemaVersion::Ifc4,
        Family::Ifc4x1 => SchemaVersion::Ifc4x1,
        Family::Ifc4x2 => SchemaVersion::Ifc4x2,
        Family::Ifc4x3 => SchemaVersion::Ifc4x3,
    }
}

/// The header and the first schema FILE_SCHEMA names. A statement the tokenizer cannot read is left out: the header
/// says who wrote the file, the records say what it is.
pub(super) fn read(bytes: &[u8]) -> Result<(Header, Option<String>), ReadError> {
    let head = &bytes[..bytes.len().min(HEAD)];
    let Some(start) = find(head, b"HEADER;") else {
        return Ok((
            Header::default(),
            ifc_lite_core::declared_schema_bounded(bytes),
        ));
    };
    let mut header = Header::default();
    let mut schema = None;
    for statement in statements(&head[start + 7..]) {
        let Some(open) = statement.iter().position(|&c| c == b'(') else {
            continue;
        };
        // A comment before the keyword is passed over.
        let from = statement[..open]
            .windows(2)
            .rposition(|w| w == b"*/")
            .map_or(0, |at| at + 2);
        let keyword = String::from_utf8_lossy(&statement[from..open])
            .trim()
            .to_ascii_uppercase();
        let mut record = b"#1=".to_vec();
        record.extend_from_slice(statement[from..].trim_ascii_start());
        record.push(b';');
        let Ok((_, _, tokens)) = ifc_lite_core::parse_entity(&record) else {
            continue;
        };
        let values: Vec<AttributeValue> = tokens.iter().map(AttributeValue::from_token).collect();
        let text = |i: usize| {
            values
                .get(i)
                .and_then(AttributeValue::as_string)
                .map(str::to_owned)
        };
        let texts = |i: usize| -> Vec<String> {
            values
                .get(i)
                .and_then(AttributeValue::as_list)
                .map(|l| {
                    l.iter()
                        .filter_map(AttributeValue::as_string)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default()
        };
        match keyword.as_str() {
            "FILE_DESCRIPTION" => {
                header.description = texts(0);
                header.implementation_level = text(1);
            }
            "FILE_NAME" => {
                header.name = text(0);
                header.time_stamp = text(1);
                header.author = texts(2);
                header.organization = texts(3);
                header.preprocessor = text(4);
                header.originating_system = text(5);
                header.authorization = text(6);
            }
            "FILE_SCHEMA" => schema = texts(0).into_iter().find(|s| !s.is_empty()),
            _ => {}
        }
    }
    Ok((
        header,
        schema.or_else(|| ifc_lite_core::declared_schema_bounded(bytes)),
    ))
}

/// Where `needle` starts in `hay`, outside quoted strings.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    let mut quoted = false;
    let mut i = 0;
    while i < hay.len() {
        let c = hay[i];
        if c == b'\'' {
            quoted = !quoted;
        } else if !quoted && hay[i..].starts_with(needle) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The header's statements up to `ENDSEC;`, each without its `;`, cut outside quoted strings and comments.
fn statements(section: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let (mut quoted, mut comment, mut from) = (false, false, 0);
    let mut i = 0;
    while i < section.len() {
        let c = section[i];
        if comment {
            if section[i..].starts_with(b"*/") {
                comment = false;
                i += 1;
            }
        } else if c == b'\'' {
            quoted = !quoted;
        } else if !quoted && section[i..].starts_with(b"/*") {
            comment = true;
            i += 1;
        } else if !quoted && c == b';' {
            let statement = section[from..i].trim_ascii();
            if statement.eq_ignore_ascii_case(b"ENDSEC") {
                break;
            }
            out.push(statement);
            from = i + 1;
        }
        i += 1;
    }
    out
}
