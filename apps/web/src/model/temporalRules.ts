// The rules of temporal layers and scenarios in the layer tree (docs/adr/0210 §2, §9): the contract's
// (crates/shared/contracts/src/temporal.rs) in TypeScript, word for word; both pass fixtures/temporal/v1/rules.json.

import type { LayerTime } from '../contracts/generated/LayerTime';
import type { ScenarioInfo } from '../contracts/generated/ScenarioInfo';
import type { LayerNode } from './layers';

/** The longest name of a time setting's field, in characters. */
export const TIME_FIELD_MAX = 64;
/** The longest scenario note, in characters. */
export const SCENARIO_NOTE_MAX = 500;
/** The longest scenario name, in characters. */
export const SCENARIO_NAME_MAX = 80;

const chars = (s: string) => [...s].length;

function fieldProblem(what: string, name: string): string | null {
  if (name.trim() !== name) return `${what} alanının adının başında ya da sonunda boşluk var`;
  if (!name) return `${what} alanının adı boş`;
  return chars(name) > TIME_FIELD_MAX ? `${what} alanının adı ${TIME_FIELD_MAX} karakterden uzun` : null;
}

/** What is wrong with a layer's time setting, when anything is (`LayerTime::problem`). */
export function layerTimeProblem(t: LayerTime): string | null {
  return (
    fieldProblem('başlangıç', t.start) ??
    (t.end != null ? fieldProblem('bitiş', t.end) : null) ??
    (t.key != null ? fieldProblem('kimlik', t.key) : null) ??
    (t.end != null && t.end === t.start ? 'başlangıç ve bitiş aynı alan olamaz' : null)
  );
}

/** What is wrong with a scenario's note, when anything is (`ScenarioInfo::problem`). */
export function scenarioProblem(s: ScenarioInfo): string | null {
  const note = s.note;
  if (note == null) return null;
  if (note.trim() !== note || !note) return 'senaryonun notu boş ya da başında veya sonunda boşluk var';
  return chars(note) > SCENARIO_NOTE_MAX ? `senaryonun notu ${SCENARIO_NOTE_MAX} karakterden uzun` : null;
}

/**
 * What is wrong with the layer tree's time settings and scenarios, when anything is (`scenarios_problem`): a time on
 * a group or a service layer, a broken setting; a scenario on a layer or inside another scenario; a `replaces`
 * outside a scenario, on a group, empty, naming itself, naming a node that is not a base layer, or a base layer
 * replaced twice in one scenario. A `replaces` naming no node of the tree is not wrong: it is left out.
 */
export function scenariosProblem(tree: readonly LayerNode[]): string | null {
  const find = (nodes: readonly LayerNode[], id: string, inScenario: boolean): [LayerNode, boolean] | null => {
    for (const n of nodes) {
      if (n.id === id) return [n, inScenario];
      const found = find(n.children, id, inScenario || n.scenario != null);
      if (found) return found;
    }
    return null;
  };
  const replaced: [string, string][] = [];
  const walk = (nodes: readonly LayerNode[], scenario: LayerNode | null): string | null => {
    for (const n of nodes) {
      if (n.time) {
        if (n.type === 'group') return `“${n.name}” bir grup; grubun zamanı olmaz, zaman katmanındır`;
        if (n.service) return `“${n.name}” servisten çizilir; nesnesi olmayan katmanın zamanı olmaz`;
        const p = layerTimeProblem(n.time);
        if (p) return `“${n.name}” katmanının zamanı: ${p}`;
      }
      if (n.scenario) {
        if (n.type === 'layer') return `“${n.name}” bir katman; yalnız grup senaryo olur`;
        if (scenario) return `“${n.name}” senaryosu “${scenario.name}” senaryosunun içinde; senaryo iç içe olmaz`;
        const p = scenarioProblem(n.scenario);
        if (p) return `“${n.name}” senaryosu: ${p}`;
      }
      if (n.replaces != null) {
        const target = n.replaces;
        if (!scenario) return `“${n.name}” bir senaryonun içinde değil; yalnız senaryo katmanı bir katmanın yerine geçer`;
        if (n.type === 'group') return `“${n.name}” bir grup; yalnız katman bir katmanın yerine geçer`;
        if (!target) return `“${n.name}” katmanının yerine geçtiği katman boş`;
        if (target === n.id) return `“${n.name}” katmanı kendi yerine geçemez`;
        const found = find(tree, target, false);
        if (found) {
          const [base, inScenario] = found;
          if (base.type !== 'layer' || inScenario || base.scenario != null)
            return `“${n.name}” katmanı “${base.name}” düğümünün yerine geçiyor; yalnız bir ana katmanın (senaryo dışındaki katmanın) yerine geçilir`;
          if (replaced.some(([s, t]) => s === scenario.id && t === target)) return `“${scenario.name}” senaryosunda “${base.name}” katmanının yerine iki katman geçiyor`;
          replaced.push([scenario.id, target]);
        }
      }
      const p = walk(n.children, n.scenario ? n : scenario);
      if (p) return p;
    }
    return null;
  };
  return walk(tree, null);
}

/** The base layers a scenario group stands in for (its layers' `replaces` naming a base layer of `tree`), each with the scenario layer, in the tree's order. */
export function scenarioPairs(tree: readonly LayerNode[], scenario: LayerNode): [base: string, layer: string][] {
  const isBase = (nodes: readonly LayerNode[], id: string, inside: boolean): boolean =>
    nodes.some((n) => (n.id === id && !inside && n.type === 'layer') || isBase(n.children, id, inside || n.scenario != null));
  const out: [string, string][] = [];
  const walk = (nodes: readonly LayerNode[]) => {
    for (const n of nodes) {
      if (n.replaces != null && n.type === 'layer' && isBase(tree, n.replaces, false)) out.push([n.replaces, n.id]);
      walk(n.children);
    }
  };
  walk(scenario.children);
  return out;
}

/** Whether the node or a group above it is a scenario. */
export function inScenario(layers: { get(id: string): LayerNode | undefined; parentOf(id: string): LayerNode | null }, id: string): boolean {
  for (let n = layers.get(id) ?? null; n; n = layers.parentOf(n.id)) if (n.scenario) return true;
  return false;
}
