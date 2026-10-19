//! The web's calls (`model/ops/spatialStats.ts`): each tool's run by name,
//! objects as the drawing holds them (only their geometry is read), values
//! and weights as attribute texts. The desktop's tools call the functions
//! themselves.

use super::autocorrelation::{Neighbourhood, hot_spots, morans_i, morans_i_with};
use super::centers::{CenterInput, CenterKind, centers};
use super::clusters::{dbscan, k_means};
use super::inference::{Correction, Inference, Profile};
use super::local::{hot_spots_with, local_morans};
use super::nearest::nearest;
use super::weights::Concept;
use super::{NewObject, Number, ObjectCopy, StatsRun, Table};
use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::op;
use crate::ops::sampling::random::seed_of;

crate::json_struct!(out NewObject { shape, attrs });
crate::json_struct!(out ObjectCopy { index, attrs, color });
crate::json_struct!(out Table { columns, rows });
crate::json_struct!(out Number { name, value });
crate::json_struct!(out Profile { keys, labels, values, inputs });
crate::json_struct!(out StatsRun { summary, infos, warnings, table, numbers, objects, copies, profile });

fn shapes(entities: &[Entity]) -> Vec<Shape> {
    entities.iter().map(|e| e.shape.clone()).collect()
}

fn hood(concept: &str, band: Option<f64>, k: usize) -> Result<Neighbourhood, String> {
    Ok(Neighbourhood {
        concept: Concept::from_key(concept).ok_or("bilinmeyen komşuluk")?,
        band,
        k,
    })
}

/// The inference a call asks for (docs/adr/0520): permutations (0: the normal distribution's p), the seed as the
/// sampling tools read it, the correction by key.
fn inference(permutations: usize, seed: f64, correction: &str) -> Result<Inference, String> {
    Ok(Inference {
        permutations,
        seed: seed_of(seed),
        correction: Correction::from_key(correction).ok_or("bilinmeyen düzeltme")?,
    })
}

pub static OPS: &[Op] = &[
    op!("statsCenters", |entities: Vec<Entity>,
                         kind: String,
                         weights: Option<Vec<Option<String>>>,
                         groups: Option<Vec<Option<String>>>,
                         weight_field: String,
                         k: f64| {
        let kind = CenterKind::from_key(&kind).ok_or("bilinmeyen merkez")?;
        let input = CenterInput {
            weights: weights.as_deref(),
            groups: groups.as_deref(),
            weight_field: &weight_field,
            k,
        };
        centers(&shapes(&entities), kind, &input)
    }),
    op!("statsNearest", |entities: Vec<Entity>,
                         area: Option<f64>| {
        nearest(&shapes(&entities), area)
    }),
    op!("statsMoran", |entities: Vec<Entity>,
                       values: Vec<Option<String>>,
                       field: String,
                       concept: String,
                       band: Option<f64>,
                       k: usize,
                       standardize: bool| {
        let hood = hood(&concept, band, k)?;
        morans_i(&shapes(&entities), &values, &field, hood, standardize)
    }),
    op!("statsHotSpots", |entities: Vec<Entity>,
                          values: Vec<Option<String>>,
                          field: String,
                          concept: String,
                          band: Option<f64>,
                          k: usize| {
        let hood = hood(&concept, band, k)?;
        hot_spots(&shapes(&entities), &values, &field, hood)
    }),
    op!("statsMoranWith", |entities: Vec<Entity>,
                           values: Vec<Option<String>>,
                           field: String,
                           concept: String,
                           band: Option<f64>,
                           k: usize,
                           standardize: bool,
                           permutations: usize,
                           seed: f64| {
        let hood = hood(&concept, band, k)?;
        let inf = inference(permutations, seed, "none")?;
        morans_i_with(&shapes(&entities), &values, &field, hood, standardize, inf)
    }),
    op!(
        "statsHotSpotsWith",
        |entities: Vec<Entity>,
         values: Vec<Option<String>>,
         field: String,
         concept: String,
         band: Option<f64>,
         k: usize,
         permutations: usize,
         seed: f64,
         correction: String| {
            let hood = hood(&concept, band, k)?;
            let inf = inference(permutations, seed, &correction)?;
            hot_spots_with(&shapes(&entities), &values, &field, hood, inf)
        }
    ),
    op!("statsLocalMoran", |entities: Vec<Entity>,
                            values: Vec<Option<String>>,
                            field: String,
                            concept: String,
                            band: Option<f64>,
                            k: usize,
                            standardize: bool,
                            permutations: usize,
                            seed: f64,
                            correction: String,
                            alpha: f64| {
        let hood = hood(&concept, band, k)?;
        let inf = inference(permutations, seed, &correction)?;
        local_morans(
            &shapes(&entities),
            &values,
            &field,
            hood,
            standardize,
            inf,
            alpha,
        )
    }),
    op!("statsDbscan", |entities: Vec<Entity>,
                        radius: f64,
                        min_points: usize,
                        border_noise: bool| {
        dbscan(&shapes(&entities), radius, min_points, border_noise)
    }),
    op!("statsKMeans", |entities: Vec<Entity>, k: usize| {
        k_means(&shapes(&entities), k)
    }),
];
