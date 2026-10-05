import type { AppContext } from './context';
import { lockedTemplateLayerText, memberIssues, templateFromObject, templateIssues, templateLayer, type ObjectTemplate, type TemplateLayer, type TemplateMember } from '../model/objectTemplate';
import { formOfTemplate } from '../model/templateForm';
import type { RunMember, TemplateRun, TemplateSeed } from '../tools/templateStamp';
import { RIBBON_TEXTS } from '../ui/ribbon/ribbonPlan';

/**
 * Drawing with an object template (docs/adr/0176 §3; the desktop's `templates.rs`). Choosing a template makes its
 * layer active (opened under its path, one undo step “Katman ekle”, when the drawing lacks it), takes its colour and
 * line weight into the session's current ones and starts its tool, with its method; every object the tool writes
 * takes its symbol, attributes and label (tools/templateStamp.ts). A point, text or block template also sets its tool's
 * own options for the run (docs/adr/0176 §3b, tools/templateSeeds.ts): Nokta's Ad and Kod (a template's names go on
 * from run to run in the drawing), Yazı's height on paper, alignment and mask, Blok ekle's block, found by its name
 * (a template whose block the drawing lacks does not start, and says so). The run ends with the tool (the ToolManager
 * lets it go): Esc, another command or another template give the colour, the weight and the tool's own options back;
 * the active layer stays. Son komutu yinele starts the template again. A locked layer keeps the template from
 * starting, and says so. A group template's members (§5) are checked against the library first (what keeps one from
 * starting is said), their layers found or opened with the template's in one step, and every object the tool writes
 * takes their objects in its undo step (tools/templateMembers.ts).
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
  // A group template's members, from the library: what keeps one from starting is said, and nothing changes.
  const lib = ctx.styles.library;
  const memberIssue = memberIssues(t, (mid) => lib.template(mid))[0];
  if (memberIssue) return void ctx.log.warn(`“${item.name}” grup şablonu başlamaz: ${memberIssue}`);
  const members = (t.members ?? []).map((m) => ({ m, item: lib.template(m.template)! }));
  const layerIds = useLayers(ctx, [t.layer, ...members.map(({ item: mi }) => mi.template.layer)], item.name);
  if (!layerIds) return;
  const settings = ctx.settings;
  const seed = seedOf(t, block);
  const run: TemplateRun = {
    id,
    name: item.name,
    stamp: stampOf(t),
    color: t.color ?? null,
    lineWeight: t.lineWeight ?? null,
    // Another template's run gives way, its colour and weight kept as the ones to give back.
    before: settings.template.value?.before ?? { color: settings.color.value, lineWeight: settings.lineWeight.value },
    ...(seed && { seed }),
    ...(members.length > 0 && { members: members.map(({ m, item: mi }, i): RunMember => runMember(m, mi, layerIds[i + 1])) }),
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

/** What every object drawn with a template takes besides its geometry: its symbol, attributes and label. */
function stampOf(t: ObjectTemplate): TemplateRun['stamp'] {
  return { ...(t.symbol !== undefined && { symbol: t.symbol }), attrs: { ...t.attrs }, ...(t.label !== undefined && { label: t.label }) };
}

/** A group template's member as the run writes it: its rule, the layer found or opened for it, its template's look. */
function runMember(m: TemplateMember, item: { readonly id: string; readonly name: string; readonly template: ObjectTemplate }, layerId: string): RunMember {
  const t = item.template;
  return {
    id: item.id,
    name: item.name,
    rule: m.rule,
    ...(m.distance !== undefined && { distance: m.distance }),
    ...(m.side !== undefined && { side: m.side }),
    layerId,
    color: t.color ?? null,
    lineWeight: t.lineWeight ?? null,
    stamp: stampOf(t),
    ...(t.tool === 'point' && { point: { ...(t.point?.name && { name: t.point.name }), code: t.point?.code ?? '' } }),
    ...(t.tool === 'text' && t.text && { text: { heightMm: t.text.height, align: t.text.align ?? null, mask: t.text.mask ?? false } }),
  };
}

/**
 * Finds or opens the template's layers (its own first, then its members'), the opened ones in one step “Katman ekle”,
 * and makes the first active: their ids, in order; null (said) when one of them, or the group it would go in, is locked
 * or the drawing refuses it, and then nothing changes.
 */
function useLayers(ctx: AppContext, layers: readonly TemplateLayer[], name: string): string[] | null {
  const doc = ctx.doc;
  for (const layer of layers) {
    const answer = templateLayer(doc.layers, layer);
    if ('locked' in answer) {
      const node = doc.layers.get(answer.locked);
      if (node) ctx.log.warn(lockedTemplateLayerText(node, name));
      return null;
    }
  }
  try {
    const ids = doc.transact('Katman ekle', () =>
      // One after another: a layer opened for one is found for the next.
      layers.map((layer) => {
        const answer = templateLayer(doc.layers, layer);
        if ('found' in answer) return answer.found;
        if ('locked' in answer) throw new Error(`“${layer.name}” katmanı kilitli; “${name}” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın.`);
        let under = answer.open.parent;
        for (const group of answer.open.create) under = doc.addLayer({ name: group, type: 'group' }, under).id;
        const style = { ...(layer.color !== undefined && { color: layer.color }), ...(layer.lineType !== undefined && { lineType: layer.lineType }), ...(layer.lineWeight !== undefined && { lineWeight: layer.lineWeight }) };
        return doc.addLayer({ name: layer.name.trim(), type: 'layer', style }, under).id;
      }),
    );
    doc.layers.setActive(ids[0]);
    return ids;
  } catch (e) {
    ctx.log.warn(e instanceof Error ? e.message : String(e));
    return null;
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
