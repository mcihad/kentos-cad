import type { AppContext } from '../app/context';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import type { NewObject } from '../contracts/generated/NewObject';
import { drawsLines, entityGeometry, type Entity, type EntityGeometry } from '../model/entities';
import { templateMemberCentroid, templateMemberOffsets } from '../model/ops/templateMembers';
import { vertexPoints } from '../model/ops/vertexPoints';
import { textIncrement } from '../model/textEdit';
import { elevatedPaths } from '../product/elevation';
import { entitiesCreate } from '../product/entitiesCreate';
import type { RunMember, TemplateRun } from './templateStamp';

/**
 * A group template's members (docs/adr/0176 §5; the desktop's `templates.rs`): while a group template runs, every
 * object its tool writes takes its members' objects in the same undo step: the same shape on the member's layer, its
 * parallels, points at its vertices (Köşelere nokta's rule: a shared corner once, a corner with a point passed over),
 * a point or a text at its centroid. A member's point names and texts go on in its series (`ToolManager.templateNames`,
 * by the member template's id). A member that makes nothing, or whose layer refuses it, is said; the object and the
 * other members stay.
 */

/** Runs a tool's write of its object; with a group template running, its members' objects are written too, in one undo step. */
export function withMembers<T>(ctx: AppContext, write: () => T): T {
  const run = ctx.settings.template.value;
  if (!run?.members?.length) return write();
  const { doc } = ctx;
  return doc.transact('Ekle', () => {
    const from = doc.nextSlot;
    const out = write();
    const made: Entity[] = [];
    for (let id = from; id < doc.nextSlot; id++) {
      const e = doc.get(id);
      if (e) made.push(e);
    }
    if (made.length) writeMembers(ctx, run, made);
    return out;
  });
}

/** Every member's objects for the objects the tool wrote, each member on its layer; how many were written is said. */
function writeMembers(ctx: AppContext, run: TemplateRun, made: readonly Entity[]): void {
  let count = 0;
  for (const main of made)
    for (const m of run.members ?? []) {
      const objects = memberObjects(ctx, m, main);
      if (!objects.length) continue;
      const result = entitiesCreate.execute({ doc: ctx.doc }, { layerId: m.layerId, objects });
      if (result.status !== 'completed') {
        if ('error' in result) ctx.log.warn(`“${m.name}” üyesi yazılmadı: ${result.error.message}`);
        continue;
      }
      for (const w of result.warnings) ctx.log.warn(w.message);
      count += objects.length;
    }
  if (count) ctx.log.info(`${run.name}: ${count} üye nesnesi yazıldı.`);
}

/** What one member makes from one object the tool wrote. */
function memberObjects(ctx: AppContext, m: RunMember, main: Entity): NewObject[] {
  const series = ctx.tools.templateNames;
  /** An object with the member's look, attributes (`attrs`, the object's own, over them) and label (`label` first). */
  const object = (g: EntityGeometry, own: { attrs?: Record<string, string>; label?: string | null } = {}): NewObject => {
    const attrs = { ...m.stamp.attrs, ...own.attrs };
    // A text's label is its first text: the object takes none.
    const label = own.label === null ? undefined : (own.label ?? m.stamp.label);
    return {
      geometry: g as unknown as NewGeometry,
      ...(m.color !== null && { color: m.color }),
      ...(m.lineWeight !== null && drawsLines(g) && { lineWeight: m.lineWeight }),
      ...(m.stamp.symbol !== undefined && { symbol: m.stamp.symbol }),
      ...(Object.keys(attrs).length > 0 && { attrs }),
      ...(label !== undefined && { label }),
    };
  };
  const code = m.point?.code ? { Kod: m.point.code } : undefined;
  switch (m.rule) {
    case 'same':
      return [object(entityGeometry(main))];
    case 'offset': {
      const made = templateMemberOffsets(main, m.distance ?? 0, m.side ?? 'both');
      if ('error' in made) {
        ctx.log.warn(`“${m.name}” üyesi yazılmadı: ${made.error}`);
        return [];
      }
      return made.geometries.map((g) => object(g));
    }
    case 'vertices': {
      // Every point's place is taken, hidden layers' too (Köşelere nokta's rule).
      const existing = [...ctx.doc.all()].flatMap((e) => (e.kind === 'point' ? [e.p] : []));
      const first = series.get(m.id) ?? m.point?.name ?? null;
      const result = vertexPoints([elevatedPaths(main)], existing, first || null);
      if (result.next) series.set(m.id, result.next);
      return result.points.map((p) => object({ kind: 'point', p: p.p, ...(p.z !== null && p.z !== undefined && { z: p.z }) } as EntityGeometry, { ...(code && { attrs: code }), ...(p.name && { label: p.name }) }));
    }
    case 'centroid': {
      const p = templateMemberCentroid(main);
      if (!p) return [];
      if (m.text) {
        const text = series.get(m.id) ?? m.stamp.label ?? '';
        if (!text) return [];
        series.set(m.id, textIncrement(text) ?? text);
        const height = (m.text.heightMm * ctx.doc.settings.plotScale.value) / 1000;
        const g = { kind: 'text', p, text, height, rotation: 0, ...(m.text.align && { align: m.text.align }), ...(m.text.mask && { mask: true }) } as EntityGeometry;
        return [object(g, { label: null })];
      }
      const name = series.get(m.id) ?? m.point?.name;
      if (name) series.set(m.id, textIncrement(name) ?? name);
      return [object({ kind: 'point', p } as EntityGeometry, { ...(code && { attrs: code }), ...(name && { label: name }) })];
    }
  }
}
