//! `cad.readjustment.distribute` v1 (plan/map-cad.md MAP-11, docs/adr/1460): Düzenleme dağıtım taslağı, read only.
//! The numbers are the shared core's exact ones (`kentos_geometry_core::numeric`): decimal text, exact shares, rounding
//! and remainder only by the rule package's policy. The deduction: its exact total (Σ area × ratio) rounded once, then
//! split over the parcels by their areas; each parcel's net split over its holders by their shares; every remainder's
//! receivers said. The totals are checked exactly. The answer is a draft, never a decision. The shared cases are
//! `fixtures/commands/v1/cad.readjustment.distribute.json`.

use kentos_commands::{
    CommandResult, ReadjustmentDistribute, ReadjustmentDistributed, ReadjustmentHolder, ReadjustmentParcel,
    ReadjustmentPart,
};
use kentos_geometry_core::numeric::{
    NumericError, RemainderRule, RoundingMode, Share, distribute, distribute_shares, multiply_exact, parse_decimal,
    round_decimal, subtract_exact, sum_exact, sum_shares,
};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;

/// The most places a policy may ask for here.
const MOST_PLACES: u32 = 6;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn numeric(e: NumericError, path: &str) -> Stop {
    match e {
        NumericError::NeedsRule(r) => fail(
            codes::NEEDS_RULE,
            format!("Yuvarlanan değerler toplamı {r} kadar tutmuyor ve kural paketinin artık kuralı “reject”; hiçbir şey dağıtılmadı. Artık kuralı onaylı bir pakette verin."),
            path,
        ),
        other => fail(codes::INVALID_INPUT, format!("Sayı hesaplanamadı ({}).", other.code()), path),
    }
}

/// Places after the point in plain decimal text.
fn places(text: &str) -> u32 {
    text.split_once('.').map_or(0, |(_, f)| f.len() as u32)
}

/// `text` written with exactly `scale` places (it has at most that many: padding only).
fn fixed(text: &str, scale: u32) -> Result<String, NumericError> {
    round_decimal(text, scale, RoundingMode::HalfEven)
}

