import type { AppContext } from '../app/context';
import type { PolarAngles } from '../app/format';
import type { Vec2 } from '../model/geometry';
import type { Edge } from '../model/geom/intersect';
import { nearestEdge } from '../model/ops/edges';
import { op } from '../wasm/core';
import { takesTypedInput, type ToolPointer } from './Tool';
import type { Tracking } from './tracking';

/**
 * The digitizing locks' arithmetic (docs/adr/0166), from the Rust core (tools/locks.rs): the locked point, the cursor
 * rules with the locks, the direction of a typed angle and of a deflection, and Dik kapat's corner. Lock text (`<45`)
 * is read here with the point grammar's number, as coordinateInput.ts reads point text; the shared cases in
 * fixtures/locks/v1/cases.json hold both readers to the same answers.
 */

/** A locked direction: its unit vector, and whether the cursor may take the other way along the line (Paralel, Dik). */
export interface LockDirection {
  readonly u: Vec2;
  readonly both: boolean;
}

/** The locked point from the reference `o` with the cursor (or a snapped point) at `c`; null when a length alone has no way. */
export const lockPoint = op<(o: Vec2, c: Vec2, length: number | null, u: Vec2 | null, both: boolean) => Vec2 | null>('lockPoint');

/** The cursor rules (ortho, polar tracking) with the locks; null when a length alone has no way to go. */
export const constrainLocked =
  op<
    (
      from: Vec2 | null,
      world: Vec2,
      exact: boolean,
      ortho: boolean,
      polarStep: number | null,
      tol: number,
      length: number | null,
      u: Vec2 | null,
      both: boolean,
    ) => { point: Vec2; tracking: Tracking | null } | null
  >('constrainLocked');

/** The unit direction of an angle typed in the project's way (ADR 0165 §4). */
export const lockDirection = op<(angle: number, fromNorth: boolean, grads: boolean) => Vec2>('lockDirection');

/** Sapma: the previous edge's direction turned by `angle` the way the project's angles run; null without an edge. */
export const lockDeflected = op<(prev: Vec2, from: Vec2, angle: number, fromNorth: boolean, grads: boolean) => Vec2 | null>('lockDeflected');

/** Dik kapat: the corner that closes a right-angled shape square; null when the first and last edge are parallel. */
export const squareCorner = op<(first: Vec2, second: Vec2, prev: Vec2, last: Vec2) => Vec2 | null>('squareCorner');

/** Nesneye paralel ve dik: a picked edge's direction at `p` (a straight edge's own way, an arc's tangent); null without one. */
export const lockEdgeDirection = op<(edge: Edge, p: Vec2) => Vec2 | null>('lockEdgeDirection');

const LOCK_TEXT = /^<\s*([-+]?\d+(?:\.\d+)?)$/;

/** `<45`: the angle a typed direction lock names (in the project's way and unit); null for any other text. */
export function parseLockText(text: string): number | null {
  const m = text.trim().match(LOCK_TEXT);
  return m ? +m[1] : null;
}

// ── The session's locks (docs/adr/0166 §1), the desktop's kentos_interaction::locks ─────────────────────────────────

/** Where a locked direction comes from: an angle typed in the project's way (Açı, CBS's Semt), a turn from the previous edge (Sapma), a picked edge (Nesneye paralel, dik). */
export type Toward =
  | { readonly kind: 'angle'; readonly value: number }
  | { readonly kind: 'deflection'; readonly value: number }
  | { readonly kind: 'parallel'; readonly u: Vec2 }
  | { readonly kind: 'perpendicular'; readonly u: Vec2 };

/** What the value card asks for when a lock is chosen from the menu: its typed number locks that. */
export type LockAsk = 'length' | 'angle' | 'deflection';

/** What holds the next point; the session's, never saved, a new command starts with none. */
export interface LockState {
  /** Metres from the reference. */
  readonly length: number | null;
  readonly toward: Toward | null;
  /** Kalıcı: the locks stay for the points after the next, until the command ends. */
  readonly keep: boolean;
  /** The reference the locks were made at: when it moves (a point was placed), one-shot locks go and kept ones follow. */
  readonly at: Vec2 | null;
  /** The edge a Paralel or Dik lock was picked on: drawn while it holds. */
  readonly edge: Edge | null;
}

