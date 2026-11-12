//! Where an office standard's block and sheet-template references are read from (docs/adr/1100 §2–§3): the
//! libraries KentOS can read, in order. A block reference names a publication (family, type, release), never a
//! library's file or a block's local name; the first library holding that publication gives its definition. A sheet
//! template reference names a template id and revision; the first library listing both holds it. KentOS's own
//! libraries are here now (`kentos-temel`, `kentos-ofis`, the system sheet templates); a host plugs a project's,
//! an office folder's or a cloud store's in front of them with [`Libraries::with_blocks`] and
//! [`Libraries::with_templates`] without the install's rules changing.

use std::sync::{Arc, OnceLock};

use kentos_commands::{
    BLOCKS_FORMAT, BlockDefinition, StandardBlockReference, StandardSheetTemplateReference,
};

/// The library name the system sheet templates are listed under.
pub const SYSTEM_TEMPLATES: &str = "sistem";

/// A block library's definitions, by the name it is shown with.
#[derive(Clone, Debug, PartialEq)]
pub struct BlockLibrary {
    pub name: String,
    pub blocks: Vec<BlockDefinition>,
}

/// A sheet template library's templates: each id and revision it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateLibrary {
    pub name: String,
    pub templates: Vec<(String, u32)>,
}

/// The libraries an install reads, searched in order.
#[derive(Clone, Debug, Default)]
pub struct Libraries {
    pub blocks: Vec<Arc<BlockLibrary>>,
    pub templates: Vec<Arc<TemplateLibrary>>,
}

/// A reference's answer: the library and its entry, or what the nearest entry differs in (none: no library has the
/// type or template at all).
#[derive(Debug)]
pub enum Found<'a, T> {
    In { library: &'a str, item: T },
    Not { fields: Vec<String> },
}

/// A `kentos.blocks` file's definitions as a library named `name`; why not when the text is not one.
pub fn read_block_library(name: &str, text: &str) -> Result<BlockLibrary, String> {
    let file: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("JSON değil ({e})"))?;
    if file.get("format").and_then(serde_json::Value::as_str) != Some(BLOCKS_FORMAT) {
        return Err("biçimi kentos.blocks değil".into());
    }
    let blocks: Vec<BlockDefinition> = serde_json::from_value(
        file.get("blocks")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    )
    .map_err(|e| format!("blokları okunmuyor ({e})"))?;
    Ok(BlockLibrary {
        name: name.to_owned(),
        blocks,
    })
}

fn builtin_blocks() -> &'static [Arc<BlockLibrary>] {
    static LIBRARIES: OnceLock<Vec<Arc<BlockLibrary>>> = OnceLock::new();
    LIBRARIES.get_or_init(|| {
        crate::blocks_library::BUILTIN_LIBRARIES
            .iter()
            .filter_map(|(name, text)| read_block_library(name, text).ok().map(Arc::new))
            .collect()
    })
}

fn builtin_templates() -> &'static Arc<TemplateLibrary> {
    static LIBRARY: OnceLock<Arc<TemplateLibrary>> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        Arc::new(TemplateLibrary {
            name: SYSTEM_TEMPLATES.to_owned(),
            templates: kentos_sheet::template::system_templates()
                .iter()
                .map(|t| (t.meta.id.clone(), t.meta.revision))
                .collect(),
        })
    })
}

impl Libraries {
    /// KentOS's own libraries.
    pub fn builtin() -> Self {
        Self {
            blocks: builtin_blocks().to_vec(),
            templates: vec![builtin_templates().clone()],
        }
    }

    /// `library` searched before those already here (a project's or an office's own, read by the host).
    #[must_use]
    pub fn with_blocks(mut self, library: BlockLibrary) -> Self {
        self.blocks.insert(0, Arc::new(library));
        self
    }

    /// `library` searched before those already here.
    #[must_use]
    pub fn with_templates(mut self, library: TemplateLibrary) -> Self {
        self.templates.insert(0, Arc::new(library));
        self
    }

