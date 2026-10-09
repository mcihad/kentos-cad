import type { ScenarioOperation } from '../contracts/generated/ScenarioOperation';
import type { ScenariosEdit } from '../contracts/generated/ScenariosEdit';
import type { ScenariosEdited } from '../contracts/generated/ScenariosEdited';
import type { ScenariosEditPlan } from '../contracts/generated/ScenariosEditPlan';
import { Refusal, type CadDocument } from '../model/document';
import type { NewEntity } from '../model/entities';
import type { LayerNode } from '../model/layers';
import { inScenario, SCENARIO_NAME_MAX, scenarioPairs, scenarioProblem } from '../model/temporalRules';
import { checkRevision, error, failed, isBlank, validated, type Stop } from './checks';
import type { ProductCommand } from './command';

/**
 * `cad.scenarios.edit` v1 (docs/adr/0210 §9, §11): Senaryo oluştur makes a scenario group on top of the tree, hidden
 * (the view stays Mevcut durum until the interface shows it), with a copy of each base layer given standing for its
 * source (`replaces`) and, when asked, copies of its objects under new ids; Senaryoyu uygula moves each scenario
 * layer's objects onto its base layer in place of that layer's own, removes the emptied scenario layers and makes the
 * group an ordinary one keeping its other layers (or removes it when it keeps none). Each is one undo step. The web's
 * handler over `CadDocument`; the desktop's is `crates/native/application/src/scenarios_edit.rs`. Both pass the shared
 * cases in fixtures/commands/v1/cad.scenarios.edit.json.
 */

/** The undo step's name. */
export const SCENARIO_LABEL: Record<ScenarioOperation, string> = {
  create: 'Senaryo oluştur',
  apply: 'Senaryoyu uygula',
};

type Checked = { op: 'create'; sources: LayerNode[] } | { op: 'apply'; group: LayerNode; pairs: [string, string][] };

