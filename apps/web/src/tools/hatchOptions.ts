import type { AppContext } from '../app/context';
import type { HatchPattern } from '../model/entities';
import { HATCH_COLOURS, hatchChoiceNamed, hatchChoices, hatchColour, hatchToolPattern, type HatchChoice } from '../model/ops/hatchPatterns';
import { parseNumber } from './coordinateInput';
import type { OptionChoice } from './Tool';

/**
 * Tarama's and Çoklu tara's pattern options (docs/adr/0186 §4): Desen (D) with its menu, Ölçek (Ö) and Açı (Ç),
 * İkinci renk (R) and Ters (T) for a gradient, Adalar (A), İlişkili (İ) and Yazılar (Y); the values asked in the
 * command line, and the pattern a hatch is written with. The choices and the pattern are the core's
 * (`tools::hatch`); the desktop's are kentos_interaction's hatch_options.rs. The session's values live as long as
 * the page (the desktop's `Memory`).
 */

/** What an option asks for while it is asked. */
export type HatchAsking = 'scale' | 'angle' | 'colour';

/** The session's options, shared by Tarama and Çoklu tara. */
export const hatchSession = {
  /** The library's ANSI31 (the core's `DEFAULT_CHOICE`). */
  choice: 3,
  scale: 1,
  angle: 0,
  color2: '#FFFFFF',
  inverted: false,
  byLines: false,
  islands: true,
  assoc: true,
  texts: false,
};

/** The largest Ölçek taken. */
const MOST_SCALE = 10_000;

let list: readonly HatchChoice[] | null = null;

/** Desen's choices, read from the core once. */
export function choiceList(): readonly HatchChoice[] {
  return (list ??= hatchChoices());
}

/** The session's choice. */
export function currentChoice(): HatchChoice {
  const l = choiceList();
  return l[Math.min(hatchSession.choice, l.length - 1)];
}

/** The pattern a hatch is written with at the drawing's plot scale (1:N). */
export function hatchPattern(ctx: AppContext): HatchPattern {
  const s = hatchSession;
  return hatchToolPattern({ choice: s.choice, scale: s.scale, angle: s.angle, color2: s.color2, inverted: s.inverted, plotScale: ctx.doc.settings.plotScale.value });
}

const on = (b: boolean) => (b ? 'açık' : 'kapalı');

/** Desen and what it reads: İkinci renk and Ters for a gradient, Ölçek for lines and patterns, Açı for all but Dolu. */
export function patternOptions(): string[] {
  const c = currentChoice();
  const s = hatchSession;
  const out = [`Desen (D): ${c.name}`];
  if (c.kind === 'gradient') out.push(`İkinci renk (R): ${s.color2}`, `Ters (T): ${on(s.inverted)}`);
  else if (c.kind !== 'solid') out.push(`Ölçek (Ö): ${s.scale}`);
  if (c.kind !== 'solid') out.push(`Açı (Ç): ${s.angle}°`);
  return out;
}

/** Adalar, İlişkili (where the hatch can follow its objects) and Yazılar. */
export function regionOptions(tie: boolean): string[] {
  const s = hatchSession;
  return [`Adalar (A): ${s.islands ? 'taranmaz' : 'taranır'}`, ...(tie ? [`İlişkili (İ): ${on(s.assoc)}`] : []), `Yazılar (Y): ${s.texts ? 'boş bırakılır' : 'taranır'}`];
}

/** What a stage that asks says, after the tool's label. */
export function askingText(asking: HatchAsking): string {
  if (asking === 'scale') return 'desenin kâğıttaki ölçeğini yazın (1: kitaplığın büyüklüğü)';
  if (asking === 'angle') return 'desenin açısını derece olarak yazın';
  return 'ikinci rengi yazın (#RRGGBB ya da renk adı) ya da menüden seçin [İkinci renk (R)]';
}

/** The options' keys: whether `key` was one of them (a key that asks for a value returns what it asks). */
export function hatchOption(key: string, tie: boolean): { taken: boolean; asking?: HatchAsking } {
  const kind = currentChoice().kind;
  const s = hatchSession;
  const gradient = kind === 'gradient';
  if (key === 'D') s.choice = (s.choice + 1) % choiceList().length;
  else if (key === 'Ö' && !gradient && kind !== 'solid') return { taken: true, asking: 'scale' };
  else if (key === 'Ç' && kind !== 'solid') return { taken: true, asking: 'angle' };
  else if (key === 'R' && gradient) return { taken: true, asking: 'colour' };
  else if (key === 'T' && gradient) s.inverted = !s.inverted;
  else if (key === 'A') s.islands = !s.islands;
  else if (key === 'İ' && tie) s.assoc = !s.assoc;
  else if (key === 'Y') s.texts = !s.texts;
  else return { taken: false };
  return { taken: true };
}

