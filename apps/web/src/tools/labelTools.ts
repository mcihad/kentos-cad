import type { AppContext } from '../app/context';
import type { LabelPin } from '../contracts/generated/LabelPin';
import type { LabelPinChange } from '../contracts/generated/LabelPinChange';
import { Signal } from '../core/signal';
import type { Vec2 } from '../model/geometry';
import { labelsPin } from '../product/labelsPin';
import type { ViewTransform } from '../viewport/Camera';
import type { LabelHit } from '../viewport/picking';
import { LABEL_STATE } from '../viewport/placedLabels';
import { parseNumber } from './coordinateInput';
import { drawSelectionBox, drawTag } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { pointFromText } from './tracking';

/**
 * The label tools (docs/adr/0212 §4; the desktop's `kentos_interaction::label_tools`): Etiketi taşı, Etiketi döndür,
 * Etiketi sabitle and Etiketi gizle. Each takes a label the label engine placed in the view (`view.labelAt`, the main
 * view's last labels) and writes its pin with `cad.labels.pin` as one undo step “Etiket”; the pin is the label's
 * middle from its object's anchor, so the label goes with the object.
 *
 * - Etiketi taşı: a click on a label, then its new place (it moves as the pointer does from where it was taken; a
 *   typed point is its new middle).
 * - Etiketi döndür: a click on a label, then its angle: toward the pointer from its middle, made to read (Shift: 15°
 *   steps), or typed in degrees counter-clockwise.
 * - Etiketi sabitle: a click pins a label where it is; a window pins every label whose middle is in it. Çöz (Ç) frees
 *   pinned labels the same way.
 * - Etiketi gizle: a click hides a label. Göster (G) draws the hidden ones faint, and a click on one shows it again.
 *
 * Unplaced labels (drawn while Yerleşmeyen etiketleri göster is on) are taken too: moving one is how it gets a place.
 */

export type LabelAction = 'move' | 'rotate' | 'pin' | 'hide';

/** The tools' ids: their commands are `tool.<id>`. */
export const LABEL_TOOL_ID: Record<LabelAction, string> = { move: 'labelMove', rotate: 'labelRotate', pin: 'labelPin', hide: 'labelHide' };

const LABEL: Record<LabelAction, string> = { move: 'Etiketi taşı', rotate: 'Etiketi döndür', pin: 'Etiketi sabitle', hide: 'Etiketi gizle' };

/** How far a press moves before it is a window, px. */
const DRAG_THRESHOLD = 4;
/** Shift's steps for an angle, degrees. */
const STEP_DEGREES = 15;

interface Press {
  readonly from: Vec2;
  to: Vec2;
  readonly fromWorld: Vec2;
  toWorld: Vec2;
  dragging: boolean;
}

export class LabelTool implements Tool {
  readonly id: string;
  readonly prompt = new Signal('');
  readonly snaps = false;
  private readonly ctx: AppContext;
  private readonly action: LabelAction;
  /** The label taken (Etiketi taşı, Etiketi döndür) and where it was taken. */
  private taken: { hit: LabelHit; from: Vec2 } | null = null;
  /** Çöz for Etiketi sabitle, Göster for Etiketi gizle. */
  private other = false;
  private hover: { hit: LabelHit; at: Vec2 } | null = null;
  private pointer: { at: Vec2; shift: boolean } | null = null;
  private press: Press | null = null;

  constructor(ctx: AppContext, action: LabelAction) {
    this.ctx = ctx;
    this.action = action;
    this.id = LABEL_TOOL_ID[action];
  }

  get cursor(): 'pick' | 'cross' {
    return this.taken ? 'cross' : 'pick';
  }

  get pointCount(): number {
    return this.taken ? 1 : 0;
  }

  activate(): void {
    this.refresh();
  }

  deactivate(): void {
    this.ctx.view.showHiddenLabels(false);
  }

  /** Whether a click takes hidden labels: Etiketi gizle's Göster. */
  private get showing(): boolean {
    return this.action === 'hide' && this.other;
  }