export const NO_LOCKS: LockState = { length: null, toward: null, keep: false, at: null, edge: null };

/** Nesneye paralel or dik waiting for its edge (§3): the next press on the drawing picks it instead of reaching the tool. */
export type LockPick = 'parallel' | 'perpendicular';

/** The step the prompt shows while the edge is awaited (the desktop's `LockPick::step`). */
export const lockPickStep = (pick: LockPick): string => `${pick === 'parallel' ? 'paralel' : 'dik'} kilidi için kenarı ya da yayı seçin [Vazgeç (Esc)]`;

/** What a press finds no edge under. */
export const NO_LOCK_EDGE = 'Tıklanan yerde düz kenar ya da yay yok.';

/** The kinds whose edges lend a lock their direction: those a tangent circle touches (curveTools.ts). */
const LOCK_EDGE_KINDS = ['line', 'polyline', 'polygon', 'arc', 'circle', 'xline', 'ray'];

/** The edge under a press and its direction there: the nearest edge of the nearest line, polyline, area, arc, circle, construction line or ray. */
export function pickedEdge(ctx: AppContext, p: ToolPointer): { edge: Edge; u: Vec2 } | null {
  const e = ctx.view.pickEdge(p.screen, (x) => LOCK_EDGE_KINDS.includes(x.kind));
  const edge = e && nearestEdge(e, p.raw);
  const u = edge && lockEdgeDirection(edge, p.raw);
  return edge && u ? { edge, u } : null;
}

/** A lock asked for with no point to measure it from (the desktop's words). */
export const NO_LOCK_REFERENCE = 'Kilit için önce bir nokta verin: uzunluk ve doğrultu son noktadan ölçülür.';
/** Sapma asked for with no edge to turn from. */
export const NO_TRAVEL = 'Sapma için önce bir kenar çizin: sapma önceki kenarın doğrultusundan ölçülür.';
/** What Esc and Kilitleri kaldır say. */
export const LOCKS_GONE = 'Kilitler kaldırıldı.';

export const hasLocks = (s: LockState): boolean => s.length !== null || s.toward !== null;

const samePoint = (a: Vec2 | null, b: Vec2 | null): boolean => (a === null || b === null ? a === b : a.x === b.x && a.y === b.y);

/** The locks after the reference moved to `from`: one-shot ones go, kept ones follow it. */
export function followed(s: LockState, from: Vec2 | null): LockState {
  if (!hasLocks(s) || samePoint(from, s.at)) return s;
  if (from && s.keep) return { ...s, at: from };
  return { ...s, length: null, toward: null, at: null, edge: null };
}

/** The direction locked for the next point, as the core takes it (none for a deflection with no edge to turn from). */
export function lockedDirection(s: LockState, travel: Vec2 | null, angles: PolarAngles): LockDirection | null {
  const t = s.toward;
  if (!t) return null;
  switch (t.kind) {
    case 'angle':
      return { u: lockDirection(t.value, angles.fromNorth, angles.grads), both: false };
    case 'deflection': {
      const u = travel && lockDeflected({ x: 0, y: 0 }, travel, t.value, angles.fromNorth, angles.grads);
      return u ? { u, both: false } : null;
    }
    case 'parallel':
      return { u: t.u, both: true };
    case 'perpendicular':
      return { u: { x: -t.u.y, y: t.u.x }, both: true };
  }
}

/** The locks as the value card's chips and the cursor's tag say them (“Uzunluk 12.500 m”, “Semt 100.0000 g” …). */
export function lockWords(ctx: AppContext, s: LockState = ctx.settings.locks.value): string[] {
  const f = ctx.format;
  const out: string[] = [];
  if (s.length !== null) out.push(`Uzunluk ${f.length(s.length)}`);
  const t = s.toward;
  if (t?.kind === 'angle') out.push(`${f.directionName} ${f.angle(f.angleFromTyped(t.value))}`);
  else if (t?.kind === 'deflection') out.push(`Sapma ${f.angle(f.angleFromTyped(t.value))}`);
  else if (t?.kind === 'parallel') out.push('Paralel');
  else if (t?.kind === 'perpendicular') out.push('Dik');
  if (s.keep && hasLocks(s)) out.push('Kalıcı');
  return out;
}

