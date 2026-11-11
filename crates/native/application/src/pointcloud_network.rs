//! `cad.pointclouds.network` v1 (plan/map-cad.md MAP-13, docs/adr/4720): Tarama ağı dengelemesi, read only. Each link
//! is registered as `cad.pointclouds.register` registers two scans (its first held, its second moving; the input's
//! options), and its answer becomes an observation of the second's correction relative to the first: the rotation
//! (ω, φ, κ) and the translation moved to the network's centre (the scans' boxes' middles' mean), weighed by the
//! registration's last normal matrix moved there too (`kentos_pointcloud::ops::network::moved`). The free scans'
//! corrections are adjusted together by least squares (`ops::network::adjust`); a link's shift is the square root of
//! rᵀ·N·r over its pairs (how far, along the pairs' normals, the network moves it from its own registration). Links not
//! given are every pair (in the scans' order) whose plan boxes overlap. Nothing is written.
//! The shared cases are `fixtures/commands/v1/cad.pointclouds.network.json`.

use kentos_commands::{
    CommandResult, CommandWarning, NetworkLink, NetworkLoop, NetworkScan, PointCloudsNetwork, PointCloudsNetworked,
    PointCloudsRegister,
};
use kentos_domain::Document;
use kentos_pointcloud::ops::network::{self, NetworkFault, Observation};
use kentos_pointcloud::ops::transform::Affine;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::pointcloud_register::registered_from;
use crate::pointclouds::clouds;

/// Fewer pairs than this and a registration says nothing about six unknowns.
const LEAST_PAIRS: u32 = 6;

fn fail(message: String, path: &str) -> Stop {
    Stop::failed(error(codes::INVALID_INPUT, message, Some(path.to_owned())))
}