/// net × share as a reduced fraction `n/d` (square metres).
fn exact_part(net: &str, share: Share) -> Result<String, NumericError> {
    let d = parse_decimal(net)?;
    let mut num = d.mantissa().checked_mul(i128::try_from(share.numerator).map_err(|_| NumericError::Overflow)?).ok_or(NumericError::Overflow)?;
    let mut den = 10i128
        .checked_pow(d.scale())
        .and_then(|p| p.checked_mul(i128::try_from(share.denominator).ok()?))
        .ok_or(NumericError::Overflow)?;
    let (mut a, mut b) = (num.unsigned_abs(), den.unsigned_abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let g = a.max(1) as i128;
    num /= g;
    den /= g;
    Ok(format!("{num}/{den}"))
}

fn run(input: &ReadjustmentDistribute) -> Result<ReadjustmentDistributed, Stop> {
    let r = &input.rules;
    for (v, path, what) in [(&r.id, "rules.id", "kimliği"), (&r.source, "rules.source", "kaynak belgesi"), (&r.jurisdiction, "rules.jurisdiction", "kurumu")] {
        if v.trim().is_empty() {
            return Err(fail(codes::INVALID_INPUT, format!("Kural paketinin {what} boş; kuralın dayanağı yazılmadan dağıtım yapılmaz."), path));
        }
    }
    let one = parse_decimal("1").ok();
    let ratio = parse_decimal(r.deduction.trim()).ok().filter(|d| !d.is_sign_negative() && Some(*d) < one);
    let Some(_) = ratio else {
        return Err(fail(codes::INVALID_INPUT, format!("Kesinti oranı “{}” 0 ile 1 arasında (1 hariç) bir ondalık yazı olmalı.", r.deduction), "rules.deduction"));
    };
    if r.scale > MOST_PLACES {
        return Err(fail(codes::INVALID_INPUT, format!("Basamak {} en çok {MOST_PLACES} olabilir.", r.scale), "rules.scale"));
    }
    let Some(mode) = RoundingMode::parse(r.rounding.trim()) else {
        return Err(fail(codes::INVALID_INPUT, format!("“{}” bir yuvarlama yönü değil (half_even, half_away_from_zero …).", r.rounding), "rules.rounding"));
    };
    let Some(rule) = RemainderRule::parse(r.remainder.trim()) else {
        return Err(fail(codes::INVALID_INPUT, format!("“{}” bir artık kuralı değil (reject, largest_remainder).", r.remainder), "rules.remainder"));
    };
    if input.entries.is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Kayıt yok: parsel, hak sahibi, tapu alanı ve pay verin.".into(), "entries"));
    }
    let scale = r.scale;
    // The parcels in their first record's order: (parcel, area, [(record, holder, share)]).
    let mut parcels: Vec<(String, String, Vec<(usize, String, Share)>)> = Vec::new();
    for (i, e) in input.entries.iter().enumerate() {
        let (parcel, holder, area) = (e.parcel.trim(), e.holder.trim(), e.area.trim());
        if parcel.is_empty() || holder.is_empty() {
            let path = if parcel.is_empty() { format!("entries[{i}].parcel") } else { format!("entries[{i}].holder") };
            return Err(fail(codes::INVALID_INPUT, format!("{}. kaydın parseli ya da hak sahibi boş.", i + 1), &path));
        }
        let ok = parse_decimal(area).ok().filter(|d| !d.is_sign_negative());
        if ok.is_none() || places(area) > scale {
            return Err(fail(
                codes::INVALID_INPUT,
                format!("{}. kaydın tapu alanı “{}” eksi olmayan, en çok {scale} basamaklı düz bir ondalık yazı olmalı.", i + 1, e.area),
                &format!("entries[{i}].area"),
            ));
        }
        let Ok(share) = Share::parse(e.share.trim()) else {
            return Err(fail(codes::INVALID_INPUT, format!("{}. kaydın payı “{}” n/d biçiminde değil (payda sıfır olamaz).", i + 1, e.share), &format!("entries[{i}].share")));
        };
        match parcels.iter_mut().find(|(p, _, _)| p == parcel) {
            Some((_, first, holders)) => {
                if parse_decimal(first).ok() != parse_decimal(area).ok() {
                    return Err(fail(
                        codes::AREA_MISMATCH,
                        format!("“{parcel}” parselinin tapu alanı kayıtlarda farklı: {first} ve {area}."),
                        &format!("entries[{i}].area"),
                    ));
                }
                if holders.iter().any(|(_, h, _)| h == holder) {
                    return Err(fail(
                        codes::DUPLICATE_HOLDER,
                        format!("“{holder}” “{parcel}” parselinde iki kez var; payları tek kayıtta toplayın."),
                        &format!("entries[{i}].holder"),
                    ));
                }
                holders.push((i, holder.to_owned(), share));
            }
            None => parcels.push((parcel.to_owned(), area.to_owned(), vec![(i, holder.to_owned(), share)])),
        }
    }
    for (parcel, _, holders) in &parcels {
        let shares: Vec<Share> = holders.iter().map(|(_, _, s)| *s).collect();
        let sum = sum_shares(&shares).map_err(|e| numeric(e, "entries"))?;
        if !sum.is_whole() {
            let first = holders[0].0;
            return Err(fail(
                codes::SHARES_NOT_WHOLE,
                format!("“{parcel}” parselinin payları toplamı {sum}, tam 1 değil."),
                &format!("entries[{first}].share"),
            ));
        }
    }
    // The deduction: its exact total rounded once, split over the parcels by their areas.
    let areas: Vec<&str> = parcels.iter().map(|(_, a, _)| a.as_str()).collect();
    let products: Vec<String> =
        areas.iter().map(|a| multiply_exact(a, r.deduction.trim())).collect::<Result<_, _>>().map_err(|e| numeric(e, "rules.deduction"))?;
    let product_refs: Vec<&str> = products.iter().map(String::as_str).collect();
    let exact = sum_exact(&product_refs).map_err(|e| numeric(e, "rules.deduction"))?;
    let total = round_decimal(&exact, scale, mode).map_err(|e| numeric(e, "rules.deduction"))?;
    let registered = sum_exact(&areas).map_err(|e| numeric(e, "entries"))?;
    let split = if parse_decimal(&registered).is_ok_and(|d| d.is_zero()) {
        kentos_geometry_core::numeric::Distribution { parts: vec![fixed("0", scale).unwrap_or_default(); areas.len()], remainder: "0".into(), given: Vec::new() }
    } else {
        distribute(&total, &areas, scale, mode, rule).map_err(|e| numeric(e, "rules.deduction"))?
    };
    let mut out_parcels = Vec::new();
    let mut nets = Vec::new();
    let mut held_parts = Vec::new();
    let mut holders: Vec<(String, Vec<String>)> = Vec::new();
    for ((parcel, area, hs), deduction) in parcels.iter().zip(&split.parts) {
        let net = fixed(&subtract_exact(area, deduction).map_err(|e| numeric(e, "entries"))?, scale).map_err(|e| numeric(e, "entries"))?;
        let shares: Vec<Share> = hs.iter().map(|(_, _, s)| *s).collect();
        let d = distribute_shares(&net, &shares, scale, mode, rule).map_err(|e| numeric(e, &format!("entries[{}].share", hs[0].0)))?;
        let mut parts = Vec::new();
        for ((_, holder, share), value) in hs.iter().zip(&d.parts) {
            parts.push(ReadjustmentPart {
                holder: holder.clone(),
                share: share.to_string(),
                exact: exact_part(&net, *share).map_err(|e| numeric(e, "entries"))?,
                value: value.clone(),
            });
            held_parts.push(value.clone());
            match holders.iter_mut().find(|(h, _)| h == holder) {
                Some((_, v)) => v.push(value.clone()),
                None => holders.push((holder.clone(), vec![value.clone()])),
            }
        }
        out_parcels.push(ReadjustmentParcel {
            parcel: parcel.clone(),
            area: area.clone(),
            deduction: deduction.clone(),
            net: net.clone(),
            remainder: d.remainder,
            remainder_to: d.given.iter().map(|&k| hs[k].1.clone()).collect(),
            parts,
        });
        nets.push(net);
    }
    let sum = |v: &[String]| -> Result<String, Stop> {
        let refs: Vec<&str> = v.iter().map(String::as_str).collect();
        fixed(&sum_exact(&refs).map_err(|e| numeric(e, "entries"))?, scale).map_err(|e| numeric(e, "entries"))
    };
    let net = sum(&nets)?;
    let held = sum(&held_parts)?;
    let registered_fixed = fixed(&registered, scale).map_err(|e| numeric(e, "entries"))?;
    let deducted = sum(&split.parts)?;
    let back = sum(&[deducted.clone(), net.clone()])?;
    let conserved = back == registered_fixed && held == net && deducted == total;
    Ok(ReadjustmentDistributed {
        rules: r.id.trim().to_owned(),
        release: r.release,
        draft: true,
        registered: registered_fixed,
        deduction_exact: exact,
        deduction: total,
        deduction_remainder: split.remainder,
        deduction_remainder_to: split.given.iter().map(|&k| parcels[k].0.clone()).collect(),
        net,
        held,
        conserved,
        parcels: out_parcels,
        holders: holders
            .into_iter()
            .map(|(holder, values)| Ok(ReadjustmentHolder { holder, value: sum(&values)? }))
            .collect::<Result<_, Stop>>()?,
    })
}

pub fn validate(_cx: &ExecutionContext<'_>, input: &ReadjustmentDistribute) -> CommandResult<()> {
    match run(input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The draft; reads only.
pub fn plan(_cx: &ExecutionContext<'_>, input: &ReadjustmentDistribute) -> CommandResult<ReadjustmentDistributed> {
    match run(input) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The draft; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ReadjustmentDistribute) -> CommandResult<ReadjustmentDistributed> {
    plan(cx, &input)
}