  private refresh(): void {
    const label = LABEL[this.action];
    const toggle = (name: string, key: string) => `${name} (${key})${this.other ? ': açık' : ''}`;
    let text: string;
    if (this.action === 'move') text = this.taken ? 'etiketin yeni yerine tıklayın ya da koordinat yazın' : 'taşınacak etikete tıklayın';
    else if (this.action === 'rotate') text = this.taken ? 'açıyı imleçle gösterin ya da derece olarak yazın [Shift: 15° adım]' : 'döndürülecek etikete tıklayın';
    else if (this.action === 'pin')
      text = `${this.other ? 'serbest bırakılacak' : 'sabitlenecek'} etikete tıklayın ya da pencereyle seçin [${toggle('Çöz', 'Ç')}]`;
    else text = `${this.other ? 'yeniden gösterilecek gizli' : 'gizlenecek'} etikete tıklayın [${toggle('Göster', 'G')}]`;
    this.prompt.set(`${label}: ${text}`);
    this.ctx.view.showHiddenLabels(this.showing);
    this.ctx.view.requestOverlay();
  }

  /** The label under `at`: unplaced ones too while they are drawn, hidden ones under Göster. */
  private hit(at: Vec2): LabelHit | null {
    const all = this.showing || this.ctx.prefs.unplacedLabels.value;
    const hit = this.ctx.view.labelAt(at, this.ctx.prefs.pickAperture.value, all);
    if (!hit) return null;
    const hidden = (hit.state & LABEL_STATE.hidden) !== 0;
    return this.showing === hidden ? hit : null;
  }

  /** The angle Etiketi döndür gives toward `at`, degrees: made to read, Shift's 15° steps. */
  private angleToward(at: Vec2, shift: boolean): number | null {
    if (!this.taken) return null;
    const { hit } = this.taken;
    const dx = at.x - hit.at.x;
    const dy = at.y - hit.at.y;
    if (dx === 0 && dy === 0) return null;
    let a = (Math.atan2(dy, dx) * 180) / Math.PI;
    while (a > 90) a -= 180;
    while (a <= -90) a += 180;
    if (shift) a = Math.round(a / STEP_DEGREES) * STEP_DEGREES;
    return a === 0 ? 0 : a;
  }

  /** Where Etiketi taşı puts the taken label's middle with the pointer at `at`. */
  private moved(at: Vec2): Vec2 | null {
    if (!this.taken) return null;
    const { hit, from } = this.taken;
    return { x: hit.at.x + at.x - from.x, y: hit.at.y + at.y - from.y };
  }

  /** A rule-based layer's class name of a label; undefined for a single label's. */
  private className(hit: LabelHit): string | undefined {
    const e = this.ctx.doc.get(hit.id);
    const labels = e && this.ctx.doc.layers.get(e.layerId)?.style.labels;
    return labels?.mode === 'rules' ? labels.classes?.[hit.cls]?.name : undefined;
  }

  /** A label's place as a pin: its middle from its object's anchor, its angle; null without an anchor. */
  private pinnedAt(hit: LabelHit, middle: Vec2, angle: number): LabelPin | null {
    const anchor = this.ctx.view.labelAnchor(hit.id);
    if (!anchor) return null;
    return { at: { x: middle.x - anchor.x, y: middle.y - anchor.y }, ...(angle !== 0 && { rotation: angle }) };
  }

  /** Writes the changes with `cad.labels.pin` and says `done` (or why not). */
  private write(changes: { hit: LabelHit; pin: LabelPin | null }[], done: string): void {
    const { doc, log } = this.ctx;
    const pins: LabelPinChange[] = [];
    for (const { hit, pin } of changes) {
      const uid = doc.uidOf(hit.id);
      if (!uid) continue;
      const cls = this.className(hit);
      pins.push({ uid, ...(cls !== undefined && { class: cls }), pin });
    }
    if (!pins.length) return;
    const r = labelsPin.execute({ doc }, { pins });
    if (r.status === 'completed') log.success(done);
    else if ('error' in r) log.warn(r.error.message);
  }