/** A value typed for what is asked: whether it was taken (a wrong one is said and asked again). */
export function hatchAnswer(ctx: AppContext, asking: HatchAsking, text: string): boolean {
  const t = text.trim();
  if (asking === 'colour') {
    const c = hatchColour(t);
    if (c) hatchSession.color2 = c;
    else ctx.log.warn(`İkinci renk #RRGGBB biçiminde ya da bir renk adı olmalı (beyaz, siyah, kırmızı …); “${t}” yazıldı.`);
    return !!c;
  }
  const n = parseNumber(t);
  if (asking === 'scale') {
    if (n !== null && n > 0 && n <= MOST_SCALE) hatchSession.scale = n;
    else ctx.log.warn(`Ölçek sıfırdan büyük, en çok ${MOST_SCALE} bir sayı olmalı; “${t}” yazıldı.`);
    return n !== null && n > 0 && n <= MOST_SCALE;
  }
  if (n !== null && Number.isFinite(n)) hatchSession.angle = n;
  else ctx.log.warn(`Açı derece olarak bir sayı olmalı; “${t}” yazıldı.`);
  return n !== null && Number.isFinite(n);
}

/** A pattern's name typed while nothing is asked (“ansi31”, “dolu”): whether it was one. */
export function typedChoice(text: string): boolean {
  const k = hatchChoiceNamed(text);
  if (k === null) return false;
  hatchSession.choice = k;
  return true;
}

/** Desen's and İkinci renk's menus. */
export function hatchChoicesOf(key: string): readonly OptionChoice[] | null {
  if (key === 'D') return choiceList().map((c, k) => ({ label: c.label, typed: c.typed, icon: c.icon, preview: c.preview, checked: k === hatchSession.choice }));
  if (key === 'R') return HATCH_COLOURS.map(([name, hex]) => ({ label: `${name} (${hex})`, typed: name.toLocaleLowerCase('tr-TR'), checked: hex === hatchSession.color2 }));
  return null;
}

/** One of the menus' entries chosen: whether it was taken. */
export function chooseHatchOption(key: string, typed: string): boolean {
  if (key === 'D') return typedChoice(typed);
  if (key === 'R') {
    const c = hatchColour(typed);
    if (c) hatchSession.color2 = c;
    return !!c;
  }
  return false;
}

/**
 * A hatch's pattern given another of Desen's choices in Öznitelikler (docs/adr/0186 §7): its own Ölçek and Açı
 * carried over as the tool's (a pattern's scale or user lines' spacing on the paper, user lines' turn past 45°), its
 * gradient's colour and way. The desktop's is hatch_options.rs' `rechosen`.
 */
export function rechosenPattern(p: HatchPattern, choice: number, plotScale: number): HatchPattern {
  const metres = plotScale / 1000;
  const scale = p.type === 'pattern' ? (p.scale ?? metres) / metres : p.type === 'lines' || p.type === 'cross' ? p.spacing / (3 * metres) : 1;
  const angle = p.type === 'lines' || p.type === 'cross' ? p.angle - 45 : p.type === 'solid' ? 0 : p.angle;
  return hatchToolPattern({ choice, scale: Number.isFinite(scale) && scale > 0 ? scale : 1, angle, color2: p.gradient?.color2 ?? '#FFFFFF', inverted: p.gradient?.inverted === true, plotScale });
}

/** The choice a hatch's pattern is: its library name, else its kind's; null for a pattern the library has not. */
export function choiceOf(p: HatchPattern): number | null {
  const list = choiceList();
  if (p.type === 'pattern') return p.name ? hatchChoiceNamed(p.name) : null;
  if (p.type === 'gradient') {
    const icon = { linear: 'hatchGradientLinear', cylinder: 'hatchGradientCylinder', spherical: 'hatchGradientSpherical' }[p.gradient?.shape ?? 'linear'];
    const k = list.findIndex((c) => c.icon === icon);
    return k < 0 ? null : k;
  }
  return p.type === 'lines' ? 0 : p.type === 'cross' ? 1 : 2;
}
