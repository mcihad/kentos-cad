//! A broken block rule (`kentos_contracts::blocks`, docs/adr/0144) said as a
//! file error: its code, where it is and why. The writer and the reader say
//! the same fault the same way.

use kentos_contracts::blocks::{BlockFault, Place};
use kentos_contracts::{BlockDefinition, MAX_BLOCK_DEPTH};

use crate::cbor::Seg;
use crate::error::Code;

/// A fault's code, the document list it is in (`blocks` or `entities`), the
/// path inside that list and the reason.
pub(crate) struct Said {
    pub code: Code,
    pub list: &'static str,
    pub path: Vec<Seg<'static>>,
    pub what: String,
}

pub(crate) fn said(fault: &BlockFault, blocks: &[BlockDefinition]) -> Said {
    let name = |d: usize| blocks.get(d).map_or("", |b| b.name.as_str());
    let in_block = |d: usize, rest: &[Seg<'static>]| {
        let mut path = vec![Seg::Index(d)];
        path.extend_from_slice(rest);
        path
    };
    let insert = |at: &Place, field: &'static str| -> (&'static str, Vec<Seg<'static>>) {
        let tail = [Seg::Index(at.entity), Seg::Name("insert"), Seg::Name(field)];
        match at.definition {
            Some(d) => (
                "blocks",
                in_block(d, &[Seg::Name("entities")])
                    .into_iter()
                    .chain(tail)
                    .collect(),
            ),
            None => ("entities", tail.to_vec()),
        }
    };
    let (code, list, path, what) = match fault {
        BlockFault::EmptyName { definition } => (
            Code::BadValue,
            "blocks",
            in_block(*definition, &[Seg::Name("name")]),
            "blok adı boş ya da yalnız boşluk olamaz".to_owned(),
        ),
        BlockFault::DuplicateId { definition, first } => (
            Code::DuplicateBlock,
            "blocks",
            in_block(*definition, &[Seg::Name("id")]),
            format!(
                "blok kimliği {} {}. tanımda da var; her tanımın kendi kimliği olmalı",
                blocks[*definition].id,
                first + 1
            ),
        ),
        BlockFault::DuplicateName { definition, first } => (
            Code::DuplicateBlock,
            "blocks",
            in_block(*definition, &[Seg::Name("name")]),
            format!(
                "“{}” adı {}. tanımda da var (büyük/küçük harf ayrımı yapılmaz); her bloğun adı ayrı olmalı",
                name(*definition),
                first + 1
            ),
        ),
        BlockFault::EmptyTag {
            definition,
            attribute,
        } => (
            Code::BadValue,
            "blocks",
            in_block(
                *definition,
                &[
                    Seg::Name("attributes"),
                    Seg::Index(*attribute),
                    Seg::Name("tag"),
                ],
            ),
            format!("“{}” bloğunun bir öznitelik etiketi boş", name(*definition)),
        ),
        BlockFault::DuplicateTag {
            definition,
            attribute,
            first,
        } => (
            Code::BadValue,
            "blocks",
            in_block(
                *definition,
                &[
                    Seg::Name("attributes"),
                    Seg::Index(*attribute),
                    Seg::Name("tag"),
                ],
            ),
            format!(
                "“{}” bloğunda “{}” etiketi {}. öznitelikte de var; her etiket bir kez olmalı",
                name(*definition),
                blocks[*definition].attributes[*attribute].tag,
                first + 1
            ),
        ),
        BlockFault::BadScale { at } => {
            let (list, path) = insert(at, "scale");
            (
                Code::BadValue,
                list,
                path,
                "blok ölçeği pozitif ve sonlu olmalı".to_owned(),
            )
        }
        BlockFault::UnknownBlock { at } => {
            let (list, path) = insert(at, "block");
            (
                Code::UnknownBlock,
                list,
                path,
                "yerleştirilen blok çizimde tanımlı değil".to_owned(),
            )
        }
        BlockFault::Cycle { definition } => (
            Code::BlockCycle,
            "blocks",
            in_block(*definition, &[]),
            format!(
                "“{}” bloğu kendini içeriyor (doğrudan ya da başka bloklar yoluyla)",
                name(*definition)
            ),
        ),
        BlockFault::TooDeep { definition } => (
            Code::BlockTooDeep,
            "blocks",
            in_block(*definition, &[]),
            format!(
                "“{}” bloğunda bloklar {MAX_BLOCK_DEPTH} düzeyden derin iç içe",
                name(*definition)
            ),
        ),
    };
    Said {
        code,
        list,
        path,
        what,
    }
}
