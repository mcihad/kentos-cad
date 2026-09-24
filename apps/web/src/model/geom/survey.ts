import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * Surveying point constructions ("Koordinat hesap makinası"), in the
 * conventions a Turkish surveyor reads off field notes: abscissa along the
 * line A→B, ordinate square to it and positive to the right; horizontal
 * angles clockwise from the reference direction. Computed by the geometry
 * core (docs/adr/0008).
 */

/** Yan nokta (dik ayak / dik boy): `absis` along A→B from A, `ordinat` square to it (+ right). */
export const sidePoint = op<(a: Vec2, b: Vec2, absis: number, ordinat: number) => Vec2 | null>('sidePoint');

/** Absis and ordinat (+ right) of p relative to the line A→B. */
export const sideOffsets = op<(a: Vec2, b: Vec2, p: Vec2) => { absis: number; ordinat: number } | null>('sideOffsets');

/**
 * Kenar kesişimi: points at distance d1 from A and d2 from B. Two
 * solutions, the one right of A→B first; one when the circles touch.
 */
export const distanceIntersection = op<(a: Vec2, b: Vec2, d1: number, d2: number) => Vec2[]>('distanceIntersection');

/** Doğru kesişimi: where the lines A→B and C→D (unbounded) cross. */
export const lineIntersection = op<(a: Vec2, b: Vec2, c: Vec2, d: Vec2) => Vec2 | null>('lineIntersection');

/** Hat üzerinde nokta: `distance` from A towards B (negative: behind A). */
export const alongLine = op<(a: Vec2, b: Vec2, distance: number) => Vec2 | null>('alongLine');

/** Açı-mesafe: from station S, `angle` (radians) clockwise from S→R, then `distance`. */
export const polarPoint = op<(s: Vec2, r: Vec2, angle: number, distance: number) => Vec2 | null>('polarPoint');

/** Clockwise angle (radians, 0…2π) at S from S→R to S→P. */
export const clockwiseAngle = op<(s: Vec2, r: Vec2, p: Vec2) => number>('clockwiseAngle');
