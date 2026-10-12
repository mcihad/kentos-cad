//! Schema 42's address data (docs/specs/kcad-v2.md §6.4.10, docs/adr/0229
//! §2): the settings' `addressLocators`, read field by field and then
//! checked whole by the contract's rules (`address_locators_problem`), as
//! the writers check them; an empty list, an empty area list and a field's
//! empty name are refused (a writer leaves them out).

use kentos_contracts::{
    AddressLocator, AreaLevel, AreaSource, DoorSource, StreetSource, address_locators_problem,
};

use super::{list, map, named, required, text, unknown};
use crate::cbor::Reader;
use crate::error::{Code, KcadError};

/// A field's name: a text that is not empty.
fn field(r: &mut Reader<'_>) -> Result<String, KcadError> {
    let at = r.position();
    let t = text(r)?;
    if t.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş alan adı yazılmaz"));
    }
    Ok(t)
}

fn doors(r: &mut Reader<'_>) -> Result<DoorSource, KcadError> {
    let (mut layer, mut street, mut door) = (None, None, None);
    let (mut neighbourhood, mut district, mut province) = (None, None, None);
    let (mut postcode, mut building, mut code) = (None, None, None);
    map(r, |r, key| {
        match key {
            "layer" => layer = Some(text(r)?),
            "street" => street = Some(field(r)?),
            "door" => door = Some(field(r)?),
            "neighbourhood" => neighbourhood = Some(field(r)?),
            "district" => district = Some(field(r)?),
            "province" => province = Some(field(r)?),
            "postcode" => postcode = Some(field(r)?),
            "building" => building = Some(field(r)?),
            "code" => code = Some(field(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(DoorSource {
        layer: required(r, layer, "layer")?,
        street: required(r, street, "street")?,
        door: required(r, door, "door")?,
        neighbourhood,
        district,
        province,
        postcode,
        building,
        code,
    })
}

fn streets(r: &mut Reader<'_>) -> Result<StreetSource, KcadError> {
    let (mut layer, mut name, mut neighbourhood, mut district) = (None, None, None, None);
    map(r, |r, key| {
        match key {
            "layer" => layer = Some(text(r)?),
            "name" => name = Some(field(r)?),
            "neighbourhood" => neighbourhood = Some(field(r)?),
            "district" => district = Some(field(r)?),
            _ => return Err(unknown(r)),
        }
        Ok(())
    })?;
    Ok(StreetSource {
        layer: required(r, layer, "layer")?,
        name: required(r, name, "name")?,
        neighbourhood,
        district,
    })
}

fn areas(r: &mut Reader<'_>) -> Result<Vec<AreaSource>, KcadError> {
    let at = r.position();
    let levels = [
        ("neighbourhood", AreaLevel::Neighbourhood),
        ("district", AreaLevel::District),
        ("province", AreaLevel::Province),
    ];
    let all = list(r, |r, _| {
        let (mut layer, mut level, mut name, mut district) = (None, None, None, None);
        map(r, |r, key| {
            match key {
                "layer" => layer = Some(text(r)?),
                "level" => level = Some(named(r, &levels)?),
                "name" => name = Some(field(r)?),
                "district" => district = Some(field(r)?),
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(AreaSource {
            layer: required(r, layer, "layer")?,
            level: required(r, level, "level")?,
            name: required(r, name, "name")?,
            district,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş bölge listesi yazılmaz"));
    }
    Ok(all)
}

/// The project's address locators, checked whole; an empty list is not written.
pub(super) fn address_locators(r: &mut Reader<'_>) -> Result<Vec<AddressLocator>, KcadError> {
    let at = r.position();
    let all = list(r, |r, _| {
        let (mut id, mut name, mut min_score, mut reach) = (None, None, None, None);
        let (mut door_source, mut street_source, mut area_sources) = (None, None, Vec::new());
        map(r, |r, key| {
            match key {
                "id" => id = Some(text(r)?),
                "name" => name = Some(text(r)?),
                "minScore" => min_score = Some(r.uint(u64::from(u32::MAX))? as u32),
                "reach" => reach = Some(r.float()?),
                "doors" => door_source = Some(doors(r)?),
                "streets" => street_source = Some(streets(r)?),
                "areas" => area_sources = areas(r)?,
                _ => return Err(unknown(r)),
            }
            Ok(())
        })?;
        Ok(AddressLocator {
            id: required(r, id, "id")?,
            name: required(r, name, "name")?,
            doors: door_source,
            streets: street_source,
            areas: area_sources,
            min_score: required(r, min_score, "minScore")?,
            reach: required(r, reach, "reach")?,
        })
    })?;
    if all.is_empty() {
        return Err(r.fail_at(Code::BadValue, at, "boş adres verisi listesi yazılmaz"));
    }
    match address_locators_problem(&all) {
        Some(problem) => Err(r.fail_at(Code::BadValue, at, &problem)),
        None => Ok(all),
    }
}
