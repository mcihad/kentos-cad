import type { AppContext } from '../app/context';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import { Signal } from '../core/signal';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { LabelStyle } from '../model/layers';
import type { LabelTexts, LabelWanted } from '../model/ops/labelText';
import { CREATE_LABEL, entitiesCreate } from '../product/entitiesCreate';
import { DEFAULT_LABELS } from '../model/labelDefaults';
import type { ViewTransform } from '../viewport/Camera';
import { drawTag, drawTextGhost } from './preview';
import { standardLayerName, writeOnStandardLayer } from './standardLayer';
import type { Tool, ToolPointer } from './Tool';

/**
 * Etiketleri yazıya çevir (docs/adr/0175 §3; Netcad's Etiketleri CAD'e çevir, ArcGIS' Convert Labels To Annotation,
 * QGIS' Extract labels): the layers' labels written as text objects, as a sheet at 1:N writes them. The rule is the
 * shared core's (`ops::label_text`, through the geometry store's `labelTexts`); the desktop's tool is
 * `kentos_interaction::labels_to_text`, and both play `fixtures/interaction/v1/labels-to-text.json`.
 *
 * - Scope, taken when it starts: the selection's labels, else every labelled object's on a visible layer; a label's
 *   style is its layer's, else its kind's default, as the drawing shows it. Texts, dimensions and leaders show none,
 *   and an object whose label a text writes already (docs/adr/0175 §4) has its text for a label.
 * - Options, as Topolojik temizlik's: a number typed is the scale (Ö asks for it; each run starts at the project's
 *   drawing scale); Örtüşenler de (R), Zemin (Z), Katman (K: the standard text layer or the active one) and Nesneye
 *   bağlı (B: the texts know their objects and follow them, `labelOf`, `labelScale`) are kept for the session.
 * - The texts are shown in place, faint, and the counts beside the cursor. Enter, Uygula or a quick right click
 *   writes them through `cad.entities.create` (operation `labels`) in one step, opening the text layer in that step
 *   when the drawing lacks it, selects them and leaves; Esc leaves.
 */

const LABEL = 'Etiketleri yazıya çevir';
/** The standard text layer the texts go to unless the active layer is chosen (docs/adr/0175 §3). */
export const TEXT_LAYER = 'yazi';
/** The texts shown at most. */
const MAX_SHOWN = 2000;
/** The kinds whose own text is what they show: they have no label. */
const NO_LABEL = new Set<Entity['kind']>(['text', 'dimension', 'leader']);

