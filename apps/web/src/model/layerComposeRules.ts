import type { BlendMode } from '../contracts/generated/BlendMode';
import type { LayerCompose } from '../contracts/generated/LayerCompose';
import type { LayerMask } from '../contracts/generated/LayerMask';
import type { LayerNode } from './layers';

/**
 * Katman çizimi's rules (docs/adr/0226 §3, §4), the web's copy of `kentos_contracts::layer_compose`: the readers', the
 * command's and the server's words. A mask's targets are other layers' ids; the command checks them when it writes
 * them, and a target the tree no longer has is skipped when drawn, never refused (`maskTargets`).
 */

/** The largest mask growth, paper mm. */
export const MAX_MASK_SIZE = 10;
/** The most layers a mask cuts into. */
export const MAX_MASK_TARGETS = 64;

/** Every mode, in the list's order (`BlendMode::ALL`): its place is its number in the composing shaders. */
export const BLEND_MODES: readonly BlendMode[] = [
  'normal',
  'multiply',
  'screen',
  'overlay',
  'darken',
  'lighten',
  'colorDodge',
  'colorBurn',
  'hardLight',
  'softLight',
  'difference',
  'exclusion',
  'addition',
];

/** A mode's name in the menus (`BlendMode::label`). */
export const BLEND_LABELS: Record<BlendMode, string> = {
  normal: 'Normal',
  multiply: 'Çoğalt',
  screen: 'Ekran',
  overlay: 'Bindirme',
  darken: 'Koyulaştır',
  lighten: 'Açıklaştır',
  colorDodge: 'Renk soldurma',
  colorBurn: 'Renk yakma',
  hardLight: 'Sert ışık',
  softLight: 'Yumuşak ışık',
  difference: 'Fark',
  exclusion: 'Dışlama',
  addition: 'Toplama',
};

/** Rust's `trim` (the White_Space property; JavaScript's `trim` takes U+FEFF too). */
const trim = (s: string): string => s.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');

/** What is wrong with a mask itself, when anything is (`LayerMask::problem`). */
export function maskProblem(m: LayerMask): string | null {
  if (!m.labels && !m.markers) return 'maskenin kaynağı yok: etiketler ya da işaretler seçilmeli';
  if (!Number.isFinite(m.size) || m.size < 0 || m.size > MAX_MASK_SIZE) return `maskenin boyu 0 ile ${MAX_MASK_SIZE} mm arasında olmalı`;
  if (m.targets.length === 0) return 'maskelenecek katman yok';
  if (m.targets.length > MAX_MASK_TARGETS) return `maske en çok ${MAX_MASK_TARGETS} katmanı maskeler`;
  const seen = new Set<string>();
  for (const t of m.targets) {
    if (!trim(t)) return 'maskelenen katmanlar arasında boş kimlik var';
    if (seen.has(t)) return `“${t}” katmanı iki kez maskeleniyor`;
    seen.add(t);
  }
  return null;
}

/** What is wrong with a composition alone, when anything is (`LayerCompose::problem`). */
export function composeProblem(c: LayerCompose): string | null {
  if (c.opacity != null && (!Number.isFinite(c.opacity) || c.opacity < 0 || c.opacity > 1)) return 'saydamlık 0 ile 1 arasında olmalı';
  const mask = c.mask ? maskProblem(c.mask) : null;
  if (mask) return mask;
  if (c.opacity == null && c.blend == null && c.mask == null) return 'katman çizimi boş';
  return null;
}

/** A composition as a layer keeps it (`LayerCompose::normalized`): a copy without what says nothing; null when nothing is left. */
export function composeOf(c: LayerCompose | null | undefined): LayerCompose | null {
  if (!c) return null;
  const out: LayerCompose = {
    ...(c.opacity != null && c.opacity !== 1 && { opacity: c.opacity }),
    ...(c.blend != null && c.blend !== 'normal' && { blend: c.blend }),
    ...(c.mask && { mask: maskOf(c.mask) }),
  };
  return out.opacity === undefined && out.blend === undefined && out.mask === undefined ? null : out;
}

/** A mask as a layer keeps it: a copy, its switches only when on, in the contract's order. */
function maskOf(m: LayerMask): LayerMask {
  return { ...(m.labels && { labels: true }), ...(m.markers && { markers: true }), size: m.size, targets: [...m.targets] };
}

/** What is wrong with the tree's compositions, when anything is (`composes_problem`): on a group, or broken. */
export function composesProblem(tree: readonly LayerNode[]): string | null {
  for (const n of tree) {
    if (n.compose) {
      if (n.type !== 'layer') return `“${n.name}” bir grup; katman çizimi yalnız katmanın olur`;
      const p = composeProblem(n.compose);
      if (p) return `“${n.name}”: ${p}`;
    }
    const below = composesProblem(n.children);
    if (below) return below;
  }
  return null;
}

/** The layers a mask cuts into as drawn (`mask_targets`): its targets the tree has as other layers, in its order. */
export function maskTargets(tree: readonly LayerNode[], own: string, mask: LayerMask): LayerNode[] {
  const find = (ns: readonly LayerNode[], id: string): LayerNode | undefined => {
    for (const n of ns) {
      if (n.id === id) return n;
      const below = find(n.children, id);
      if (below) return below;
    }
    return undefined;
  };
  const out: LayerNode[] = [];
  for (const t of mask.targets) {
    if (t === own) continue;
    const n = find(tree, t);
    if (n && n.type === 'layer') out.push(n);
  }
  return out;
}
