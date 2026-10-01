import type { AppContext } from '../app/context';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { Entity, PointEntity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { textIncrement } from '../model/textEdit';
import { entitiesEdit } from '../product/entitiesEdit';
import { entitiesSet } from '../product/entitiesSet';
import { pointCreate } from '../product/pointCreate';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { PointInputTool } from './drawTools';
import { drawTag } from './preview';
import type { ToolPointer } from './Tool';

/**
 * Nokta (docs/adr/0152 §2–§3): survey points, each with its name (its label), code (the `Kod` attribute) and
 * elevation. The desktop's tool is `kentos_interaction::point`; both play `fixtures/interaction/v1/survey-points.json`.
 *
 * - Ad (A) and Kod (K) are asked in the text field Yazı and Kılavuz use (letters typed over the drawing would start
 *   a command); an empty Enter clears them. Kot (Z) is typed as a number; an empty Enter clears it. All three are
 *   kept for the session, and the name moves on by Yazı's Artır after each point written (101 → 102, 101/12 →
 *   101/13; a name not ending in a number stays).
 * - Every point, clicked or typed, is written through `cad.point.create`: its own object and undo step. Where a
 *   point is already (1 µm), the tool asks first: Düzelt gives that point the name, code and elevation given (one
 *   step “Nokta düzelt”), Ekle writes the new one too, Esc passes; the name moves on after Düzelt and Ekle.
 * - Ctrl+Z takes the newest point (or Düzelt) back while nothing else changed the drawing, and gives its name back.
 * - Without a name, a code or an elevation it is the plain point tool it was (ADR 0032).
 */

const LABEL = 'Nokta';
/** “The same place”, metres (ADR 0142). */
const SAME = 1e-6;
/** The longest name or code: what the desktop's session memory holds (`Name::MAX_CHARS`). */
export const MAX_CHARS = 60;

type Stage = 'points' | 'z' | 'field' | 'question';

/** A point already where the new one goes (the first in the drawing's order), or null. */
export function pointAt(doc: { all(): Iterable<Entity> }, p: Vec2): PointEntity | null {
  for (const e of doc.all()) if (e.kind === 'point' && Math.hypot(e.p.x - p.x, e.p.y - p.y) <= SAME) return e;
  return null;
}

/** Ad (A) and Kod (K) as they are asked (docs/adr/0152 §2). */
const FIELDS = {
  A: { what: 'Noktanın adı', hint: 'Enter: kaydet · boş Enter: adsız · Esc: vazgeç' },
  K: { what: 'Noktaların kodu', hint: 'Enter: kaydet · boş Enter: kodsuz · Esc: vazgeç' },
} as const;

/**
 * Asks Ad (A) or Kod (K) in the text field by the cursor (or the view's middle), the old value in it: Enter keeps what
 * is typed, an empty Enter clears it, Esc keeps the old. Köşelere nokta asks them too: they are Nokta's (§5). The field
 * opens once the key or the chip that asked has given the drawing its focus back; `done` runs after its answer, with
 * whether the value changed.
 */
export function askPointField(ctx: AppContext, key: 'A' | 'K', near: Vec2 | null, done: (changed: boolean) => void): void {
  const { view, log } = ctx;
  const f = FIELDS[key];
  const b = view.camera.visibleBounds();
  const at = near ?? { x: (b.minX + b.maxX) / 2, y: (b.minY + b.maxY) / 2 };
  const S = SurveyPointTool;
  const end = (typed: string | null) => {
    const v = typed?.trim() ?? null;
    let changed = false;
    if (v !== null && [...v].length > MAX_CHARS) log.warn(`${f.what} en çok ${MAX_CHARS} harf olabilir.`);
    else if (v !== null) {
      changed = v !== (key === 'A' ? S.next : S.code);
      if (key === 'A') S.next = v;
      else S.code = v;
    }
    done(changed);
    view.focus();
  };
  queueMicrotask(() =>
    view.requestTextInput({
      at,
      // Readable at any zoom: 14 px on the screen.
      height: view.worldTolerance(14),
      rotation: 0,
      initial: key === 'A' ? S.next : S.code,
      placeholder: f.what,
      hint: f.hint,
      commit: (v) => end(v),
      empty: () => end(''),
      cancel: () => end(null),
    }),
  );
}

/** The options' values in a prompt: `Ad (A): 101 / Kod (K): SN`. */
export function pointOptions(): string {
  const S = SurveyPointTool;
  return `Ad (A): ${S.next || '—'} / Kod (K): ${S.code || '—'}`;
}

export class SurveyPointTool extends PointInputTool {
  readonly id = 'point';
  protected readonly label = LABEL;
  /** The next point's name, the code and the elevation, kept for the session (docs/adr/0152 §2). */
  static next = '';
  static code = '';
  static z: number | null = null;
  private stage: Stage = 'points';
  /** The question's place and the point already there. */
  private asked: { at: Vec2; there: PointEntity } | null = null;
  /** The name the newest step took, given back when Ctrl+Z takes the step back; null when there is none to give. */
  private givenBack: string | null = null;

  override activate(): void {
    this.stage = 'points';
    this.asked = null;
    this.givenBack = null;
    super.activate();
  }

  protected promptFor(): string {
    const S = SurveyPointTool;
    switch (this.stage) {
      case 'z':
        return 'noktaların kotunu yazın, metre (boş Enter: kotsuz)';
      case 'field':
        return 'değeri yazın';
      case 'question': {
        const name = this.asked?.there.label?.trim();
        return `bu yerde ${name ? `“${name}” noktası` : 'adsız bir nokta'} var [Düzelt (D) / Ekle (E) / Atla (Esc)]`;
      }
      default:
        return `nokta konumunu belirtin [${pointOptions()} / Kot (Z): ${S.z === null ? 'yok' : this.ctx.format.length(S.z)}]`;
    }
  }

  /** A new point: what was written before is the drawing's to undo, its name no longer given back. */
  protected override accept(p: Vec2): void {
    this.givenBack = null;
    super.accept(p);
  }

  protected onPoint(p: Vec2): void {
    const there = pointAt(this.ctx.doc, p);
    if (there) {
      this.asked = { at: p, there };
      this.stage = 'question';
      return;
    }
    this.write(p);
  }

  /** A point given by the point calculator or by `#ad`: only where a place is asked. */
  override acceptPoint(p: Vec2): boolean {
    return this.stage === 'points' && super.acceptPoint(p);
  }

  override pointerDown(p: ToolPointer): void {
    if (this.stage === 'points') super.pointerDown(p);
  }

  protected override option(key: string): boolean {
    if (this.stage === 'question') {
      if (key === 'D') return this.correct(), true;
      if (key === 'E') {
        this.write(this.asked!.at);
        this.back();
        return true;
      }
      return false;
    }
    if (this.stage !== 'points') return false;
    if (key === 'A' || key === 'K') {
      this.stage = 'field';
      this.refreshPrompt();
      askPointField(this.ctx, key, this.hover, (changed) => {
        // A name given anew is not taken back by Ctrl+Z.
        if (changed && key === 'A') this.givenBack = null;
        this.back();
      });
      return true;
    }
    if (key === 'Z') {
      this.stage = 'z';
      this.refreshPrompt();
      return true;
    }
    return false;
  }

  override input(text: string): boolean {
    if (this.stage === 'z') {
      const z = parseNumber(text);
      if (z === null) return false;
      SurveyPointTool.z = z;
      this.back();
      return true;
    }
    if (this.stage === 'field') return false;
    if (this.stage === 'question') return this.option(text.trim().toLocaleUpperCase('tr-TR'));
    return super.input(text);
  }

  /** Enter: an empty one in Kot clears it; in the question it does nothing; otherwise it leaves, as before. */
  override confirm(): void {
    if (this.stage === 'z') {
      SurveyPointTool.z = null;
      return this.back();
    }
    if (this.stage !== 'points') return;
    super.confirm();
  }

  /** Esc: out of Kot, or past the point asked about (Atla); otherwise the tool leaves. */
  cancel(): boolean {
    if (this.stage === 'z') return this.back(), true;
    if (this.stage === 'question') {
      this.ctx.log.info(`${LABEL}: atlandı.`);
      return this.back(), true;
    }
    return false;
  }

  /**
   * Ctrl+Z: out of Kot or the question first (as Esc); then the newest point or Düzelt as an undo while the drawing
   * has not changed since, its name given back (105 taken back: the next is 105 again); else the drawing's undo.
   */
  override undoStep(): boolean {
    if (this.stage === 'field') return true;
    if (this.stage !== 'points') return this.back(), true;
    const name = this.givenBack;
    if (!super.undoStep()) return false;
    this.givenBack = null;
    if (name !== null) {
      SurveyPointTool.next = name;
      this.refreshPrompt();
    }
    return true;
  }

  private back(): void {
    this.stage = 'points';
    this.asked = null;
    this.refreshPrompt();
    this.ctx.view.requestOverlay();
  }

  /** The point through `cad.point.create`: its name, code and elevation; then the name moves on. */
  private write(p: Vec2): void {
    const S = SurveyPointTool;
    const layerId = this.ctx.doc.layers.active.value;
    const input = {
      layerId,
      p,
      ...(S.z !== null && { z: S.z }),
      ...(S.next && { label: S.next }),
      ...(S.code && { attrs: { Kod: S.code } }),
      ...this.colour(),
    };
    if (this.written(pointCreate.execute({ doc: this.ctx.doc }, input))) this.advance();
  }

  /** Düzelt: the point already there takes the name, code and elevation given, in one step “Nokta düzelt”. */
  private correct(): void {
    const S = SurveyPointTool;
    const { doc, log } = this.ctx;
    const there = this.asked!.there;
    if (!S.next && !S.code && S.z === null) {
      log.warn(`${LABEL}: düzeltilecek değer yok; Ad (A), Kod (K) ya da Kot (Z) verin.`);
      return;
    }
    const uid = doc.uidOf(there.id) ?? '';
    let refused: string | null = null;
    try {
      // Both or neither: a refusal of the second takes the first back.
      doc.transact('Nokta düzelt', () => {
        if (S.next || S.code) {
          const r = entitiesSet.execute({ doc }, { uids: [uid], ...(S.next && { label: S.next }), ...(S.code && { attrs: { Kod: S.code } }), operation: 'attributes' });
          if (r.status !== 'completed' && 'error' in r) refused = r.error.message;
        }
        if (!refused && S.z !== null && there.z !== S.z) {
          const geometry: EditGeometry = { kind: 'point', p: there.p, z: S.z };
          const r = entitiesEdit.execute({ doc }, { operation: 'elevation', changes: [{ kind: 'update', uid, geometry }] });
          if (r.status !== 'completed' && 'error' in r) refused = r.error.message;
        }
        if (refused) throw new Error(refused);
      });
    } catch {
      // Rolled back; the refusal is said below.
    }
    const name = S.next || there.label?.trim();
    if (refused) log.warn(refused);
    else {
      log.success(`${LABEL}: ${name ? `“${name}” noktası` : 'nokta'} düzeltildi.`);
      // Ctrl+Z takes Düzelt back as the tool's own step, as it does a point.
      this.noteMade(there.id);
      this.advance();
    }
    this.back();
  }

  /** The name moves on by Yazı's Artır; a name not ending in a number stays. Ctrl+Z gives the old one back. */
  private advance(): void {
    const S = SurveyPointTool;
    this.givenBack = S.next || null;
    if (S.next) S.next = textIncrement(S.next) ?? S.next;
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const S = SurveyPointTool;
    if (this.stage !== 'points' || !this.hover || !S.next) return;
    // The name the next point takes, beside the cursor.
    const pal = this.ctx.view.palette;
    drawTag(g, view.worldToScreen(this.hover), [S.next], pal.accent, pal.labelHalo);
  }
}
