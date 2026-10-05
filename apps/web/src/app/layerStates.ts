import type { LayerState } from '../contracts/generated/LayerState';
import type { LayerStyle } from '../model/layers';
import { captureLayerState, layerStateChanges, layerStateMatches, type LayerStateParts } from '../model/layerStates';
import { treeLocked } from '../ui/layers/treeRights';
import type { AppContext } from './context';

/**
 * Katman durumları (docs/adr/0177 §4; the desktop's `layer_states.rs`): named records of the layers' visibility, and
 * when asked their locks and styles, kept in the project's settings (`layerStates`, KCAD schema 19). Saving, updating,
 * renaming and removing one change the project's settings: an edit, not an undo step; refused where the project's
 * settings may not change (a cloud project without project.edit). Applying one changes the visibility and locks as
 * the eye and the lock do (the tree's own changes, the user's own) and the styles in one undo step “Katman durumu:
 * <ad>”.
 */

/** Why the layer states may not change here, in the words the window and the commands say it. */
export const STATES_LOCKED = 'Bu projede katman durumlarını değiştirme yetkiniz yok (project.edit); proje sahibinden ya da yöneticisinden isteyin.';

/** Why the layer states may not change in the open drawing, or null when they may. */
export function statesLocked(ctx: AppContext): string | null {
  return treeLocked(ctx) ? STATES_LOCKED : null;
}

/** What a saved state keeps: locks when one of its nodes has its lock, styles when one has its style. */
export function partsOf(state: LayerState): LayerStateParts {
  return { locks: state.nodes.some((n) => n.locked !== undefined), styles: state.nodes.some((n) => n.style !== undefined) };
}

/** What a state keeps, in words: “görünürlük”, “görünürlük ve stil”, “görünürlük, kilit ve stil”. */
export function partsText(parts: LayerStateParts): string {
  const words = ['görünürlük', ...(parts.locks ? ['kilit'] : []), ...(parts.styles ? ['stil'] : [])];
  return words.length === 1 ? words[0] : `${words.slice(0, -1).join(', ')} ve ${words.at(-1)}`;
}

/** The first “Durum n” no state is named. */
export function nextStateName(states: readonly LayerState[]): string {
  for (let n = 1; ; n++) if (!states.some((s) => s.name === `Durum ${n}`)) return `Durum ${n}`;
}

/** The first “durum-n” no state has as its id. */
function nextStateId(states: readonly LayerState[]): string {
  for (let n = 1; ; n++) if (!states.some((s) => s.id === `durum-${n}`)) return `durum-${n}`;
}

/** Why `name` may not name a state (other than `self`), or null. */
function nameRefused(states: readonly LayerState[], name: string, self?: string): string | null {
  if (!name) return 'Durumun adını yazın.';
  const same = states.find((s) => s.name.trim() === name && s.id !== self);
  return same ? `“${name}” adında bir katman durumu var; başka bir ad yazın ya da o durumu seçip Güncelle'ye basın.` : null;
}

/** Whether the drawing's layers are in the state now (the menu's mark). */
export function stateMatches(ctx: AppContext, state: LayerState): boolean {
  return layerStateMatches(ctx.doc.layers.tree, state);
}

/** Saves the tree as a new state named `name` keeping `parts`; its id, or null when refused (said). */
export function saveLayerState(ctx: AppContext, name: string, parts: LayerStateParts): string | null {
  const { doc, log } = ctx;
  const locked = statesLocked(ctx);
  if (locked) return (log.warn(locked), null);
  const states = doc.settings.layerStates.value;
  const trimmed = name.trim();
  const refused = nameRefused(states, trimmed);
  if (refused) return (log.warn(refused), null);
  const state = captureLayerState(doc.layers.tree, nextStateId(states), trimmed, parts);
  doc.settings.layerStates.set([...states, state]);
  log.success(`“${trimmed}” katman durumu kaydedildi: ${partsText(parts)}.`);
  return state.id;
}

