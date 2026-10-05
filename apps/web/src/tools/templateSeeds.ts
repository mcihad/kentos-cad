import { TextTool } from './annotateTools';
import { BlockInsertTool } from './blockTools';
import { SurveyPointTool } from './surveyPointTool';
import type { TemplateRun } from './templateStamp';

/**
 * A point, text or block template's own options set in its tool for the run (docs/adr/0176 §3b; the desktop's
 * `templates.rs` `seed_tool`): Nokta's Ad and Kod (a template's names go on from run to run: `names` keeps each
 * one's next, by its id), Yazı's height on paper, alignment and mask, Blok ekle's block. Gives back what puts the
 * tool's own options back at the run's end; null when the template sets none.
 */
export function seedTool(run: TemplateRun, names: Map<string, string>): (() => void) | null {
  const seed = run.seed;
  if (!seed) return null;
  if (seed.kind === 'point') {
    const S = SurveyPointTool;
    const before = { next: S.next, code: S.code };
    S.code = seed.code;
    const named = seed.name !== undefined;
    if (named) S.next = names.get(run.id) ?? seed.name;
    return () => {
      if (named) {
        names.set(run.id, S.next);
        S.next = before.next;
      }
      S.code = before.code;
    };
  }
  if (seed.kind === 'text') {
    const before = TextTool.useOptions(seed);
    return () => void TextTool.useOptions(before);
  }
  const before = BlockInsertTool.block;
  BlockInsertTool.block = seed.block;
  return () => {
    BlockInsertTool.block = before;
  };
}
