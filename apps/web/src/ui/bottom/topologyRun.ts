import type { AppContext } from '../../app/context';
import { Signal } from '../../core/signal';
import type { TopologyException } from '../../contracts/generated/TopologyException';
import type { TopologyRule } from '../../contracts/generated/TopologyRule';
import type { EntityGeometry } from '../../model/entities';
import { topologyCatalog, topologyCheck, topologyFix, type TopologyCatalog, type TopologyFinding } from '../../model/ops/topologyRules';
import { toleranceOf } from '../../model/topologyRules';
import type { EntityEdit } from '../../contracts/generated/EntityEdit';
import type { TopologySettings } from '../../contracts/generated/TopologySettings';
import { editGeometry, writeEdit } from '../../tools/editCommand';
import { findingView, missingLayers, noFindings, type TopologyFilter } from './topologyPlan';

/**
 * Topoloji kuralları' check, as the session keeps it (docs/adr/0202 §3–§5): the last check's findings and what they
 * were made of, the rows chosen and the filter; the check itself, a fix (`cad.entities.edit`'s `topologyFix`, the step
 * “Topoloji düzelt”) and the exceptions (the project's settings). The Topoloji tab draws it (TopologyPanel.ts); the
 * commands `topology.check` and `topology.rules` start here. The desktop's is apps/desktop/src/topology/.
 */

/** The last check: its findings, the objects they name (ids by the core's places), what it looked at, when. */
export interface TopologyChecked {
  readonly findings: readonly TopologyFinding[];
  readonly ids: readonly number[];
  /** The objects as they went to the core (a fix is worked out from the same). */
  readonly entities: readonly EntityGeometry[];
  readonly layers: readonly string[];
  readonly uids: readonly string[];
  readonly rules: readonly TopologyRule[];
  readonly looked: readonly number[];
  /** The drawing's revision the check saw: another means the results may be stale. */
  readonly revision: number;
  /** Rules whose layer or other layer the project does not have. */
  readonly missing: number;
}

/** The tab's state for the session: the last check, the chosen rows (places in the findings), the filter. */
export const topologyState = {
  checked: new Signal<TopologyChecked | null>(null),
  selected: new Signal<readonly number[]>([]),
  filter: new Signal<TopologyFilter>('open'),
  /** One rule's place in the rules, or null: every rule. */
  rule: new Signal<number | null>(null),
};

/** A new drawing starts the tab again. */
export function resetTopology(): void {
  topologyState.checked.set(null);
  topologyState.selected.set([]);
  topologyState.filter.set('open');
  topologyState.rule.set(null);
}

let catalog: TopologyCatalog | null = null;

/** The kinds', problems' and fixes' names (the core's, read once). */
export function topologyNames(): TopologyCatalog {
  catalog ??= topologyCatalog();
  return catalog;
}

/** The project's rules and exceptions. */
export function projectRules(ctx: AppContext): { rules: readonly TopologyRule[]; exceptions: readonly TopologyException[]; tolerance: number } {
  const t = ctx.doc.settings.topology.value;
  return { rules: t?.rules ?? [], exceptions: t?.exceptions ?? [], tolerance: toleranceOf(t) };
}

/** Denetle: every rule over the objects of the layers the rules name; the findings to the tab, the first row chosen. */
export function runTopologyCheck(ctx: AppContext, quiet = false): TopologyChecked {
  const { rules, exceptions, tolerance } = projectRules(ctx);
  const layerIds = new Set(ctx.doc.layers.leaves().map((l) => l.id));
  const missing = rules.filter((r) => !layerIds.has(r.layer) || (r.other !== undefined && !layerIds.has(r.other))).length;
  const named = new Set(rules.flatMap((r) => (r.other !== undefined ? [r.layer, r.other] : [r.layer])));
  const objects = [...ctx.doc.all()].filter((e) => named.has(e.layerId));
  const entities = objects as unknown as EntityGeometry[];
  const layers = objects.map((e) => e.layerId);
  const uids = objects.map((e) => ctx.doc.uidOf(e.id) ?? '');
  const got = topologyCheck(entities, layers, uids, rules, tolerance, exceptions);
  const checked: TopologyChecked = {
    findings: got.findings,
    ids: objects.map((e) => e.id),
    entities,
    layers,
    uids,
    rules,
    looked: got.looked,
    revision: ctx.doc.revision,
    missing,
  };
  topologyState.checked.set(checked);
  topologyState.selected.set([]);
  ctx.selection.problem.set(null);
  if (!quiet) {
    const open = got.findings.filter((f) => !f.exception).length;
    const exc = got.findings.length - open;
    if (missing) ctx.log.warn(missingLayers(missing));
    if (!got.findings.length) ctx.log.success(noFindings(rules.length, objects.length));
    else ctx.log.info(`Topoloji denetlendi: ${rules.length} kural, ${got.findings.length} bulgu (açık ${open}, istisna ${exc}).`);
  }
  return checked;
}

