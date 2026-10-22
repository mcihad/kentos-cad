//! A template becomes one atomic set of book operations, shared by the gallery and automation (ADR 0592).
use crate::{
    Result,
    model::SheetId,
    ops::{AddAssets, AddMaster, AddSheet, Op},
    template::{self, AssetWithBytes, InstanceIds, InstanceOptions, Template},
};

pub struct Created {
    pub sheet: SheetId,
    pub ops: Vec<Op>,
    pub pictures: Vec<AssetWithBytes>,
}

/// Deterministic ids for automation; the gallery may supply its own ids to `from_template`.
pub fn ids_for(template: &Template, sheet: &str) -> InstanceIds {
    let prefix = if sheet.chars().count() > 90 {
        template::sha256_hex(sheet.as_bytes())
    } else {
        sheet.to_owned()
    };
    InstanceIds {
        sheet: sheet.into(),
        master: template.master.as_ref().map(|_| format!("{prefix}-master")),
        items: template
            .sheet
            .items
            .iter()
            .enumerate()
            .map(|(i, _)| format!("{prefix}-item-{i}"))
            .collect(),
        master_items: template
            .master
            .iter()
            .flat_map(|m| m.items.iter())
            .enumerate()
            .map(|(i, _)| format!("{prefix}-master-item-{i}"))
            .collect(),
    }
}

pub fn from_template(
    template: &Template,
    ids: &InstanceIds,
    options: &InstanceOptions,
    index: Option<u32>,
) -> Result<Created> {
    template::validate_template(template, template.meta.id.starts_with("sys:"))?;
    let mut answered = std::collections::BTreeSet::new();
    for (i, answer) in options.values.iter().enumerate() {
        let name = crate::expr::fold(&answer.name);
        if !template
            .variables
            .iter()
            .any(|v| crate::expr::fold(&v.name) == name)
        {
            return Err(crate::SheetError::at(
                "unknown_variable",
                format!("options.values[{i}].name"),
                format!(
                    "“{}” şablon sorusu yok; şablonun variables listesindeki adı kullanın.",
                    answer.name
                ),
            ));
        }
        if !answered.insert(name) {
            return Err(crate::SheetError::at(
                "duplicate_variable",
                format!("options.values[{i}].name"),
                "Aynı şablon sorusuna iki kez cevap verilemez.",
            ));
        }
    }
    let instance = template::instantiate(template, ids, options)?;
    let sheet = instance.sheet.id.clone();
    let mut ops = Vec::new();
    if !instance.assets.is_empty() {
        ops.push(Op::AddAssets(AddAssets {
            assets: instance.assets,
        }));
    }
    if let Some(master) = instance.master {
        ops.push(Op::AddMaster(AddMaster {
            master,
            index: None,
        }));
    }
    ops.push(Op::AddSheet(AddSheet {
        sheet: instance.sheet,
        index,
    }));
    Ok(Created {
        sheet,
        ops,
        pictures: instance.asset_bytes,
    })
}
