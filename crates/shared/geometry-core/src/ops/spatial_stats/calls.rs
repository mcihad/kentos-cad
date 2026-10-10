//! The web's calls (`model/ops/spatialStats.ts`): each tool's run by name,
//! objects as the drawing holds them (only their geometry is read), values
//! and weights as attribute texts. The desktop's tools call the functions
//! themselves.

use super::autocorrelation::{Neighbourhood, hot_spots, morans_i};
use super::centers::{CenterInput, CenterKind, centers};
use super::clusters::{dbscan, k_means};
use super::nearest::nearest;
use super::weights::Concept;
use super::{NewObject, Number, ObjectCopy, StatsRun, Table};
use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::op;

crate::json_struct!(out NewObject { shape, attrs });
crate::json_struct!(out ObjectCopy { index, attrs, color });
crate::json_struct!(out Table { columns, rows });
crate::json_struct!(out Number { name, value });
crate::json_struct!(out StatsRun { summary, infos, warnings, table, numbers, objects, copies });

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