export class LabelsToTextTool implements Tool {
  readonly id = 'labelsToText';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  /** Kept for the session (docs/adr/0175 §3): every label (no thinning), masks, the active layer as the target, linked texts. */
  static every = false;
  static mask = false;
  static active = false;
  static linked = false;
  private readonly ctx: AppContext;
  /** The labels it converts, taken when it starts, in the drawing's order. */
  private wanted: LabelWanted[] = [];
  private whole = true;
  /** The scale's denominator: the project's when the tool starts. */
  private scale = 1000;
  /** Ö: the scale is being typed. */
  private typing = false;
  private plan: { key: string; result: LabelTexts } | null = null;
  /** The finding last said, so a change that finds the same says nothing again. */
  private said = '';
  /** The cursor's world point: the tag beside it says the finding. */
  private hover: Vec2 | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    this.typing = false;
    this.plan = null;
    this.said = '';
    this.scale = this.ctx.doc.settings.plotScale.value;
    if (!this.takeScope()) return void queueMicrotask(() => this.ctx.tools.exit());
    this.tell(`${this.whole ? 'bütün çizimde' : 'seçimde'} ${this.wanted.length} etiket`);
    this.refresh();
  }

  /** The labels to convert (docs/adr/0175 §3); false when there are none, said. */
  private takeScope(): boolean {
    const { doc, selection } = this.ctx;
    this.whole = selection.size === 0;
    const chosen = this.whole ? [...doc.all()] : [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
    this.wanted = chosen.flatMap((e): LabelWanted[] => {
      // A text writes its label already (docs/adr/0175 §4).
      if (!e.label || NO_LABEL.has(e.kind) || !doc.layers.isVisible(e.layerId) || doc.hasLinkedText(doc.uidOf(e.id) ?? '')) return [];
      const style: LabelStyle | undefined = doc.layers.get(e.layerId)?.style.label ?? DEFAULT_LABELS[e.kind];
      return style ? [{ id: e.id, label: e.label, style }] : [];
    });
    if (!this.wanted.length) this.ctx.log.warn(`${LABEL}: ${this.whole ? 'çizimde' : 'seçimde'} yazıya çevrilecek etiket yok.`);
    return this.wanted.length > 0;
  }

  /** The texts for the drawing as it is, kept until the drawing, the scale or the thinning change. */
  private current(): LabelTexts {
    const key = `${this.ctx.doc.revision}|${this.scale}|${+LabelsToTextTool.every}`;
    if (this.plan?.key === key) return this.plan.result;
    const result = this.ctx.view.geometry.labelTexts(this.wanted, this.scale, !LabelsToTextTool.every);
    this.plan = { key, result };
    return result;
  }

  /** Says the finding when it changed; when the tool starts, with its scope and what to do. */
  private tell(scope?: string): void {
    const r = this.current();
    const text = finding(r, this.scale);
    if (!scope && text === this.said) return;
    this.said = text;
    const head = `${LABEL}: ${scope ? `${scope}; ` : ''}${text}`;
    if (r.texts.length) this.ctx.log.info(`${head}.${scope ? ' Enter ile yazın.' : ''}`);
    else this.ctx.log.warn(`${head}; başka bir ölçek yazın.`);
  }

  private refresh(): void {
    const on = (b: boolean) => (b ? 'açık' : 'kapalı');
    if (this.typing) this.prompt.set(`${LABEL}: ölçeği 1:N ya da N olarak yazın (Enter: 1:${this.scale})`);
    else {
      const { doc } = this.ctx;
      const layer = LabelsToTextTool.active ? (doc.layers.get(doc.layers.active.value)?.name ?? '') : standardLayerName(this.ctx, TEXT_LAYER);
      this.prompt.set(
        `${LABEL}: ${finding(this.current(), this.scale)} [Ölçek (Ö): 1:${this.scale} / Örtüşenler de (R): ${on(LabelsToTextTool.every)} / Zemin (Z): ${on(LabelsToTextTool.mask)} / Katman (K): ${layer} / Nesneye bağlı (B): ${on(LabelsToTextTool.linked)} / Uygula (Enter)]`,
      );
    }
    this.ctx.view.requestOverlay();
  }

  /** The tag follows the cursor; a drawing changed under the tool (an undo) is worked out again. */
  pointerMove(p: ToolPointer): void {
    this.hover = p.world;
    if (this.plan && !this.plan.key.startsWith(`${this.ctx.doc.revision}|`)) {
      this.refresh();
      this.tell();
    }
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    if (!this.typing) {
      const flip = OPTION_KEYS[key];
      if (flip) {
        LabelsToTextTool[flip] = !LabelsToTextTool[flip];
        this.refresh();
        this.tell();
        return true;
      }
      if (key === 'Ö' || key === 'O') {
        this.typing = true;
        this.refresh();
        return true;
      }
    }
    const n = readScale(text);
    if (n === null) {
      if (!this.typing) return false;
      this.ctx.log.warn('Ölçeği 1:N ya da N olarak, 1 ya da daha büyük bir tam sayıyla yazın (1:500, 1000).');
      return true;
    }
    this.scale = n;
    this.typing = false;
    this.refresh();
    this.tell();
    return true;
  }

  /** Esc while the scale is typed goes back to the finding; otherwise the tool leaves. */
  cancel(): boolean {
    if (!this.typing) return false;
    this.typing = false;
    this.refresh();
    return true;
  }

  /** Enter: the scale kept while it is typed; otherwise the texts written in one step, and the tool leaves. */
  confirm(): void {
    if (this.typing) {
      this.typing = false;
      return this.refresh();
    }
    const { doc, log } = this.ctx;
    const r = this.current();
    if (!r.texts.length) {
      log.warn(`${LABEL}: yazılacak etiket yok; hiçbir şey değişmedi.`);
      return this.ctx.tools.exit();
    }
    const mask = LabelsToTextTool.mask;
    // Nesneye bağlı: each text knows its object and the scale (docs/adr/0175 §4).
    const linkOf = (item: number) => {
      const uid = LabelsToTextTool.linked ? doc.uidOf(this.wanted[item]?.id ?? -1) : undefined;
      return uid ? { labelOf: uid, labelScale: this.scale } : {};
    };
    const objects = r.texts.map((t) => ({
      geometry: { kind: 'text', p: t.p, text: t.text, height: t.height, rotation: t.rotation, align: t.align, ...(mask && { mask: true }) } as NewGeometry,
      ...linkOf(t.item),
    }));
    const layerId = LabelsToTextTool.active ? doc.layers.active.value : TEXT_LAYER;
    const write = () => entitiesCreate.execute({ doc }, { layerId, objects, operation: 'labels' });
    const result = LabelsToTextTool.active ? write() : writeOnStandardLayer(this.ctx, TEXT_LAYER, 'etiketlerin yazıları', write, CREATE_LABEL.labels);
    if (result.status !== 'completed') {
      if ('error' in result) log.warn(result.error.message);
      return;
    }
    for (const w of result.warnings) log.warn(w.message);
    this.ctx.selection.set(result.output.ids);
    log.success(`${LABEL}: ${r.texts.length} etiket yazıya çevrildi${skipped(r, true)}.`);
    this.ctx.tools.exit();
  }

  /** The texts in place, faint, over their masks when they will have them; the counts beside the cursor. */
  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const r = this.plan?.result;
    if (!r) return;
    const pal = this.ctx.view.palette;
    const mask = LabelsToTextTool.mask ? pal.paper : null;
    for (const t of r.texts.slice(0, MAX_SHOWN)) drawTextGhost(g, view, t, { color: pal.accent, font: pal.drawingFont, mask });
    if (this.hover) drawTag(g, view.worldToScreen(this.hover), tagLines(r, this.scale), pal.accent, pal.labelHalo);
  }
}

