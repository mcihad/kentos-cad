import type { AppContext } from './context';
import { lockedTemplateLayerText, templateIssues, templateLayer, type TemplateLayer } from '../model/objectTemplate';
import type { TemplateRun } from '../tools/templateStamp';
import { RIBBON_TEXTS } from '../ui/ribbon/ribbonPlan';

/**
 * Drawing with an object template (docs/adr/0176 §3; the desktop's `templates.rs`). Choosing a template makes its
 * layer active (opened under its path, one undo step “Katman ekle”, when the drawing lacks it), takes its colour and
 * line weight into the session's current ones and starts its tool, with its method; every object the tool writes
 * takes its symbol, attributes and label (tools/templateStamp.ts). The run ends with the tool (the ToolManager lets it go): Esc,
 * another command or another template give the colour and weight back; the active layer stays. Son komutu yinele
 * starts the template again. A locked layer keeps the template from starting, and says so.
 */

/** `template.draw`: draws with the library's template `id` (Stil yöneticisi's Şablonla çiz, a trace's `template` step). */
export function drawWithTemplate(ctx: AppContext, id: string): void {
  const item = ctx.styles.library.template(id);
  if (!item) return void ctx.log.warn(`“${id}” kimlikli şablon kitaplıkta yok: silinmiş olabilir. Şablonu Stil yöneticisinde seçin.`);
  const issue = templateIssues(item.template, `“${item.name}” şablonu`)[0];
  if (issue) return void ctx.log.warn(`${issue}; şablonu Stil yöneticisinde düzeltin.`);
  const t = item.template;
  if (!useLayer(ctx, t.layer, item.name)) return;
  const settings = ctx.settings;
  const run: TemplateRun = {
    id,
    name: item.name,
    stamp: { ...(t.symbol !== undefined && { symbol: t.symbol }), attrs: { ...t.attrs }, ...(t.label !== undefined && { label: t.label }) },
    color: t.color ?? null,
    lineWeight: t.lineWeight ?? null,
    // Another template's run gives way, its colour and weight kept as the ones to give back.
    before: settings.template.value?.before ?? { color: settings.color.value, lineWeight: settings.lineWeight.value },
  };
  ctx.tools.activate(t.tool, run);
  // Its method, as the ribbon's menu starts one (Daire: 2 nokta).
  if (t.method && ctx.tools.activeId.value === t.tool && !ctx.tools.active.input?.(t.method)) ctx.log.warn(RIBBON_TEXTS.cannotStart(ctx.tools.get(t.tool)?.label ?? t.tool, t.method));
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
