// Eylemler and Bilgi kartı (docs/adr/0225 §4–§6; the desktop's `object_info.rs`): a layer's actions filled for an
// object and opened, and its information card's title, rows and text for an object. Templates are split and filled by
// the expression core (model/infoTemplates.ts); `{$enlem}` and `{$boylam}` are the object's label point moved from the
// project's system into WGS 84 the way Koordinat dönüştür moves it.

import type { InfoCard } from '../contracts/generated/InfoCard';
import type { LayerAction } from '../contracts/generated/LayerAction';
import { ENTITY_KIND_LABEL, entityAnchor, type Entity } from '../model/entities';
import { compileTemplate, needsPlace, templateTarget, templateTexts, type CompiledTemplate } from '../model/infoTemplates';
import { crsTransformIn, type System } from '../model/geom/crsTransform';
import { crsSettings, datumChoices, ownSystem } from '../model/projectCrs';
import type { AppContext } from './context';

const WGS84: System = { kind: 'geographic', datum: 'WGS84' };

/** An action for one object: its name, its filled target (null when its template is broken: `error` says why) and what it is. */
export interface ObjectAction {
  readonly name: string;
  readonly target: string | null;
  readonly error: string | null;
  readonly kind: 'address' | 'file' | 'refused';
}

/** An object's card: its title, its rows (an attribute's name and value), its text, and whether it shows attachments and actions. */
export interface ObjectCard {
  readonly title: string;
  readonly rows: readonly (readonly [string, string])[];
  readonly text: string | null;
  readonly attachments: boolean;
  readonly actions: boolean;
  /** Why a template of the card is broken, when one is: what it would show is shown without it. */
  readonly error: string | null;
}

/** The object's label point in WGS 84 as [latitude, longitude]; null without the project's own system or a point. */
export function placeOf(ctx: AppContext, e: Entity): [number, number] | null {
  const settings = crsSettings(ctx.doc.settings);
  const own = ownSystem(settings)?.system;
  if (!own) return null;
  // Where its label sits (the desktop's `entity_anchor`): an area's centroid, a line's middle, a point itself.
  const at = entityAnchor(e);
  if (!at) return null;
  const moved = crsTransformIn(own, WGS84, at, datumChoices(settings));
  return 'error' in moved ? null : [moved.point.y, moved.point.x];
}

/** A template's text for one object: the store's geometry values, its place when the template asks for it. */
function filled(ctx: AppContext, t: CompiledTemplate, e: Entity): string {
  const place = needsPlace(t) ? placeOf(ctx, e) : null;
  const layerName = (id: string) => ctx.doc.layers.get(id)?.name ?? id;
  // The geometry store the object is in: its geometry values ($alan, $merkez_y …) come from its shape there.
  const geometry = { evaluateExpression: (...a: Parameters<typeof ctx.view.evaluateExpression>) => ctx.view.evaluateExpression(...a) };
  return templateTexts(t, [e], layerName, geometry, () => place)[0] ?? '';
}

/** The object's layer's actions (or `actions`, a window's draft), each filled for it. */
export function objectActions(ctx: AppContext, e: Entity, actions: readonly LayerAction[] = ctx.doc.layers.get(e.layerId)?.actions ?? []): ObjectAction[] {
  return actions.map((a) => {
    const t = compileTemplate(a.target);
    if (!t.ok) return { name: a.name, target: null, error: t.error, kind: 'refused' };
    const target = filled(ctx, t.template, e).trim();
    return { name: a.name, target, error: null, kind: templateTarget(target) };
  });
}

/**
 * Opens an object's action: an address in a new tab; a file cannot be opened from the browser, and a target that is no
 * http, https or mailto address and no file's path (`javascript:`, `data:`, `file:` …) is not opened; both are said.
 */
export function runAction(ctx: AppContext, action: ObjectAction): void {
  if (action.error) return void ctx.log.warn(`“${action.name}” eyleminin hedefi: ${action.error}`);
  if (action.kind === 'address' && action.target) {
    window.open(action.target, '_blank', 'noopener');
    return;
  }
  if (action.kind === 'file') return void ctx.log.warn(`“${action.name}” bir dosyayı açar (${action.target}); tarayıcı bilgisayardaki dosyaları açamaz. Masaüstü uygulamasında açın.`);
  ctx.log.warn(`“${action.name}” eyleminin hedefi açılmadı: “${action.target}” http, https ya da mailto adresi ya da dosya yolu değil.`);
}

/**
 * The card of an object: its layer's card's (or `card`, a window's draft) title, rows and text, or the default card (its
 * kind and label, every attribute).
 */
export function objectCard(ctx: AppContext, e: Entity, card: InfoCard | undefined = ctx.doc.layers.get(e.layerId)?.card): ObjectCard {
  let error: string | null = null;
  const text = (source: string | undefined, what: string): string | null => {
    if (source == null) return null;
    const t = compileTemplate(source);
    if (!t.ok) {
      error ??= `${what}: ${t.error}`;
      return null;
    }
    return filled(ctx, t.template, e);
  };
  const kind = ENTITY_KIND_LABEL[e.kind];
  const title = text(card?.title, 'Kartın başlığı') ?? (e.label ? `${kind} ${e.label}` : kind);
  // Every attribute in the drawing's order of names (the desktop's, a sorted map's).
  const fields = card?.fields?.length ? card.fields : Object.keys(e.attrs).sort();
  const rows = fields.map((f) => [f, e.attrs[f] ?? ''] as const);
  return { title, rows, text: text(card?.text, 'Kartın metni'), attachments: card?.attachments !== false, actions: card?.actions !== false, error };
}