function check(doc: CadDocument, input: ScenariosEdit): Stop | Checked {
  const layers = doc.layers;
  if (input.operation === 'create') {
    const name = input.name ?? '';
    if (isBlank(name)) return failed(error('empty_name', 'Senaryonun adı boş olamaz; bir ad verin.', 'name'));
    if ([...name.trim()].length > SCENARIO_NAME_MAX) return failed(error('invalid_name', `Senaryonun adı ${SCENARIO_NAME_MAX} karakterden uzun; kısaltın.`, 'name'));
    const problem = scenarioProblem({ ...(input.note != null && { note: input.note }) });
    if (problem) return failed(error('invalid_note', `Senaryonun notu: ${problem}.`, 'note'));
    const ids = input.layers ?? [];
    const seen = new Set<string>();
    for (const [i, id] of ids.entries()) {
      if (seen.has(id)) return failed(error('duplicate_layer', `“${id}” katmanı iki kez verildi; her katman bir kez kopyalanır.`, `layers/${i}`));
      seen.add(id);
    }
    const stale = checkRevision(doc, input.expectedRevision);
    if (stale) return stale;
    const copy = input.copyObjects ?? true;
    const sources: LayerNode[] = [];
    for (const [i, id] of ids.entries()) {
      const at = `layers/${i}`;
      const node = layers.get(id);
      if (!node) return failed(error('layer_not_found', `“${id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.`, at));
      if (node.type !== 'layer' || inScenario(layers, id) || node.service)
        return failed(error('not_a_base_layer', `“${node.name}” bir ana katman değil; senaryo yalnız senaryo dışındaki, nesne tutan katmanları kopyalar.`, at));
      if (copy && layers.isLocked(id)) return failed(error('layer_locked', `“${node.name}” katmanı kilitli; nesneleri kopyalanmaz. Kilidi Katmanlar panelinden açın.`, at));
      sources.push(node);
    }
    return { op: 'create', sources };
  }
  const id = input.scenario;
  if (!id) return failed(error('no_scenario', 'Uygulanacak senaryonun kimliğini (scenario) verin.', 'scenario'));
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  const group = layers.get(id);
  if (!group || group.type !== 'group' || !group.scenario)
    return failed(error('scenario_not_found', `“${id}” kimlikli senaryo çizimde yok. Bir senaryo grubunun kimliğini verin.`, 'scenario'));
  const pairs = scenarioPairs(layers.tree, group);
  const locked = [...layers.leavesOf(id).map((l) => l.id), ...pairs.map(([b]) => b)].find((l) => layers.isLocked(l));
  if (locked !== undefined)
    return failed(error('layer_locked', `“${layers.get(locked)?.name ?? locked}” katmanı kilitli; senaryo uygulanmadı. Kilidi Katmanlar panelinden açın.`, 'scenario'));
  return { op: 'apply', group, pairs };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** The scenario group's layers that stand for no base layer of the tree, in order. */
const kept = (doc: CadDocument, group: string, pairs: readonly [string, string][]) =>
  doc.layers
    .leavesOf(group)
    .filter((l) => !pairs.some(([, s]) => s === l.id))
    .map((l) => l.id);

/** The objects of `layer` as new objects on `to`. */
const copiesOf = (doc: CadDocument, layer: string, to: string): NewEntity[] =>
  doc.byLayer(layer).map((e) => {
    const { id: _id, uid: _uid, ...rest } = structuredClone(e);
    return { ...rest, layerId: to } as NewEntity;
  });

function create(doc: CadDocument, input: ScenariosEdit, sources: readonly LayerNode[]): ScenariosEdited {
  const label = SCENARIO_LABEL.create;
  const group = doc.addLayer(
    { name: (input.name ?? '').trim(), type: 'group', children: [], visible: false, scenario: input.note != null ? { note: input.note } : {} },
    null,
    { index: 0, label },
  ).id;
  const copy = input.copyObjects ?? true;
  const pairs: { base: string; layer: string }[] = [];
  let objects = 0;
  for (const src of sources) {
    const layer = doc.addLayer(
      {
        name: src.name,
        type: 'layer',
        style: structuredClone(src.style),
        ...(src.snap && { snap: structuredClone(src.snap) }),
        ...(src.fields?.length && { fields: structuredClone(src.fields) }),
        ...(src.time && { time: structuredClone(src.time) }),
        replaces: src.id,
      },
      group,
      { label },
    ).id;
    if (copy) {
      const copies = copiesOf(doc, src.id, layer);
      objects += copies.length;
      doc.addMany(copies, label);
    }
    pairs.push({ base: src.id, layer });
  }
  return { scenario: group, layers: pairs, kept: [], objects, removed: 0, revision: '' };
}

function apply(doc: CadDocument, group: LayerNode, pairs: readonly [string, string][]): ScenariosEdited {
  const label = SCENARIO_LABEL.apply;
  let moved = 0;
  let removed = 0;
  for (const [base, layer] of pairs) {
    const old = doc.byLayer(base).map((e) => e.id);
    doc.remove(old);
    removed += old.length;
    moved += doc.updateMany(
      doc.byLayer(layer).map((e) => ({ id: e.id, layerId: base })),
      label,
    );
    if (doc.layers.active.value === layer) doc.layers.setActive(base);
    doc.removeLayer(layer);
  }
  const keep = kept(doc, group.id, pairs);
  // A layer standing for a base layer the tree no longer has stays as a base layer itself, its link dropped.
  for (const id of keep) {
    const n = doc.layers.get(id);
    if (n?.replaces != null) doc.setLayerTemporal(id, { ...(n.time && { time: n.time }) }, label);
  }
  if (doc.layers.get(group.id)?.children.length) doc.setLayerTemporal(group.id, {}, label);
  else doc.removeLayer(group.id);
  return { scenario: group.id, layers: pairs.map(([base, layer]) => ({ base, layer })), kept: keep, objects: moved, removed, revision: '' };
}

export const scenariosEdit: ProductCommand<ScenariosEdit, ScenariosEdited, ScenariosEditPlan> = {
  id: 'cad.scenarios.edit',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    const revision = String(doc.revision);
    if (checked.op === 'create') {
      const ids = doc.layers.peekIds(1 + checked.sources.length);
      const copy = input.copyObjects ?? true;
      const objects = copy ? checked.sources.reduce((n, s) => n + doc.byLayer(s.id).length, 0) : 0;
      return {
        status: 'completed',
        output: { scenario: ids[0], layers: checked.sources.map((s, i) => ({ base: s.id, layer: ids[i + 1] })), kept: [], objects, removed: 0, revision },
        warnings: [],
      };
    }
    const { group, pairs } = checked;
    return {
      status: 'completed',
      output: {
        scenario: group.id,
        layers: pairs.map(([base, layer]) => ({ base, layer })),
        kept: kept(doc, group.id, pairs),
        objects: pairs.reduce((n, [, l]) => n + doc.byLayer(l).length, 0),
        removed: pairs.reduce((n, [b]) => n + doc.byLayer(b).length, 0),
        revision,
      },
      warnings: [],
    };
  },

  /** Writes the scenario's change as one undo step named after the operation; a refusal halfway takes back what was written. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    let output: ScenariosEdited;
    try {
      output = doc.transact(SCENARIO_LABEL[input.operation], () =>
        checked.op === 'create' ? create(doc, input, checked.sources) : apply(doc, checked.group, checked.pairs),
      );
    } catch (e) {
      if (e instanceof Refusal) return failed(error('layer_refused', e.message, 'layers'));
      throw e;
    }
    return { status: 'completed', output: { ...output, revision: String(doc.revision) }, warnings: [] };
  },
};
