import type { LabelPin } from '../contracts/generated/LabelPin';
import type { LabelsPin } from '../contracts/generated/LabelsPin';
import type { LabelsPinned } from '../contracts/generated/LabelsPinned';
import type { LabelsPinPlan } from '../contracts/generated/LabelsPinPlan';
import { isUuid } from '../core/uuid';
import type { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import { labelPinsProblem } from '../model/labelRules';
import { checkRevision, error, failed, validated, type Stop } from './checks';
import type { ProductCommand } from './command';
import { sameData } from './layersLabels';

/**
 * `cad.labels.pin` v1 (docs/adr/0212 §5): labels moved, turned or hidden by hand, or freed, as one undo step
 * “Etiket”. A label is an object's and a class's (a rule-based layer's name; none, the object's first label); its pin
 * is kept on the object (`labelPins`), so it goes with the object when it is moved or copied. The web's handler over
 * `CadDocument`; the desktop's is `crates/native/application/src/labels_pin.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.labels.pin.json.
 *
 * The checks, in the contract's order: at least one change; each id is one; each pin by its rules and its change's
 * class; no label twice; the expected revision; each object is the drawing's, on an unlocked layer, its class one its
 * layer labels with; the pins each object would have by their rules. A change that leaves a pin as it is changes
 * nothing.
 */

/** The undo step's name. */
export const LABELS_PIN_LABEL = 'Etiket';

interface Changed {
  readonly entity: Entity;
  readonly uid: string;
  readonly pins: LabelPin[];
}

const withClass = (pin: LabelPin, cls: string | undefined): LabelPin => {
  const { class: _class, ...rest } = structuredClone(pin);
  return cls === undefined ? rest : { class: cls, ...rest };
};

function check(doc: CadDocument, input: LabelsPin): Stop | Changed[] {
  if (!input.pins.length) return failed(error('no_entities', 'Değişecek etiket yok. En az bir nesnenin etiketini verin.', 'pins'));
  for (const [i, c] of input.pins.entries()) {
    if (!isUuid(c.uid))
      return failed(
        error('invalid_uid', `“${c.uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).`, `pins[${i}]/uid`),
      );
    if (c.pin) {
      if (c.pin.class !== undefined && c.pin.class !== c.class)
        return failed(error('invalid_pin', 'İğnenin sınıfı değişikliğin sınıfıyla aynı olmalı (ya da verilmemeli).', `pins[${i}]/pin/class`));
      const problem = labelPinsProblem([withClass(c.pin, c.class)]);
      if (problem) return failed(error('invalid_pin', `Etiket iğnesi: ${problem}.`, `pins[${i}]/pin`));
    }
    if (input.pins.slice(0, i).some((d) => d.uid === c.uid && d.class === c.class))
      return failed(error('invalid_pin', 'Aynı nesnenin aynı etiketi iki kez değişiyor; her etiketi bir kez verin.', `pins[${i}]`));
  }
  const stale = checkRevision(doc, input.expectedRevision);
  if (stale) return stale;
  // Each object once, at its first change, its pins changed in the input's order.
  const objects: { entity: Entity; uid: string; pins: LabelPin[]; first: number }[] = [];
  for (const [i, c] of input.pins.entries()) {
    let o = objects.find((x) => x.uid === c.uid);
    if (!o) {
      const entity = doc.byUid(c.uid);
      if (!entity)
        return failed(
          error('entity_not_found', `“${c.uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.`, `pins[${i}]/uid`),
        );
      if (doc.layers.isLocked(entity.layerId)) {
        const name = doc.layers.get(entity.layerId)?.name ?? entity.layerId;
        return failed(error('layer_locked', `“${name}” katmanı kilitli; üzerindeki nesnenin etiketi değişmez. Kilidi Katmanlar panelinden açın.`, `pins[${i}]/uid`));
      }
      o = { entity, uid: c.uid, pins: structuredClone(entity.labelPins ?? []), first: i };
      objects.push(o);
    }
    const node = doc.layers.get(o.entity.layerId);
    const name = node?.name ?? o.entity.layerId;
    const rules = node?.style.labels?.mode === 'rules' ? node.style.labels : undefined;
    if (c.class !== undefined && !rules?.classes?.some((k) => k.name === c.class)) {
      const why = rules ? `“${name}” katmanının “${c.class}” adlı etiket sınıfı yok.` : `“${name}” katmanının tek etiketi var; sınıfı verilmez.`;
      return failed(error('unknown_class', why, `pins[${i}]/class`));
    }
    const at = o.pins.findIndex((p) => p.class === c.class);
    if (c.pin) {
      if (at >= 0) o.pins[at] = withClass(c.pin, c.class);
      else o.pins.push(withClass(c.pin, c.class));
    } else if (at >= 0) o.pins.splice(at, 1);
  }
  const out: Changed[] = [];
  for (const o of objects) {
    const problem = labelPinsProblem(o.pins);
    if (problem) return failed(error('invalid_pin', `Etiket iğneleri: ${problem}.`, `pins[${o.first}]`));
    if (!sameData(o.entity.labelPins ?? [], o.pins)) out.push({ entity: o.entity, uid: o.uid, pins: o.pins });
  }
  return out;
}

const isStop = (c: Stop | Changed[]): c is Stop => !Array.isArray(c);

export const labelsPin: ProductCommand<LabelsPin, LabelsPinned, LabelsPinPlan> = {
  id: 'cad.labels.pin',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    return {
      status: 'completed',
      output: { objects: checked.map((c) => ({ uid: c.uid, labelPins: c.pins })), revision: String(cx.doc.revision) },
      warnings: [],
    };
  },

  /** Writes the pins as one undo step “Etiket” when any changes. */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const doc = cx.doc;
    const changed = checked.length
      ? doc.updateMany(
          checked.map((c) => ({ id: c.entity.id, labelPins: c.pins.length ? c.pins : undefined })),
          LABELS_PIN_LABEL,
        )
      : 0;
    return { status: 'completed', output: { changed, revision: String(doc.revision) }, warnings: [] };
  },
};