/** The point the locks are measured from: the running command's last point, or where a grip being moved was. */
export function lockReference(ctx: AppContext): Vec2 | null {
  const tool = ctx.tools.active;
  if (!takesTypedInput(ctx.tools.activeId.value, tool) || tool.snaps === false) return null;
  // Referans noktası's or Yapım kipi's point stands in for the last corner (docs/adr/0166 §5).
  return ctx.tools.reference?.value ?? tool.snapFrom?.() ?? null;
}

/** Whether a command runs that takes points: Referans noktası and Yapım kipi work then, before its first corner too. */
export function takesPoints(ctx: AppContext): boolean {
  const tool = ctx.tools.active;
  return takesTypedInput(ctx.tools.activeId.value, tool) && tool.snaps !== false && !!tool.acceptPoint;
}

/** Referans noktası or Yapım kipi asked where no command takes points (the desktop's words). */
export const NO_REFERENCE_TOOL = 'Referans noktası, nokta bekleyen bir komut çalışırken verilir.';

/** The direction the object being drawn travels at its last point (Sapma turns from it). */
export const lockTravel = (ctx: AppContext): Vec2 | null => ctx.tools.active.travelDirection?.() ?? null;

/** After an event: the locks follow the reference; one-shot ones go once their point is placed. */
export function followLocks(ctx: AppContext): void {
  // A one-shot reference is done once a corner was placed from it (docs/adr/0166 §5).
  ctx.tools?.followReference?.();
  // A context without locks (a tool's unit test) has nothing to follow.
  const s = ctx.settings.locks?.value;
  if (!s || !hasLocks(s)) return;
  const next = followed(s, lockReference(ctx));
  if (next !== s) ctx.settings.locks.set(next);
}

function sayLocks(ctx: AppContext): void {
  ctx.log.info(`Kilit: ${lockWords(ctx).join(' · ')}.`);
}

/** Locks the next point's length (metres) at the lock reference; false (and why) without one. */
export function lockLength(ctx: AppContext, metres: number): boolean {
  const at = lockReference(ctx);
  if (!at) return void ctx.log.warn(NO_LOCK_REFERENCE), false;
  ctx.settings.locks.set({ ...followed(ctx.settings.locks.value, at), length: metres, at });
  sayLocks(ctx);
  ctx.view.requestOverlay();
  return true;
}

/** Locks the next point's direction at the lock reference; a deflection needs an edge to turn from. */
export function lockToward(ctx: AppContext, toward: Toward): boolean {
  const at = lockReference(ctx);
  if (!at) return void ctx.log.warn(NO_LOCK_REFERENCE), false;
  if (toward.kind === 'deflection' && !lockTravel(ctx)) return void ctx.log.warn(NO_TRAVEL), false;
  ctx.settings.locks.set({ ...followed(ctx.settings.locks.value, at), toward, at, edge: null });
  sayLocks(ctx);
  ctx.view.requestOverlay();
  return true;
}

/** Kalıcı on or off for the running command. */
export function keepLocks(ctx: AppContext, keep: boolean): void {
  ctx.settings.locks.set({ ...ctx.settings.locks.value, keep });
  ctx.log.info(keep ? 'Kilitler kalıcı: sonraki noktalarda da durur.' : 'Kilitler tek seferlik: nokta konunca kalkar.');
}

/** Kilitleri kaldır (and Esc's first step): every lock goes, Kalıcı stays with the command. Whether there were any. */
export function clearLocks(ctx: AppContext): boolean {
  const s = ctx.settings.locks.value;
  if (!hasLocks(s)) return false;
  ctx.settings.locks.set({ ...s, length: null, toward: null, at: null, edge: null });
  ctx.log.info(LOCKS_GONE);
  ctx.view.requestOverlay();
  return true;
}

/** `<45` typed where a point is expected: the next point's direction locked at that angle (§6). Whether it was lock text. */
export function typedLock(ctx: AppContext, text: string): boolean {
  const a = parseLockText(text);
  if (a === null || !takesTypedInput(ctx.tools.activeId.value, ctx.tools.active)) return false;
  lockToward(ctx, { kind: 'angle', value: a });
  return true;
}