/** The letters that turn the options on and off. */
const OPTION_KEYS: Record<string, 'every' | 'mask' | 'active' | 'linked'> = { R: 'every', Z: 'mask', K: 'active', B: 'linked' };

/** `1:500`, `500`: a scale's denominator, a whole number from 1; null for anything else. */
export function readScale(text: string): number | null {
  const m = /^\s*(?:1\s*:\s*)?(\d+)\s*$/.exec(text);
  if (!m) return null;
  const n = Number(m[1]);
  return n >= 1 && Number.isSafeInteger(n) ? n : null;
}

/** `, 1 örtüşen, 2 ölçek dışı etiket atlanacak` (`past`: atlandı); empty when none is. */
function skipped(r: LabelTexts, past: boolean): string {
  const parts = [r.overlapping && `${r.overlapping} örtüşen`, r.outOfScale && `${r.outOfScale} ölçek dışı`, r.small && `${r.small} küçük`].filter(Boolean);
  return parts.length ? `, ${parts.join(', ')} etiket ${past ? 'atlandı' : 'atlanacak'}` : '';
}

/** `1:1000 ölçekte 4 yazı olacak, 1 örtüşen etiket atlanacak`: no suffix on the number, whose sound decides it. */
const finding = (r: LabelTexts, scale: number): string => `1:${scale} ölçekte ${r.texts.length ? `${r.texts.length} yazı olacak` : 'yazı olacak etiket yok'}${skipped(r, false)}`;

/** The finding beside the cursor, a count a line, and what writes it. */
function tagLines(r: LabelTexts, scale: number): string[] {
  const lines = [`${r.texts.length} yazı, 1:${scale}`];
  if (r.overlapping) lines.push(`${r.overlapping} örtüşen atlanır`);
  if (r.outOfScale) lines.push(`${r.outOfScale} ölçek dışı atlanır`);
  if (r.small) lines.push(`${r.small} küçük atlanır`);
  lines.push(r.texts.length ? 'Enter: yaz' : 'Yazılacak etiket yok');
  return lines;
}