/** Whether the drawing changed since the check. */
export const isStale = (ctx: AppContext, c: TopologyChecked): boolean => c.revision !== ctx.doc.revision;

/** A row chosen: its objects selected, the view on its box, the finding shown over the drawing. */
export function showFinding(ctx: AppContext, c: TopologyChecked, at: number, zoom: boolean): void {
  const f = c.findings[at];
  if (!f) return;
  const ids = f.objects.map((k) => c.ids[k]).filter((id) => ctx.doc.get(id));
  ctx.selection.set(ids);
  ctx.selection.problem.set({ at: f.at, label: f.label, regions: f.regions, edges: f.edges });
  if (zoom) ctx.view.zoomToBox(findingView(f.bounds), 96);
}

/**
 * Düzelt: the fix `key` of finding `at` written in one undo step; the check run again. Refused (and said) when the
 * drawing changed since the check, the fix has no answer or the command refuses it.
 */
export function applyTopologyFix(ctx: AppContext, at: number, key: string): boolean {
  const c = topologyState.checked.value;
  const f = c?.findings[at];
  if (!c || !f) return false;
  if (isStale(ctx, c)) {
    ctx.log.warn('Çizim denetimden sonra değişti; önce Denetle ile yenileyin.');
    return false;
  }
  let changes;
  try {
    changes = topologyFix(c.entities, c.layers, c.rules, f, key);
  } catch (e) {
    ctx.log.warn(e instanceof Error ? e.message : String(e));
    return false;
  }
  const edits: EntityEdit[] = changes.map((ch) => {
    const uid = c.uids[ch.object];
    return ch.shape ? { kind: 'update', uid, geometry: editGeometry(ch.shape) } : { kind: 'remove', uid };
  });
  if (!writeEdit(ctx, 'topologyFix', edits)) return false;
  const label = f.fixes.find((x) => x.key === key)?.label ?? key;
  ctx.log.success(`Topoloji düzeltildi: ${f.label}, ${label}.`);
  const again = runTopologyCheck(ctx, true);
  // The row at the same place, if any: the next finding of the list.
  if (again.findings.length) topologyState.selected.set([Math.min(at, again.findings.length - 1)]);
  return true;
}

/**
 * İstisna yap or İstisnayı kaldır: the chosen findings marked as left on purpose, or no longer; a project setting (not
 * an undo step), the check's flags updated in place.
 */
export function toggleExceptions(ctx: AppContext, rows: readonly number[], on: boolean): number {
  const c = topologyState.checked.value;
  if (!c || !rows.length) return 0;
  const t = ctx.doc.settings.topology.value;
  if (!t) return 0;
  let list = [...(t.exceptions ?? [])];
  const tol = toleranceOf(t);
  const same = (x: TopologyException, f: TopologyFinding) =>
    x.rule === c.rules[f.rule]?.id && x.objects.length === f.objects.length && x.objects.every((u, i) => u === c.uids[f.objects[i]]) && Math.hypot(x.at.x - f.at.x, x.at.y - f.at.y) <= tol;
  let n = 0;
  for (const at of rows) {
    const f = c.findings[at];
    if (!f || f.exception === on) continue;
    if (on) list.push({ rule: c.rules[f.rule].id, objects: f.objects.map((k) => c.uids[k]), at: f.at });
    else list = list.filter((x) => !same(x, f));
    n++;
  }
  if (!n) return 0;
  const next: TopologySettings = { ...t };
  if (list.length) next.exceptions = list;
  else delete next.exceptions;
  ctx.doc.settings.assign({ topology: next });
  // The flags as the settings now say; the drawing did not change, so the check holds.
  const findings = c.findings.map((f, i) => (rows.includes(i) ? { ...f, exception: on } : f));
  topologyState.checked.set({ ...c, findings, revision: ctx.doc.revision });
  ctx.log.info(on ? `${n} bulgu istisna yapıldı.` : `${n} bulgunun istisnası kaldırıldı.`);
  return n;
}