    /// The first library definition of the reference's publication, with the library's other definitions (a nested
    /// insert's); else what the nearest entry differs in: another release of the type (`release`), or the
    /// publication in another unit than the standard approved (`units`).
    pub fn block(
        &self,
        r: &StandardBlockReference,
    ) -> Found<'_, (&BlockDefinition, &[BlockDefinition])> {
        let p = &r.publication;
        let mut nearest: Option<Vec<String>> = None;
        for library in &self.blocks {
            for b in &library.blocks {
                let Some(q) = b.card.as_ref().and_then(|c| c.publication.as_ref()) else {
                    continue;
                };
                if q.family_id != p.family_id || q.type_id != p.type_id {
                    continue;
                }
                if q.release != p.release {
                    nearest.get_or_insert_with(|| vec!["release".into()]);
                    continue;
                }
                if r.units.is_some_and(|u| b.units != Some(u)) {
                    nearest = Some(vec!["units".into()]);
                    continue;
                }
                return Found::In {
                    library: &library.name,
                    item: (b, &library.blocks),
                };
            }
        }
        Found::Not {
            fields: nearest.unwrap_or_default(),
        }
    }

    /// The first library that lists the template at the revision; else `revision` when one lists it at another.
    pub fn template(&self, r: &StandardSheetTemplateReference) -> Found<'_, ()> {
        let id = r.template_id.to_lowercase();
        let mut other = false;
        for library in &self.templates {
            for (t, revision) in &library.templates {
                if t.to_lowercase() != id {
                    continue;
                }
                if *revision == r.revision {
                    return Found::In {
                        library: &library.name,
                        item: (),
                    };
                }
                other = true;
            }
        }
        Found::Not {
            fields: if other {
                vec!["revision".into()]
            } else {
                Vec::new()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(
        release: u32,
        units: Option<kentos_commands::BlockUnits>,
    ) -> StandardBlockReference {
        serde_json::from_value(serde_json::json!({
            "name": "KAPI",
            "publication": {"familyId": "0192f5b1-0000-7000-8000-0000000f0001",
                "typeId": "0192f5b1-0000-7000-8000-0000000e0001", "typeName": "Tek kanatlı kapı 90",
                "release": release},
            "units": units,
        }))
        .expect("reference")
    }

    #[test]
    fn the_office_library_holds_both_door_releases_and_says_why_not_otherwise() {
        let libraries = Libraries::builtin();
        for release in [1, 2] {
            match libraries.block(&reference(release, Some(kentos_commands::BlockUnits::M))) {
                Found::In {
                    library,
                    item: (b, _),
                } => {
                    assert_eq!(library, "kentos-ofis");
                    assert_eq!(b.name, format!("Ofis kapı 90 · yayın {release}"));
                }
                Found::Not { fields } => panic!("{release}: {fields:?}"),
            }
        }
        assert!(
            matches!(libraries.block(&reference(3, None)), Found::Not { fields } if fields == ["release"])
        );
        assert!(matches!(
            libraries.block(&reference(1, Some(kentos_commands::BlockUnits::Mm))),
            Found::Not { fields } if fields == ["units"]
        ));
    }

    #[test]
    fn a_plugged_library_is_searched_first() {
        let mut copy = Libraries::builtin().blocks[1].as_ref().clone();
        copy.name = "proje".into();
        let libraries = Libraries::builtin().with_blocks(copy);
        assert!(matches!(
            libraries.block(&reference(1, None)),
            Found::In {
                library: "proje",
                ..
            }
        ));
        let templates = Libraries::builtin().with_templates(TemplateLibrary {
            name: "ofis".into(),
            templates: vec![("ofis:a3".into(), 2)],
        });
        let t = |id: &str, revision| StandardSheetTemplateReference {
            template_id: id.into(),
            revision,
        };
        assert!(matches!(
            templates.template(&t("OFIS:A3", 2)),
            Found::In {
                library: "ofis",
                ..
            }
        ));
        assert!(matches!(
            templates.template(&t("sys:cad-mimari", 1)),
            Found::In {
                library: "sistem",
                ..
            }
        ));
        assert!(
            matches!(templates.template(&t("sys:cad-mimari", 2)), Found::Not { fields } if fields == ["revision"])
        );
        assert!(
            matches!(templates.template(&t("yok", 1)), Found::Not { fields } if fields.is_empty())
        );
    }
}
