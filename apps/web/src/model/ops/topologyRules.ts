import { op } from '../../wasm/core';
import type { TopologyException } from '../../contracts/generated/TopologyException';
import type { TopologyRule } from '../../contracts/generated/TopologyRule';
import type { EntityGeometry } from '../entities';
import type { Bounds, Vec2 } from '../geometry';

/**
 * Topoloji kuralları (docs/adr/0202): the rules' catalog (the kinds' names, what each looks at and takes, the problems'
 * and the fixes' names), the check and a fix, computed by the geometry core (`ops::topology_rules`; the desktop calls
 * the same functions). Objects go in whole, as the drawing holds them, with their layers' ids and their persistent ids
 * beside them. The core writes no field it has no value for. The independent reference is
 * scripts/fixtures/topology_rules_cases.py.
 */

/** A kind as the rules window lists it. */
export interface TopologyKind {
  readonly key: TopologyRule['kind'];
  /** Its name; “…” is the other layer's in a rule between layers. */
  readonly label: string;
  /** What it looks at (“alanlar”, “çizgiler ve noktalar” …). */
  readonly takes: string;
  readonly between: boolean;
  readonly value?: 'length' | 'angle';
  readonly defaultValue?: number;
  readonly valueLabel?: string;
}

export interface Named {
  readonly key: string;
  readonly label: string;
}

export interface TopologyCatalog {
  readonly kinds: readonly TopologyKind[];
  readonly problems: readonly Named[];
  readonly fixes: readonly Named[];
}

/** An area as the core writes it: its outer ring and holes, each its vertices and bulges. */
export interface CoreRing {
  readonly pts: readonly Vec2[];
  readonly bulges?: readonly number[] | null;
}

export interface CoreArea {
  readonly outer: CoreRing;
  readonly holes: readonly CoreRing[];
}

/** An edge as the core writes it. */
export type CoreEdge = { readonly kind: 'seg'; readonly a: Vec2; readonly b: Vec2 } | { readonly kind: 'arc'; readonly c: Vec2; readonly r: number; readonly a0: number; readonly sweep: number };

/** One finding (docs/adr/0202 §3): the rule's place in the rules, the problem, the objects (places in the list), the place. */
export interface TopologyFinding {
  readonly rule: number;
  readonly problem: string;
  readonly label: string;
  readonly objects: readonly number[];
  readonly at: Vec2;
  readonly bounds: Bounds;
  readonly regions: readonly CoreArea[];
  readonly edges: readonly CoreEdge[];
  readonly measure?: number;
  readonly measureKind?: 'area' | 'length' | 'angle' | 'distance';
  readonly fixes: readonly Named[];
  readonly exception: boolean;
  readonly spot?: { readonly part: number; readonly ring: number; readonly index: number };
  readonly target?: Vec2;
  readonly subject?: number;
}

export interface TopologyChecked {
  readonly findings: readonly TopologyFinding[];
  /** How many objects each rule looked at. */
  readonly looked: readonly number[];
}

/** One object a fix changes: its new geometry, or none when it is deleted. */
export interface TopologyChange {
  readonly object: number;
  readonly shape?: EntityGeometry;
}

/** The kinds, the problems and the fixes, as the core names them. */
export const topologyCatalog = op<() => TopologyCatalog>('topologyCatalog');

/** Every rule over the objects, each finding marked when an exception names it. */
export const topologyCheck = op<
  (entities: readonly EntityGeometry[], layers: readonly string[], uids: readonly string[], rules: readonly TopologyRule[], tolerance: number, exceptions: readonly TopologyException[]) => TopologyChecked
>('topologyCheck');

/** What a fix writes; it throws with the reason when it cannot. */
export const topologyFix = op<(entities: readonly EntityGeometry[], layers: readonly string[], rules: readonly TopologyRule[], finding: TopologyFinding, key: string) => TopologyChange[]>('topologyFix');
