import type { AppContext } from '../app/context';
import type { TextAlign } from '../model/entities';
import type { MemberRule, MemberSide } from '../model/objectTemplate';

/**
 * What a template being drawn with gives every object its tool writes (docs/adr/0176 §3): the run the session keeps
 * (`ctx.settings.template`, started by app/objectTemplates.ts, let go by the ToolManager when its tool ends) and the
 * fields a create command's input takes from it.
 */

/** What every object drawn with a template takes besides its geometry. */
export interface TemplateStamp {
  readonly symbol?: string;
  readonly attrs: Readonly<Record<string, string>>;
  readonly label?: string;
}

/**
 * A point, text or block template's own options for its tool's run (docs/adr/0176 §3b, tools/templateSeeds.ts):
 * Nokta's Ad (absent: Nokta's own series goes on) and Kod, Yazı's height on paper, alignment and mask, Blok ekle's
 * block (its id).
 */
export type TemplateSeed =
  | { readonly kind: 'point'; readonly name?: string; readonly code: string }
  | { readonly kind: 'text'; readonly heightMm: number; readonly align: TextAlign | null; readonly mask: boolean }
  | { readonly kind: 'block'; readonly block: string };

/**
 * A group template's member as the run writes it (docs/adr/0176 §5, tools/templateMembers.ts): its template's id (a
 * point or text template's series is kept by it) and name, its rule, the layer its objects go on (found or opened when
 * the group started), their look, attributes and label, and a point template's first name and code or a text template's
 * height on paper, alignment and mask.
 */
export interface RunMember {
  readonly id: string;
  readonly name: string;
  readonly rule: MemberRule;
  readonly distance?: number;
  readonly side?: MemberSide;
  readonly layerId: string;
  readonly color: string | null;
  readonly lineWeight: number | null;
  readonly stamp: TemplateStamp;
  readonly point?: { readonly name?: string; readonly code: string };
  readonly text?: { readonly heightMm: number; readonly align: TextAlign | null; readonly mask: boolean };
}

/** A template being drawn with: its library id and name, its stamp, and what it found and gives back at its end. */
export interface TemplateRun {
  readonly id: string;
  readonly name: string;
  readonly stamp: TemplateStamp;
  /** The colour and line weight it draws in (null: the layer's). */
  readonly color: string | null;
  readonly lineWeight: number | null;
  /** The session's colour and line weight before it. */
  readonly before: { readonly color: string | null; readonly lineWeight: number | null };
  /** The tool's own options it sets; the run's end gives the tool's back. */
  readonly seed?: TemplateSeed;
  /** A group template's members: their objects are written with each object the tool writes, in its undo step. */
  readonly members?: readonly RunMember[];
}

/**
 * What a new object takes from the template being drawn with: its symbol, its attributes (`own`, the tool's own, over
 * them: Nokta's Kod) and its label; nothing while the tool runs by itself. Spread into a create command's input.
 */
export function stamp(ctx: AppContext, own?: Readonly<Record<string, string>>): { symbol?: string; attrs?: Record<string, string>; label?: string } {
  const s = ctx.settings.template.value?.stamp;
  const attrs = { ...s?.attrs, ...own };
  return {
    ...(s?.symbol !== undefined && { symbol: s.symbol }),
    ...(Object.keys(attrs).length > 0 && { attrs }),
    ...(s?.label !== undefined && { label: s.label }),
  };
}
