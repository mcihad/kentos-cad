import type { AppContext } from './context';
import { lockedTemplateLayerText, templateFromObject, templateIssues, templateLayer, type ObjectTemplate, type TemplateLayer } from '../model/objectTemplate';
import { formOfTemplate } from '../model/templateForm';
import type { TemplateRun, TemplateSeed } from '../tools/templateStamp';
import { RIBBON_TEXTS } from '../ui/ribbon/ribbonPlan';

/**
 * Drawing with an object template (docs/adr/0176 §3; the desktop's `templates.rs`). Choosing a template makes its
 * layer active (opened under its path, one undo step “Katman ekle”, when the drawing lacks it), takes its colour and
 * line weight into the session's current ones and starts its tool, with its method; every object the tool writes
 * takes its symbol, attributes and label (tools/templateStamp.ts). A point, text or block template also sets its tool's
 * own options for the run (docs/adr/0176 §3b, tools/templateSeeds.ts): Nokta's Ad and Kod (a template's names go on
 * from run to run), Yazı's height on paper, alignment and mask, Blok ekle's block, found by its name (a template whose
 * block the drawing lacks does not start, and says so). The run ends with the tool (the ToolManager lets it go): Esc,
 * another command or another template give the colour, the weight and the tool's own options back; the active layer
 * stays. Son komutu yinele starts the template again. A locked layer keeps the template from starting, and says so.
 */

/** `template.draw`: draws with the library's template `id` (Stil yöneticisi's Şablonla çiz, a trace's `template` step). */
export function drawWithTemplate(ctx: AppContext, id: string): void {
  const item = ctx.styles.library.template(id);
  if (!item) return void ctx.log.warn(`“${id}” kimlikli şablon kitaplıkta yok: silinmiş olabilir. Şablonu Stil yöneticisinde seçin.`);
  const issue = templateIssues(item.template, `“${item.name}” şablonu`)[0];
  if (issue) return void ctx.log.warn(`${issue}; şablonu Stil yöneticisinde düzeltin.`);
  const t = item.template;
  // A block template's block, by its name: without it the template does not start.
  let block: string | undefined;
  if (t.tool === 'blockInsert' && t.block !== undefined) {
    block = ctx.doc.blocks.value.find((b) => b.name === t.block)?.id;
    if (block === undefined) return void ctx.log.warn(missingBlockText(t.block, item.name));
  }
  if (!useLayer(ctx, t.layer, item.name)) return;
  const settings = ctx.settings;
  const seed = seedOf(t, block);
  const run: TemplateRun = {
    id,
    name: item.name,
    stamp: { ...(t.symbol !== undefined && { symbol: t.symbol }), attrs: { ...t.attrs }, ...(t.label !== undefined && { label: t.label }) },
    color: t.color ?? null,
    lineWeight: t.lineWeight ?? null,
    // Another template's run gives way, its colour and weight kept as the ones to give back.
    before: settings.template.value?.before ?? { color: settings.color.value, lineWeight: settings.lineWeight.value },
    ...(seed && { seed }),
  };
  ctx.tools.activate(t.tool, run);
  // Its method, as the ribbon's menu starts one (Daire: 2 nokta).
  if (t.method && ctx.tools.activeId.value === t.tool && !ctx.tools.active.input?.(t.method)) ctx.log.warn(RIBBON_TEXTS.cannotStart(ctx.tools.get(t.tool)?.label ?? t.tool, t.method));
}

/** Why a block template does not start: the drawing lacks its block (the desktop's `missing_block_text`). */
export function missingBlockText(block: string, template: string): string {
  return `Çizimde “${block}” bloğu yok: “${template}” şablonu bu bloğu yerleştirir. Bloğu Blok oluştur ile tanımlayın ya da bir DXF'ten alın.`;
}

/**
 * The tool's own options a point, text or block template sets for its run: Nokta's Kod always (none: no code) and
 * its Ad when it names its points; Yazı's height on paper, alignment and mask when it has its text part; Blok ekle's
 * block (`block`, its id, found by its name).
 */
function seedOf(t: ObjectTemplate, block: string | undefined): TemplateSeed | null {
  if (t.tool === 'point') return { kind: 'point', ...(t.point?.name && { name: t.point.name }), code: t.point?.code ?? '' };
  if (t.tool === 'text' && t.text) return { kind: 'text', heightMm: t.text.height, align: t.text.align ?? null, mask: t.text.mask ?? false };
  if (t.tool === 'blockInsert' && block !== undefined) return { kind: 'block', block };
  return null;
}

/** Makes the template's layer active, opening it when the drawing lacks it; false (said) when it, or the group it would go in, is locked or the drawing refuses it. */
function useLayer(ctx: AppContext, layer: TemplateLayer, name: string): boolean {
  const doc = ctx.doc;
  const answer = templateLayer(doc.layers, layer);
  if ('found' in answer) {
    doc.layers.setActive(answer.found);
    return true;
  }
  if ('locked' in answer) {
    const node = doc.layers.get(answer.locked);
    if (node) ctx.log.warn(lockedTemplateLayerText(node, name));
    return false;
  }
  try {
    doc.transact('Katman ekle', () => {
      let under = answer.open.parent;
      for (const group of answer.open.create) under = doc.addLayer({ name: group, type: 'group' }, under).id;
      const style = { ...(layer.color !== undefined && { color: layer.color }), ...(layer.lineType !== undefined && { lineType: layer.lineType }), ...(layer.lineWeight !== undefined && { lineWeight: layer.lineWeight }) };
      doc.addLayer({ name: layer.name.trim(), type: 'layer', style }, under, { activate: true });
    });
    return true;
  } catch (e) {
    ctx.log.warn(e instanceof Error ? e.message : String(e));
    return false;
  }
}

/**
 * `template.fromSelection` (Seçili nesneden şablon, docs/adr/0176 §4): the first selected object's layer, look,
 * attributes and label in a new template, opened in the Şablon düzenleyici to be named and saved; a kind no template
 * draws is said.
 */
export function templateFromSelection(ctx: AppContext): void {
  const first = [...ctx.selection.ids.value][0];
  const e = first === undefined ? undefined : ctx.doc.get(first);
  if (!e) return void ctx.log.warn('Önce çizimde bir nesne seçin; şablon onun katmanını, görünüşünü ve özniteliklerini alır.');
  const made = templateFromObject(e, ctx.doc.layers, (id) => ctx.doc.block(id)?.name, ctx.doc.settings.plotScale.value);
  if ('refused' in made) return void ctx.log.warn(made.refused);
  const form = formOfTemplate({ name: made.name, path: [], template: made.template });
  void import('../ui/templates/TemplateEditor').then((m) => m.openTemplateEditor(ctx, { form }));
}
