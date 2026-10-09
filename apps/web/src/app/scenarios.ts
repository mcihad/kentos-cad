// Senaryolar (docs/adr/0210 §9; the desktop's `scenarios.rs`): which scenario the drawing shows, Senaryoyu göster and
// Mevcut durum as changes of the tree's visibility (as the eye button: an edit, not an undo step), Senaryo oluştur and
// Senaryoyu uygula through `cad.scenarios.edit`.

import type { LayerNode, LayerStore } from '../model/layers';
import { scenarioPairs } from '../model/temporalRules';
import { scenariosEdit } from '../product/scenariosEdit';
import type { AppContext } from './context';

/** What the drawing shows: the field state, one scenario, or a mix of them. */
export type Shown = { kind: 'base' } | { kind: 'scenario'; id: string } | { kind: 'mixed' };

/** The scenario groups of the tree, in its order. */
export function scenarioGroups(layers: LayerStore): LayerNode[] {
  return layers.all().filter((n) => n.type === 'group' && n.scenario);
}

/** The base layers scenario `id` stands in for, each with its layer in the scenario. */
export function pairsOf(layers: LayerStore, id: string): [base: string, layer: string][] {
  const group = layers.get(id);
  return group?.scenario ? scenarioPairs(layers.tree, group) : [];
}

/**
 * Gösterilen senaryo: one scenario group shown and the base layers it stands in for hidden, that scenario; none shown,
 * Mevcut durum; anything else a mix.
 */
export function shownScenario(layers: LayerStore): Shown {
  const shown = scenarioGroups(layers).filter((g) => layers.isVisible(g.id));
  if (!shown.length) return { kind: 'base' };
  if (shown.length === 1 && pairsOf(layers, shown[0].id).every(([b]) => !layers.isVisible(b))) return { kind: 'scenario', id: shown[0].id };
  return { kind: 'mixed' };
}

/** The scenario a layer or a group is in (itself when it is one), or undefined. */
export function scenarioOf(layers: LayerStore, id: string): LayerNode | undefined {
  for (let n = layers.get(id) ?? null; n; n = layers.parentOf(n.id)) if (n.scenario) return n;
  return undefined;
}

/**
 * Senaryoyu göster: its group shown, the other scenario groups hidden, the base layers it stands in for hidden, those
 * only others stand in for shown. An active base layer hidden gives way to its layer in the scenario.
 */
export function showScenario(ctx: AppContext, id: string): void {
  const layers = ctx.doc.layers;
  const group = layers.get(id);
  if (!group?.scenario) return;
  const mine = pairsOf(layers, id);
  const bases = new Set(mine.map(([b]) => b));
  for (const g of scenarioGroups(layers)) {
    if (g.id !== id) {
      layers.setVisible(g.id, false);
      for (const [b] of pairsOf(layers, g.id)) if (!bases.has(b)) layers.setVisible(b, true);
    }
  }
  layers.setVisible(id, true);
  for (const b of bases) layers.setVisible(b, false);
  const pair = mine.find(([b]) => b === layers.active.value);
  if (pair) layers.setActive(pair[1]);
  ctx.log.info(`Senaryo gösteriliyor: ${group.name}.`);
}

/** Mevcut durum: every scenario group hidden, the base layers they stand in for shown; an active scenario layer hidden gives way to its base layer. */
export function showBase(ctx: AppContext): void {
  const layers = ctx.doc.layers;
  const active = layers.active.value;
  for (const g of scenarioGroups(layers)) {
    const pairs = pairsOf(layers, g.id);
    layers.setVisible(g.id, false);
    for (const [b, l] of pairs) {
      layers.setVisible(b, true);
      if (l === active) layers.setActive(b);
    }
  }
  ctx.log.info('Mevcut durum gösteriliyor.');
}

/** The scenario a command means: the selected or the active layer's, else the one shown, else the first. */
export function scenarioAtHand(ctx: AppContext): LayerNode | undefined {
  const layers = ctx.doc.layers;
  const first = [...ctx.selection.ids.value].map((id) => ctx.doc.get(id)?.layerId).find((l) => l !== undefined);
  const shown = shownScenario(layers);
  return (
    (first !== undefined ? scenarioOf(layers, first) : undefined) ??
    scenarioOf(layers, layers.active.value) ??
    (shown.kind === 'scenario' ? layers.get(shown.id) : undefined) ??
    scenarioGroups(layers)[0]
  );
}

/** Senaryo oluştur through `cad.scenarios.edit`, then the scenario shown. Its group's id, or undefined when refused (said). */
export function createScenario(ctx: AppContext, input: { name: string; layers: string[]; copyObjects: boolean; note?: string }): string | undefined {
  const result = scenariosEdit.execute({ doc: ctx.doc }, { operation: 'create', ...input });
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return undefined;
  }
  const { scenario, objects } = result.output;
  showScenario(ctx, scenario);
  ctx.log.success(`Senaryo oluşturuldu: ${input.name.trim()} (${input.layers.length} katman, ${objects} nesne kopyalandı).`);
  return scenario;
}

/** Senaryoyu uygula through `cad.scenarios.edit`, then Mevcut durum with the group it leaves shown. Whether it was applied. */
export function applyScenario(ctx: AppContext, id: string): boolean {
  const name = ctx.doc.layers.get(id)?.name ?? id;
  const result = scenariosEdit.execute({ doc: ctx.doc }, { operation: 'apply', scenario: id });
  if (result.status !== 'completed') {
    if ('error' in result) ctx.log.warn(result.error.message);
    return false;
  }
  showBase(ctx);
  const o = result.output;
  // The layers it kept are base layers now: shown, with their group.
  if (ctx.doc.layers.get(id)) ctx.doc.layers.setVisible(id, true);
  for (const p of o.layers) ctx.doc.layers.setVisible(p.base, true);
  ctx.log.success(`Senaryo uygulandı: ${name} (${o.layers.length} katman; ${o.objects} nesne taşındı, ${o.removed} nesne silindi).`);
  return true;
}