fn run(doc: &Document, input: &PointCloudsNetwork) -> Result<(PointCloudsNetworked, Vec<CommandWarning>), Stop> {
    let uids: Vec<String> = input.uids.iter().map(|u| u.trim().to_owned()).collect();
    if uids.len() < 3 {
        return Err(fail(
            "Tarama ağı en az üç taramayla kurulur; iki tarama için Tarama kaydı'nı kullanın.".into(),
            "uids",
        ));
    }
    if let Some(k) = (1..uids.len()).find(|&k| uids[..k].contains(&uids[k])) {
        return Err(fail(format!("“{}” taraması iki kez verildi.", uids[k]), &format!("uids[{k}]")));
    }
    checks::uids(&uids, "")?;
    let scans = clouds(doc, &uids)?;
    let fixed_ids: Vec<String> = input.fixed.iter().map(|u| u.trim().to_owned()).collect();
    if fixed_ids.is_empty() {
        return Err(fail("En az bir taramayı sabit verin: ağın konumu sabit taramalarındır.".into(), "fixed"));
    }
    if let Some((k, f)) = fixed_ids.iter().enumerate().find(|(_, f)| !uids.contains(f)) {
        return Err(fail(format!("Sabit tarama “{f}” ağın taramaları arasında yok."), &format!("fixed[{k}]")));
    }
    let fixed: Vec<bool> = uids.iter().map(|u| fixed_ids.contains(u)).collect();
    if fixed.iter().all(|f| *f) {
        return Err(fail("Bütün taramalar sabit: dengelenecek tarama kalmadı.".into(), "fixed"));
    }
    let at = |u: &str| uids.iter().position(|v| v == u);
    // The links: given, or every pair whose plan boxes overlap.
    let links: Vec<(usize, usize)> = match &input.links {
        Some(given) => {
            let mut out: Vec<(usize, usize)> = Vec::with_capacity(given.len());
            for (k, [a, b]) in given.iter().enumerate() {
                let (Some(i), Some(j)) = (at(a.trim()), at(b.trim())) else {
                    return Err(fail(format!("{}. bağın taraması ağın taramaları arasında yok.", k + 1), &format!("links[{k}]")));
                };
                if i == j {
                    return Err(fail(format!("{}. bağ bir taramayı kendine bağlıyor.", k + 1), &format!("links[{k}]")));
                }
                if out.iter().any(|&(p, q)| (p, q) == (i, j) || (p, q) == (j, i)) {
                    return Err(fail(format!("{}. bağ iki kez verildi.", k + 1), &format!("links[{k}]")));
                }
                out.push((i, j));
            }
            out
        }
        None => {
            let b = |k: usize| scans[k].1.cloud.bounds;
            let mut out = Vec::new();
            for i in 0..scans.len() {
                for j in i + 1..scans.len() {
                    let (p, q) = (b(i), b(j));
                    if p[0].max(q[0]) < p[3].min(q[3]) && p[1].max(q[1]) < p[4].min(q[4]) {
                        out.push((i, j));
                    }
                }
            }
            out
        }
    };
    // The network's centre: the scans' boxes' middles' mean.
    let n = scans.len() as f64;
    let centre: [f64; 3] = std::array::from_fn(|i| {
        scans.iter().map(|(_, c)| (c.cloud.bounds[i] + c.cloud.bounds[i + 3]) / 2.0).sum::<f64>() / n
    });
    // docs/adr/5100: the outer loop's rounds; one, as before, unless asked.
    let most = input.rounds.unwrap_or(1);
    if !(1..=MOST_ROUNDS).contains(&most) {
        return Err(fail(format!("Tur sayısı 1 ile {MOST_ROUNDS} arasında olmalı."), "rounds"));
    }
    let stop = input.tolerance.unwrap_or(0.0001);
    // Each scan's map so far (none: where it is), and each round's largest move of a corner of a scan's box.
    let mut totals: Vec<Option<Affine>> = vec![None; uids.len()];
    let mut moves: Vec<f64> = Vec::new();
    let (observations, answers, warnings, adjusted) = loop {
        let (observations, answers, warnings) = round(doc, input, &uids, &links, centre, &totals)?;
        let adjusted = network::adjust(uids.len(), &fixed, &observations).map_err(|f| match f {
            NetworkFault::Unjoined(k) => fail(
                format!("“{}” taraması bağlarla hiçbir sabit taramaya bağlanmıyor; bir bağ ekleyin ya da onu da sabit yapın.", uids[k]),
                "links",
            ),
            NetworkFault::Undetermined => fail(
                "Bağlar ağı belirlemiyor: bir taramanın konumunun bir yönü serbest kalıyor (düz zeminde yatay öteleme gibi). Eğimli yüzey ya da yapı gören bir bağ ekleyin ya da o taramayı sabit yapın.".into(),
                "links",
            ),
        })?;
        if most == 1 {
            break (observations, answers, warnings, adjusted);
        }
        // The round's maps taken after the scans' maps so far; the largest move of a corner of a scan's box.
        let mut largest: f64 = 0.0;
        for k in (0..uids.len()).filter(|&k| !fixed[k]) {
            let Ok(m) = Affine::new(network::matrix(adjusted.x[k], centre)) else { continue };
            let b = scans[k].1.cloud.bounds;
            for c in 0..8 {
                let p0 = [b[if c & 1 == 0 { 0 } else { 3 }], b[if c & 2 == 0 { 1 } else { 4 }], b[if c & 4 == 0 { 2 } else { 5 }]];
                let p = totals[k].as_ref().map_or(p0, |t| t.apply(p0));
                largest = largest.max(m.shift(p));
            }
            totals[k] = Some(totals[k].as_ref().map_or(m, |t| t.then(&m)));
        }
        moves.push(largest);
        if largest < stop || moves.len() as u32 >= most {
            break (observations, answers, warnings, adjusted);
        }
    };
    let scans_out: Vec<NetworkScan> = uids
        .iter()
        .enumerate()
        .map(|(k, u)| {
            let x = adjusted.x[k];
            let (rotation, translation, matrix) = match &totals[k] {
                // docs/adr/5100: after rounds, the maps so far about the centre: their turns (R = Rz(κ)·Ry(φ)·Rx(ω))
                // and how far they take the centre.
                Some(t) if most > 1 => {
                    let m = t.matrix();
                    let w = [m[9].atan2(m[10]), -m[8].clamp(-1.0, 1.0).asin(), m[4].atan2(m[0])];
                    let c = t.apply(centre);
                    (w, [c[0] - centre[0], c[1] - centre[1], c[2] - centre[2]], m)
                }
                _ => ([x[0], x[1], x[2]], [x[3], x[4], x[5]], network::matrix(x, centre)),
            };
            NetworkScan { uid: u.clone(), fixed: fixed[k], rotation, translation, matrix, horizontal: adjusted.horizontal[k] }
        })
        .collect();
    let (mut sum, mut pairs) = (0.0, 0_u64);
    let links_out: Vec<NetworkLink> = links
        .iter()
        .enumerate()
        .map(|(l, &(i, j))| {
            let (o, out, r) = (&observations[l], &answers[l], adjusted.residuals[l]);
            let q = network::quadratic(&o.n, r);
            sum += q;
            pairs += u64::from(out.pairs);
            NetworkLink {
                scans: [uids[i].clone(), uids[j].clone()],
                pairs: out.pairs,
                overlap: out.overlap,
                rms_after: out.rms_after,
                strength: out.strength,
                observed: o.m,
                residual: r,
                shift: (q.max(0.0) / f64::from(out.pairs)).sqrt(),
            }
        })
        .collect();
    let loops: Vec<NetworkLoop> = adjusted
        .loops
        .iter()
        .map(|lp| NetworkLoop {
            scans: lp.path.iter().map(|&k| uids[k].clone()).collect(),
            rotation: (lp.w[0] * lp.w[0] + lp.w[1] * lp.w[1] + lp.w[2] * lp.w[2]).sqrt(),
            translation: (lp.w[3] * lp.w[3] + lp.w[4] * lp.w[4] + lp.w[5] * lp.w[5]).sqrt(),
        })
        .collect();
    let shift = if pairs > 0 { (sum.max(0.0) / pairs as f64).sqrt() } else { 0.0 };
    let held = fixed.iter().filter(|f| **f).count();
    let mut notes = vec![
        format!(
            "Tarama ağı: {} tarama ({} sabit), {} bağ, {} döngü; ağın RMS kayması {:.1} mm.",
            uids.len(),
            held,
            links.len(),
            loops.len(),
            shift * 1000.0
        ),
        "Her bağ Tarama kaydı gibi kaydedildi ve son normal matrisiyle ağırlıklandı; düzeltmeler ağın merkezi çevresinde birinci derecedendir (küçük açılar).".to_owned(),
    ];
    if !loops.is_empty() {
        notes.push("Döngünün kapanmaması bağların kayıtlarının birbirini ne kadar tutmadığını söyler; dengeleme onu bağlara ağırlıklarıyla dağıtır.".to_owned());
    }
    for (l, out) in answers.iter().enumerate() {
        if out.horizontal < 0.1 {
            let (i, j) = links[l];
            notes.push(format!(
                "“{}” ile “{}” bağında geometri yatay ötelemeyi zayıf tutuyor: bu bağ ağda düşeyde anlamlı, yatayda az.",
                uids[i], uids[j]
            ));
        }
    }
    for (k, u) in uids.iter().enumerate() {
        if let Some(h) = adjusted.horizontal[k]
            && h < 0.1
        {
            notes.push(format!(
                "“{u}” taramasının yatay konumunu ağ zayıf belirliyor (oran {h:.2}): düz zeminde yatay öteleme ölçülmez; düzeltmesi düşeyde anlamlıdır."
            ));
        }
    }
    // docs/adr/5100: the rounds made and what the last one moved.
    if let Some(last) = moves.last() {
        notes.push(format!(
            "Dış döngü: {} tur; bağlar her turda taramaların bir önceki turdaki yerinden yeniden kaydedildi, son turda bir taramanın kutusunun bir köşesi en çok {:.2} mm kaydı{}. Bağlar, artıklar ve döngüler son turunkidir; matrisler turların toplamıdır.",
            moves.len(),
            last * 1000.0,
            if *last < stop { "" } else { " (durma toleransının üstünde)" }
        ));
    }
    notes.push("Ağın mutlak konumu sabit taramalarındır; kontrol noktalarıyla Tarama kaydı'nda ayrıca denetlenir. Taramalar yerinde durur; her birini Nesnede tut nesnede konumlar, Dönüşüm uygula yeni dosyaya taşır.".to_owned());
    let rounds = (most > 1).then_some(moves);
    Ok((PointCloudsNetworked { centre, scans: scans_out, links: links_out, loops, shift, notes, rounds }, warnings))
}