/** Saves the tree at once as “Durum n” with its visibility and styles (the panel's Yeni durum kaydet). */
export function quickSaveLayerState(ctx: AppContext): string | null {
  return saveLayerState(ctx, nextStateName(ctx.doc.settings.layerStates.value), { locks: false, styles: true });
}

/** The state `id` saved again from the tree as it is now, keeping `parts`. Whether it did. */
export function updateLayerState(ctx: AppContext, id: string, parts: LayerStateParts): boolean {
  const { doc, log } = ctx;
  const locked = statesLocked(ctx);
  if (locked) return (log.warn(locked), false);
  const states = doc.settings.layerStates.value;
  const old = states.find((s) => s.id === id);
  if (!old) return false;
  const state = captureLayerState(doc.layers.tree, id, old.name, parts);
  doc.settings.layerStates.set(states.map((s) => (s.id === id ? state : s)));
  log.success(`“${old.name}” katman durumu şimdiki hâlle güncellendi: ${partsText(parts)}.`);
  return true;
}

/** The state `id` named `name`. Whether it did. */
export function renameLayerState(ctx: AppContext, id: string, name: string): boolean {
  const { doc, log } = ctx;
  const locked = statesLocked(ctx);
  if (locked) return (log.warn(locked), false);
  const states = doc.settings.layerStates.value;
  const old = states.find((s) => s.id === id);
  if (!old) return false;
  const trimmed = name.trim();
  if (trimmed === old.name) return true;
  const refused = nameRefused(states, trimmed, id);
  if (refused) return (log.warn(refused), false);
  doc.settings.layerStates.set(states.map((s) => (s.id === id ? { ...s, name: trimmed } : s)));
  log.success(`“${old.name}” katman durumunun adı “${trimmed}” oldu.`);
  return true;
}

/** Removes the state `id`. Whether it did. */
export function removeLayerState(ctx: AppContext, id: string): boolean {
  const { doc, log } = ctx;
  const locked = statesLocked(ctx);
  if (locked) return (log.warn(locked), false);
  const states = doc.settings.layerStates.value;
  const old = states.find((s) => s.id === id);
  if (!old) return false;
  doc.settings.layerStates.set(states.filter((s) => s.id !== id));
  log.success(`“${old.name}” katman durumu silindi.`);
  return true;
}

/**
 * Applies the state `id`: the visibility and locks of the nodes it names that the tree still has become what it kept
 * (the tree's own changes, as the eye and the lock), their styles too in one undo step “Katman durumu: <ad>”; what it
 * changed is said, and its nodes the drawing no longer has.
 */
export function applyLayerState(ctx: AppContext, id: string): void {
  const { doc, log } = ctx;
  const state = doc.settings.layerStates.value.find((s) => s.id === id);
  if (!state) return;
  const layers = doc.layers;
  const c = layerStateChanges(layers.tree, state);
  for (const [node, visible] of c.visible) layers.setVisible(node, visible);
  for (const [node] of c.locked) layers.toggleLocked(node);
  if (c.styles.length)
    doc.transact(`Katman durumu: ${state.name}`, () => {
      for (const [node, style] of c.styles) {
        const before = layers.get(node)?.style ?? {};
        // The whole style: a key the state's style lacks goes.
        const gone = Object.fromEntries(Object.keys(before).map((k) => [k, undefined])) as Partial<LayerStyle>;
        doc.setLayerStyle(node, { ...gone, ...structuredClone(style) }, `Katman durumu: ${state.name}`);
      }
    });
  const parts = [
    ...(c.visible.length ? [`${c.visible.length} görünürlük`] : []),
    ...(c.locked.length ? [`${c.locked.length} kilit`] : []),
    ...(c.styles.length ? [`${c.styles.length} stil`] : []),
  ];
  if (parts.length) log.success(`“${state.name}” katman durumu uygulandı: ${parts.join(', ')} değişti.`);
  else log.info(`Katmanlar zaten “${state.name}” durumunda.`);
  if (c.missing) log.info(`Durumdaki ${c.missing} katman ya da grup artık çizimde yok; atlandı.`);
}
