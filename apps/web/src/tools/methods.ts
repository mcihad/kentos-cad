import { foldTurkish } from '../core/text';
import { TOOL_CATALOG } from './catalog';

/** A tool's method as a typed name starts it: the tool's command, the option its method types, its words. */
export interface MethodStart {
  readonly command: string;
  readonly option: string;
  readonly title: string;
  readonly label: string;
}

let byAlias: Map<string, MethodStart> | null = null;

/**
 * The method a typed name starts (ToolMethod.aliases: DOR is Ölçülendirme as Koordinat, docs/adr/0147 §7), folded as
 * the command registry folds its aliases; none for another name.
 */
export function methodByAlias(text: string): MethodStart | undefined {
  if (!byAlias) {
    byAlias = new Map();
    for (const t of TOOL_CATALOG)
      for (const m of t.methods ?? [])
        if (m.option) for (const a of m.aliases ?? []) byAlias.set(foldTurkish(a), { command: `tool.${t.id}`, option: m.option, title: t.label, label: m.label });
  }
  return byAlias.get(foldTurkish(text.trim()));
}