/// The most rounds of the outer loop (docs/adr/5100).
const MOST_ROUNDS: u32 = 10;

/// One round (docs/adr/5100): each link registered where the scans' maps so far put them, its answer moved to the
/// network's centre and weighed by its normal matrix moved there.
#[allow(clippy::type_complexity)]
fn round(
    doc: &Document,
    input: &PointCloudsNetwork,
    uids: &[String],
    links: &[(usize, usize)],
    centre: [f64; 3],
    totals: &[Option<Affine>],
) -> Result<(Vec<Observation>, Vec<kentos_commands::PointCloudsRegistered>, Vec<CommandWarning>), Stop> {
    let mut observations = Vec::with_capacity(links.len());
    let mut answers = Vec::with_capacity(links.len());
    let mut warnings = Vec::new();
    for &(i, j) in links {
        let pair = PointCloudsRegister {
            uids: vec![uids[i].clone(), uids[j].clone()],
            spacing: input.spacing,
            reach: input.reach,
            iterations: input.iterations,
            tolerance: input.tolerance,
            neighbours: input.neighbours,
            robust: input.robust.clone(),
            reject: input.reject.clone(),
            drawing: input.drawing.clone(),
            ..Default::default()
        };
        let (out, more, normal) = registered_from(doc, &pair, [totals[i].as_ref(), totals[j].as_ref()])?;
        if out.pairs < LEAST_PAIRS {
            return Err(fail(
                format!(
                    "“{}” ile “{}” taramaları arasında {} eşleşme var; bir bağ için en az {LEAST_PAIRS} gerekir. Erişimi büyütün ya da bu bağı çıkarın.",
                    uids[i], uids[j], out.pairs
                ),
                "links",
            ));
        }
        let m = out.matrix;
        let r = [[m[0], m[1], m[2]], [m[4], m[5], m[6]], [m[8], m[9], m[10]]];
        let (obs, weight) = network::moved(out.rotation, r, out.translation, &normal, out.centre, centre);
        observations.push(Observation { a: i, b: j, m: obs, n: weight });
        answers.push(out);
        warnings.extend(more);
    }
    Ok((observations, answers, warnings))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsNetwork) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &PointCloudsNetwork) -> CommandResult<PointCloudsNetworked> {
    match run(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PointCloudsNetwork) -> CommandResult<PointCloudsNetworked> {
    plan(cx, &input)
}
