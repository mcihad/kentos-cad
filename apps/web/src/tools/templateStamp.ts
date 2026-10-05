import type { AppContext } from '../app/context';

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