  pointerMove(p: ToolPointer): void {
    this.pointer = { at: p.raw, shift: p.shift };
    if (this.press) {
      this.press.to = p.screen;
      this.press.toWorld = p.raw;
      if (!this.press.dragging && Math.hypot(p.screen.x - this.press.from.x, p.screen.y - this.press.from.y) > DRAG_THRESHOLD) this.press.dragging = true;
    }
    const hit = this.taken ? null : this.hit(p.raw);
    this.hover = hit ? { hit, at: p.raw } : null;
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.taken) {
      if (this.action === 'move') {
        const middle = this.moved(p.raw);
        if (middle) this.put(middle);
      } else {
        const angle = this.angleToward(p.raw, p.shift);
        if (angle !== null) this.turn(angle);
      }
      return;
    }
    if (this.action === 'move' || this.action === 'rotate') {
      const hit = this.hit(p.raw);
      if (!hit) return void this.ctx.log.warn(`Çizimde ${this.action === 'move' ? 'taşınacak' : 'döndürülecek'} bir etikete tıklayın.`);
      this.taken = { hit, from: p.raw };
      this.hover = null;
      return this.refresh();
    }
    this.press = { from: p.screen, to: p.screen, fromWorld: p.raw, toWorld: p.raw, dragging: false };
  }

  pointerUp(p: ToolPointer): void {
    const press = this.press;
    this.press = null;
    if (!press) return;
    const hits =
      press.dragging && this.action === 'pin'
        ? this.ctx.view.labelsIn(press.fromWorld, press.toWorld, this.ctx.prefs.unplacedLabels.value).filter((h) => (h.state & LABEL_STATE.hidden) === 0)
        : [this.hit(p.raw)].filter((h): h is LabelHit => h !== null);
    this.ctx.view.requestOverlay();
    if (!hits.length) {
      const text =
        this.action === 'pin'
          ? this.other
            ? 'Serbest bırakılacak sabit bir etikete tıklayın ya da pencereyle seçin.'
            : 'Sabitlenecek bir etikete tıklayın ya da pencereyle seçin.'
          : this.other
            ? 'Yeniden gösterilecek gizli bir etikete tıklayın (soluk çizilenler).'
            : 'Gizlenecek bir etikete tıklayın.';
      return void this.ctx.log.warn(text);
    }
    if (this.action === 'pin') this.pin(hits);
    else this.hide(hits[0]);
  }

  input(text: string): boolean {
    const key = text.trim().toLocaleUpperCase('tr-TR');
    if ((this.action === 'pin' && (key === 'Ç' || key === 'C')) || (this.action === 'hide' && key === 'G')) {
      this.other = !this.other;
      this.refresh();
      return true;
    }
    if (!this.taken) return false;
    if (this.action === 'move') {
      const p = pointFromText(this.ctx, text, null, null);
      if (!p) return false;
      this.put(p);
      return true;
    }
    if (this.action === 'rotate') {
      const n = parseNumber(text);
      if (n === null) return false;
      this.turn(n);
      return true;
    }
    return false;
  }

  /** Enter and a right click end the tool. */
  confirm(): void {
    this.ctx.tools.exit();
  }

  /** Esc lets a taken label go; with none, the tool leaves. */
  cancel(): boolean {
    this.press = null;
    if (!this.taken) return false;
    this.taken = null;
    this.refresh();
    return true;
  }

  /** Ctrl+Z with a label taken lets it go; otherwise the drawing is undone. */
  undoStep(): boolean {
    return this.cancel();
  }

  /** Etiketi taşı: the taken label's middle at `middle`, its angle kept (a curved label's level). */
  private put(middle: Vec2): void {
    const taken = this.taken;
    this.taken = null;
    if (taken) {
      const { hit } = taken;
      const pin = this.pinnedAt(hit, middle, hit.state & LABEL_STATE.curved ? 0 : hit.angle);
      if (pin) this.write([{ hit, pin }], 'Etiket taşındı ve sabitlendi.');
    }
    this.refresh();
  }

  /** Etiketi döndür: the taken label turned `angle` degrees where it is. */
  private turn(angle: number): void {
    const taken = this.taken;
    this.taken = null;
    if (taken) {
      const pin = this.pinnedAt(taken.hit, taken.hit.at, angle);
      if (pin) this.write([{ hit: taken.hit, pin }], 'Etiket döndürüldü ve sabitlendi.');
    }
    this.refresh();
  }

  /** Etiketi sabitle: the labels pinned where they are, or (Çöz) freed. */
  private pin(hits: readonly LabelHit[]): void {
    const changes: { hit: LabelHit; pin: LabelPin | null }[] = [];
    for (const hit of hits) {
      const pinned = (hit.state & LABEL_STATE.pinned) !== 0;
      if (this.other) {
        if (pinned) changes.push({ hit, pin: null });
      } else if (!pinned) {
        const pin = this.pinnedAt(hit, hit.at, hit.state & LABEL_STATE.curved ? 0 : hit.angle);
        if (pin) changes.push({ hit, pin });
      }
    }
    if (!changes.length) return void this.ctx.log.info(this.other ? 'Seçilen etiketlerin hiçbiri sabit değil.' : 'Seçilen etiketler zaten sabit.');
    const n = changes.length;
    const done = this.other ? (n === 1 ? 'Etiket serbest bırakıldı.' : `${n} etiket serbest bırakıldı.`) : n === 1 ? 'Etiket sabitlendi.' : `${n} etiket sabitlendi.`;
    this.write(changes, done);
  }

  /** Etiketi gizle: the label hidden (its place kept), or (Göster) shown again. */
  private hide(hit: LabelHit): void {
    const e = this.ctx.doc.get(hit.id);
    const cls = this.className(hit);
    const now = e?.labelPins?.find((p) => p.class === cls);
    let pin: LabelPin | null;
    if (this.other) {
      // Shown again: a pin with a place keeps it, one only hiding goes.
      pin = now?.at ? { at: now.at, ...(now.rotation !== undefined && { rotation: now.rotation }) } : null;
    } else {
      pin = now?.at ? { at: now.at, ...(now.rotation !== undefined && { rotation: now.rotation }), hidden: true } : { hidden: true };
    }
    this.write([{ hit, pin }], this.other ? 'Etiket yeniden gösteriliyor.' : 'Etiket gizlendi.');
  }

  /** The dashed box round a label standing at `middle` turned `angle` degrees. */
  private frame(g: CanvasRenderingContext2D, view: ViewTransform, hit: LabelHit, middle: Vec2, angle: number, dashed: boolean): void {
    const s = view.worldToScreen(middle);
    g.save();
    g.translate(s.x, s.y);
    g.rotate((-angle * Math.PI) / 180);
    if (dashed) g.setLineDash([4, 3]);
    g.strokeStyle = this.ctx.view.palette.accent;
    g.lineWidth = 1.5;
    g.strokeRect(-hit.w / 2 - 3, -hit.h / 2 - 3, hit.w + 6, hit.h + 6);
    g.restore();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    if (this.press?.dragging && this.action === 'pin') drawSelectionBox(g, this.press.from, this.press.to, pal.snap);
    if (this.taken) {
      const { hit } = this.taken;
      this.frame(g, view, hit, hit.at, hit.angle, true);
      const at = this.pointer?.at;
      if (!at) return;
      if (this.action === 'move') {
        const middle = this.moved(at);
        if (middle) this.frame(g, view, hit, middle, hit.angle, false);
      } else {
        const angle = this.angleToward(at, this.pointer?.shift ?? false);
        if (angle === null) return;
        this.frame(g, view, hit, hit.at, angle, false);
        drawTag(g, view.worldToScreen(at), [`${angle.toFixed(1)}°`], pal.accent, pal.labelHalo);
      }
      return;
    }
    if (!this.hover) return;
    const { hit, at } = this.hover;
    this.frame(g, view, hit, hit.at, hit.angle, true);
    const e = this.ctx.doc.get(hit.id);
    const layer = (e && this.ctx.doc.layers.get(e.layerId)?.name) ?? '';
    const cls = this.className(hit);
    let first = cls !== undefined ? `${layer}, ${cls}` : layer;
    if (hit.state & LABEL_STATE.pinned) first += ' (sabit)';
    else if (hit.state & LABEL_STATE.unplaced) first += ' (yerleşmedi)';
    const click =
      this.action === 'move'
        ? 'Tıklayın: taşı'
        : this.action === 'rotate'
          ? 'Tıklayın: döndür'
          : this.action === 'pin'
            ? this.other
              ? 'Tıklayın: serbest bırak'
              : 'Tıklayın: sabitle'
            : this.other
              ? 'Tıklayın: göster'
              : 'Tıklayın: gizle';
    drawTag(g, view.worldToScreen(at), [first, click], pal.accent, pal.labelHalo);
  }
}
