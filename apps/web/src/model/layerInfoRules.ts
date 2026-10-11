import type { InfoCard } from '../contracts/generated/InfoCard';
import type { LayerAction } from '../contracts/generated/LayerAction';
import type { LayerNode } from './layers';

/**
 * Eylemler and Bilgi kartı's rules (docs/adr/0225 §4, §5), the web's copy of `kentos_contracts::layer_info`: the
 * readers', the commands' and the server's words. Whether a template splits and its expressions compile is the
 * command's (`model/infoTemplates.ts`).
 */

/** The most actions a layer has. */
export const MAX_ACTIONS = 32;
/** The longest action name, in characters. */
export const MAX_ACTION_NAME = 100;
/** The longest action target, in characters. */
export const MAX_ACTION_TARGET = 4_096;
/** The longest card title, in characters. */
export const MAX_CARD_TITLE = 1_000;
/** The longest card text, in characters. */
export const MAX_CARD_TEXT = 10_000;
/** The most attributes a card names. */
export const MAX_CARD_FIELDS = 200;

const CONTROL = /\p{Cc}/u;
const CONTROL_BUT_LINE = /[^\P{Cc}\n]/u;
const chars = (s: string): number => [...s].length;
/** Rust's `trim` (the White_Space property; JavaScript's `trim` takes U+FEFF too). */
const trim = (s: string): string => s.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');

/** What is wrong with a layer's actions, when anything is (`actions_problem`). */
export function actionsProblem(actions: readonly LayerAction[]): string | null {
  if (actions.length > MAX_ACTIONS) return `katmanın ${actions.length} eylemi var; en çok ${MAX_ACTIONS}`;
  const seen = new Set<string>();
  for (const a of actions) {
    const name = trim(a.name);
    if (!name || name !== a.name) return 'eylemin adı boş ya da başında veya sonunda boşluk var';
    if (chars(a.name) > MAX_ACTION_NAME || CONTROL.test(a.name)) return `“${name}” eyleminin adı ${MAX_ACTION_NAME} karakterden uzun ya da denetim karakteri içeriyor`;
    const key = name.toLowerCase();
    if (seen.has(key)) return `“${name}” adlı iki eylem var`;
    seen.add(key);
    if (!trim(a.target)) return `“${name}” eyleminin hedefi boş`;
    if (chars(a.target) > MAX_ACTION_TARGET || CONTROL.test(a.target)) return `“${name}” eyleminin hedefi ${MAX_ACTION_TARGET} karakterden uzun ya da denetim karakteri içeriyor`;
  }
  return null;
}

/** What is wrong with a card, when anything is (`InfoCard::problem`). */
export function cardProblem(c: InfoCard): string | null {
  if (c.title != null && (!trim(c.title) || chars(c.title) > MAX_CARD_TITLE || CONTROL.test(c.title))) return `kartın başlığı boş, ${MAX_CARD_TITLE} karakterden uzun ya da satır sonu içeriyor`;
  if (c.text != null && (!trim(c.text) || chars(c.text) > MAX_CARD_TEXT || CONTROL_BUT_LINE.test(c.text))) return `kartın metni boş, ${MAX_CARD_TEXT} karakterden uzun ya da denetim karakteri içeriyor`;
  const fields = c.fields ?? [];
  if (fields.length > MAX_CARD_FIELDS) return `kart ${fields.length} alan gösteriyor; en çok ${MAX_CARD_FIELDS}`;
  const seen = new Set<string>();
  for (const f of fields) {
    if (!trim(f)) return 'kartın alanlarında boş ad var';
    if (seen.has(f)) return `kart “${f}” alanını iki kez gösteriyor`;
    seen.add(f);
  }
  return null;
}

/** What is wrong with the tree's actions and cards, when anything is (`infos_problem`): on a group or a service layer, broken. */
export function infosProblem(tree: readonly LayerNode[]): string | null {
  for (const n of tree) {
    const has = !!n.actions?.length || n.card !== undefined;
    if (has && (n.type !== 'layer' || n.service)) return `“${n.name}”: eylemler ve bilgi kartı yalnız nesneleri olan katmanda olur`;
    const p = actionsProblem(n.actions ?? []) ?? (n.card ? cardProblem(n.card) : null);
    if (p) return `“${n.name}”: ${p}`;
    const below = infosProblem(n.children);
    if (below) return below;
  }
  return null;
}

/** A card as a layer keeps it: a copy with only what is set, the switches only off, in the contract's order. */
export function cardOf(c: InfoCard): InfoCard {
  return {
    ...(c.title != null && { title: c.title }),
    ...(c.fields?.length && { fields: [...c.fields] }),
    ...(c.text != null && { text: c.text }),
    ...(c.attachments === false && { attachments: false }),
    ...(c.actions === false && { actions: false }),
  };
}

/** Actions as a layer keeps them: copies. */
export const actionsOf = (actions: readonly LayerAction[]): LayerAction[] => actions.map((a) => ({ name: a.name, target: a.target }));
