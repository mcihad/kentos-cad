import type { TopologyException } from '../contracts/generated/TopologyException';
import type { TopologyRule } from '../contracts/generated/TopologyRule';
import type { TopologyRuleKind } from '../contracts/generated/TopologyRuleKind';
import type { TopologySettings } from '../contracts/generated/TopologySettings';

/**
 * Topoloji kuralları (docs/adr/0202 §1): the project's rules, their tolerance and the findings marked as exceptions, as
 * the project keeps them (`ProjectSettings.topology`; the contract's `TopologySettings`). What each kind is called and
 * what it finds is the shared core's (`model/ops/topologyRules.ts`); here are the kinds' shapes and the rule a project
 * keeps them by (the contract's `TopologySettings::sanitized`). The desktop's is `kentos_contracts::topology`.
 */

export type { TopologyException, TopologyRule, TopologyRuleKind, TopologySettings };

/** The tolerance when a project names none (m). */
export const TOPOLOGY_TOLERANCE = 0.001;
/** The least and the greatest tolerance a project may name (m). */
export const TOPOLOGY_TOLERANCES = [0.000001, 1] as const;

/** What a kind's value is: a length in metres, an angle in radians below a right angle. */
export type TopologyValue = 'length' | 'angle';

/** Every kind: whether it is between two layers and what value it takes (the contract's `TopologyRuleKind`). */
export const TOPOLOGY_KINDS: Readonly<Record<TopologyRuleKind, { between: boolean; value: TopologyValue | null }>> = {
  mustNotOverlap: { between: false, value: null },
  mustNotHaveGaps: { between: false, value: null },
  mustNotHaveSlivers: { between: false, value: 'length' },
  mustNotHaveDuplicates: { between: false, value: null },
  mustNotHaveDangles: { between: false, value: null },
  mustNotHaveShortEdges: { between: false, value: 'length' },
  mustNotHaveSmallAngles: { between: false, value: 'angle' },
  mustBeValid: { between: false, value: null },
  mustNotHaveMissingVertices: { between: false, value: null },
  mustNotOverlapWith: { between: true, value: null },
  mustBeCoveredBy: { between: true, value: null },
  boundaryMustBeCoveredBy: { between: true, value: null },
  mustBeOnEndOf: { between: true, value: null },
};

/** Whether `t` is a tolerance a project may name. */
export const toleranceHolds = (t: number): boolean => Number.isFinite(t) && t >= TOPOLOGY_TOLERANCES[0] && t <= TOPOLOGY_TOLERANCES[1];

/** Whether `v` is a value a kind may take: finite and above zero; an angle below a right angle. */
export const valueHolds = (kind: TopologyValue, v: number): boolean => Number.isFinite(v) && v > 0 && (kind === 'length' || v < Math.PI / 2);

/** The tolerance the settings are checked with. */
export const toleranceOf = (t: TopologySettings | null | undefined): number => t?.tolerance ?? TOPOLOGY_TOLERANCE;

/**
 * The settings as a project keeps them (the contract's `TopologySettings::sanitized`): the tolerance that holds, the
 * rules that hold (of the same id the first), the exceptions of the rules kept with objects; null when nothing is left.
 */
export function sanitizeTopology(t: TopologySettings | null | undefined): TopologySettings | null {
  if (!t) return null;
  const rules: TopologyRule[] = [];
  for (const r of t.rules ?? []) {
    const kind = TOPOLOGY_KINDS[r.kind];
    if (!kind) continue;
    const otherHolds = kind.between ? !!r.other && r.other !== r.layer : r.other === undefined;
    if (!r.id || !r.layer || !otherHolds || rules.some((s) => s.id === r.id)) continue;
    const value = r.value !== undefined && kind.value && valueHolds(kind.value, r.value) ? r.value : undefined;
    rules.push({ id: r.id, kind: r.kind, layer: r.layer, ...(r.other !== undefined ? { other: r.other } : {}), ...(value !== undefined ? { value } : {}) });
  }
  const exceptions = (t.exceptions ?? []).filter(
    (x) => rules.some((r) => r.id === x.rule) && x.objects.length > 0 && Number.isFinite(x.at.x) && Number.isFinite(x.at.y),
  );
  const tolerance = t.tolerance !== undefined && toleranceHolds(t.tolerance) ? t.tolerance : undefined;
  if (!rules.length && !exceptions.length && tolerance === undefined) return null;
  return {
    ...(tolerance !== undefined ? { tolerance } : {}),
    ...(rules.length ? { rules } : {}),
    ...(exceptions.length ? { exceptions } : {}),
  };
}

/** Whether two settings are the same (the signal's equality: a project's settings are small). */
export const sameTopology = (a: TopologySettings | null, b: TopologySettings | null): boolean => JSON.stringify(a) === JSON.stringify(b);

/** A new rule's id: “kural-n”, the least n the rules do not use. */
export function nextRuleId(rules: readonly TopologyRule[]): string {
  for (let n = 1; ; n++) {
    const id = `kural-${n}`;
    if (!rules.some((r) => r.id === id)) return id;
  }
}
