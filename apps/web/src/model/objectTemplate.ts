import { ENTITY_KIND_LABEL, MAX_LINE_WEIGHT, TEXT_ALIGNS, type Entity, type TextAlign } from './entities';
import type { LayerNode, LineType } from './layers';

/**
 * Nesne şablonları (docs/adr/0176): a template is a drawing recipe for one kind of object (a parcel's boundary, a building, a
 * traverse point): the tool it is drawn with, the layer it goes on (found by name, opened when the drawing lacks it),
 * its colour, line weight, symbol, attributes and label. Templates are the style library's third kind of item, beside
 * symbols and assets (`LibraryTemplate`, model/style.ts): shipped, the user's own or the project's, shared as .kstil files.
 * The desktop's rules are kentos_native_style's `template`; both pass fixtures/style/v1/object-templates.json.
 */

/** The tools a template draws with. */
export const TEMPLATE_TOOLS = ['point', 'line', 'polyline', 'polygon', 'rectangle', 'rectangle3', 'circle', 'text', 'blockInsert'] as const;
export type TemplateTool = (typeof TEMPLATE_TOOLS)[number];

/** The tools' names, as the interface says them. */
export const TEMPLATE_TOOL_LABEL: Record<TemplateTool, string> = {
  point: 'Nokta',
  line: 'Çizgi',
  polyline: 'Çoklu çizgi',
  polygon: 'Kapalı alan',
  rectangle: 'Dikdörtgen',
  rectangle3: 'Döndürülmüş dikdörtgen',
  circle: 'Daire',
  text: 'Yazı',
  blockInsert: 'Blok ekle',
};

/** The tools a group template draws with: its members' rules need a line or an area (docs/adr/0176 §5). */
export const GROUP_TOOLS: readonly TemplateTool[] = ['line', 'polyline', 'polygon', 'rectangle', 'rectangle3'];
/** The group tools that draw a closed shape: their offsets go inside or outside, not left or right. */
export const CLOSED_TOOLS: readonly TemplateTool[] = ['polygon', 'rectangle', 'rectangle3'];

/** How a group template's member makes its object from the drawn shape (docs/adr/0176 §5). */
export const MEMBER_RULES = ['same', 'offset', 'vertices', 'centroid'] as const;
export type MemberRule = (typeof MEMBER_RULES)[number];
/** An offset's side: by the drawing's direction on an open shape, inside or outside on a closed one; or both. */
export const MEMBER_SIDES = ['left', 'right', 'inside', 'outside', 'both'] as const;
export type MemberSide = (typeof MEMBER_SIDES)[number];
/** The sides an offset member may take on an open and on a closed shape. */
export const OPEN_SIDES: readonly MemberSide[] = ['left', 'right', 'both'];
export const CLOSED_SIDES: readonly MemberSide[] = ['inside', 'outside', 'both'];

/** A group template's member: another template of the library, by its id, and the rule it makes its object by. */
export interface TemplateMember {
  readonly template: string;
  readonly rule: MemberRule;
  /** An offset's distance (metres, above zero) and side. */
  readonly distance?: number;
  readonly side?: MemberSide;
}

/** The methods a template may name, by tool (their options in the tool catalog); a tool not here has none to choose. */
export const TEMPLATE_METHODS: Partial<Record<TemplateTool, readonly string[]>> = { circle: ['2N', '3N', 'TTY', 'TTT'] };

/** The layer a template's objects go on: found by its name, opened under its path with this look when the drawing lacks it. */
export interface TemplateLayer {
  /** The groups above it, from the top (“Kadastro”); empty: at the top. */
  readonly path: readonly string[];
  readonly name: string;
  /** Its look when it is opened; absent parts are a new layer's. */
  readonly color?: string;
  readonly lineType?: LineType;
  readonly lineWeight?: number;
}

export interface ObjectTemplate {
  readonly tool: TemplateTool;
  /** The tool's method (Dikdörtgen's “3 nokta”, Daire's “2 nokta”); absent: the tool's first. */
  readonly method?: string;
  readonly layer: TemplateLayer;
  /** The objects' own colour and line weight; absent: their layer's. */
  readonly color?: string;
  readonly lineWeight?: number;
  /** A library symbol's id, drawn instead of the layer's style. */
  readonly symbol?: string;
  /** Attributes written with each object, by name. */
  readonly attrs?: Readonly<Record<string, string>>;
  /** The text shown beside each object. */
  readonly label?: string;
  /** A point template's first name (it goes up with each point, as Nokta's Ad) and code (docs/adr/0152). */
  readonly point?: { readonly name?: string; readonly code?: string };
  /** A text template's height on paper (mm, as Yazı's Yükseklik: the same size printed at every scale), alignment and mask. */
  readonly text?: { readonly height: number; readonly align?: TextAlign; readonly mask?: boolean };
  /** A block template's block, by name. */
  readonly block?: string;
  /** A group template's members: objects made from the drawn one, in the same undo step (docs/adr/0176 §5). */
  readonly members?: readonly TemplateMember[];
}

const LINE_TYPES: readonly LineType[] = ['continuous', 'dashed', 'dashdot', 'dotted'];
/** A colour: hex with or without alpha, or a theme token (the style file's rule). */
const COLOR = /^(#[0-9a-f]{6}([0-9a-f]{2})?|ink|paper|fg|fg-dim)$/i;
/** Empty or only white space, as Unicode's White_Space has it (the product commands' `isBlank`). */
const blank = (text: string) => /^[\t-\r \u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]*$/.test(text);

type O = Record<string, unknown>;
const isObj = (v: unknown): v is O => !!v && typeof v === 'object' && !Array.isArray(v);
const weightOk = (v: unknown) => typeof v === 'number' && Number.isFinite(v) && v >= 0 && v <= MAX_LINE_WEIGHT;

/**
 * What is wrong with a template, each problem with where it is (`where`: “öğe 3 (Parsel sınırı)”), in the order the fields
 * are read; none for a template the app can draw with. A template read from a file or a library is untrusted data.
 */
export function templateIssues(template: unknown, where = 'şablon'): string[] {
  if (!isObj(template)) return [`${where}: şablon tanımı yok`];
  const out: string[] = [];
  const say = (text: string) => out.push(`${where}: ${text}`);
  const tool = template.tool as TemplateTool;
  const known = (TEMPLATE_TOOLS as readonly unknown[]).includes(tool);
  if (!known) say(`bilinmeyen araç “${String(template.tool)}”`);
  if (template.method !== undefined && known) {
    const methods = TEMPLATE_METHODS[tool] ?? [];
    if (!methods.length) say(`${TEMPLATE_TOOL_LABEL[tool]} aracının seçilecek yöntemi yok`);
    else if (!methods.includes(template.method as string)) say(`“${String(template.method)}” yöntemi ${TEMPLATE_TOOL_LABEL[tool]} aracında yok`);
  }
  const layer = template.layer;
  if (!isObj(layer)) say('katmanı yok');
  else {
    if (typeof layer.name !== 'string' || blank(layer.name)) say('katmanın adı boş');
    if (!Array.isArray(layer.path) || layer.path.some((p) => typeof p !== 'string')) say('katmanın yolu metin listesi olmalı');
    else if (layer.path.some((p) => blank(p as string))) say('katmanın yolunda boş ad var');
    if (layer.color !== undefined && (typeof layer.color !== 'string' || !COLOR.test(layer.color))) say(`katmanın rengi geçersiz “${String(layer.color)}”`);
    if (layer.lineType !== undefined && !LINE_TYPES.includes(layer.lineType as LineType)) say(`katmanın çizgi tipi bilinmiyor “${String(layer.lineType)}”`);
    if (layer.lineWeight !== undefined && !weightOk(layer.lineWeight)) say(`katmanın kalınlığı 0 ile ${MAX_LINE_WEIGHT} mm arasında olmalı`);
  }
  if (template.color !== undefined && (typeof template.color !== 'string' || !COLOR.test(template.color))) say(`renk geçersiz “${String(template.color)}”`);
  if (template.lineWeight !== undefined && !weightOk(template.lineWeight)) say(`kalınlık 0 ile ${MAX_LINE_WEIGHT} mm arasında olmalı`);
  if (template.symbol !== undefined && (typeof template.symbol !== 'string' || blank(template.symbol))) say('sembolün kimliği boş');
  if (template.attrs !== undefined) {
    if (!isObj(template.attrs)) say('öznitelikler ad ve metin değer olmalı');
    else
      // By name, as the desktop's JSON map holds them, so both say them in the same order.
      for (const [name, value] of Object.entries(template.attrs).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))) {
        if (blank(name)) say('öznitelik adı boş');
        if (typeof value !== 'string') say(`“${name}” özniteliğinin değeri metin olmalı`);
      }
  }
  if (template.label !== undefined && typeof template.label !== 'string') say('etiket metin olmalı');
  if (template.point !== undefined) {
    if (tool !== 'point') say('ad ve kod yalnız nokta şablonunda olur');
    else if (!isObj(template.point) || (template.point.name !== undefined && typeof template.point.name !== 'string') || (template.point.code !== undefined && typeof template.point.code !== 'string'))
      say('noktanın adı ve kodu metin olmalı');
  }
  if (template.text !== undefined) {
    if (tool !== 'text') say('yazı bilgisi yalnız yazı şablonunda olur');
    else if (!isObj(template.text)) say('yazı bilgisi yok');
    else {
      const h = template.text.height;
      if (typeof h !== 'number' || !Number.isFinite(h) || !(h > 0)) say('yazının yüksekliği sıfırdan büyük olmalı');
      if (template.text.align !== undefined && !TEXT_ALIGNS.includes(template.text.align as TextAlign)) say(`bilinmeyen hiza “${String(template.text.align)}”`);
      if (template.text.mask !== undefined && typeof template.text.mask !== 'boolean') say('zemin açık ya da kapalı olmalı');
    }
  }
  if (tool === 'blockInsert') {
    if (typeof template.block !== 'string' || blank(template.block)) say('blok şablonunun bloğu yok');
  } else if (template.block !== undefined) say('blok yalnız blok şablonunda olur');
  if (template.members !== undefined) {
    if (!GROUP_TOOLS.includes(tool)) say('üyeler yalnız çizgi, çoklu çizgi, kapalı alan ve dikdörtgen şablonunda olur');
    else if (!Array.isArray(template.members)) say('üyeler liste olmalı');
    else {
      const closed = CLOSED_TOOLS.includes(tool);
      template.members.forEach((m: unknown, i) => {
        const at = `${i + 1}. üye`;
        if (!isObj(m)) return void say(`${at}: tanımı yok`);
        if (typeof m.template !== 'string' || blank(m.template)) say(`${at}: şablonu yok`);
        if (!(MEMBER_RULES as readonly unknown[]).includes(m.rule)) say(`${at}: bilinmeyen kural “${String(m.rule)}”`);
        if (m.rule === 'offset') {
          if (typeof m.distance !== 'number' || !Number.isFinite(m.distance) || !(m.distance > 0)) say(`${at}: öteleme uzaklığı sıfırdan büyük olmalı`);
          if (!(closed ? CLOSED_SIDES : OPEN_SIDES).includes(m.side as MemberSide))
            say(`${at}: ${closed ? 'kapalı şekilde yan içe, dışa' : 'açık şekilde yan sola, sağa'} ya da iki yana olmalı`);
        } else {
          if (m.distance !== undefined) say(`${at}: uzaklık yalnız ötelenmiş üyede olur`);
          if (m.side !== undefined) say(`${at}: yan yalnız ötelenmiş üyede olur`);
        }
      });
    }
  }
  return out;
}

/**
 * What keeps a group template from starting with this library (docs/adr/0176 §5), a problem for each member that has
 * one, in their order: its template the library lacks, has a problem of its own, or is itself a group (one with
 * members); a Köşelere nokta member that is not a point template; an Ağırlık merkezine member that is neither a point
 * nor a text template, or a text template without a label (its first text). An Aynı geometri or offset member's tool is
 * free: it gives only its layer, look, attributes and label. None for a template without members. The desktop's is
 * `kentos_native_style::object_template::member_issues`; both pass fixtures/style/v1/template-groups.json.
 */
export function memberIssues(template: ObjectTemplate, find: (id: string) => { readonly name: string; readonly template: unknown } | undefined): string[] {
  const out: string[] = [];
  (template.members ?? []).forEach((m, i) => {
    const at = `${i + 1}. üye`;
    const item = find(m.template);
    if (!item) return void out.push(`${at}: “${m.template}” kimlikli şablon kitaplıkta yok; üyeyi düzenleyicide yeniden seçin.`);
    const own = templateIssues(item.template, `“${item.name}” şablonu`)[0];
    if (own) return void out.push(`${at}: ${own}`);
    const t = item.template as ObjectTemplate;
    if (t.members?.length) return void out.push(`${at}: “${item.name}” bir grup şablonu; grup şablonu üye olamaz.`);
    if (m.rule === 'vertices' && t.tool !== 'point') out.push(`${at}: köşelere nokta üyesi bir nokta şablonu olmalı; “${item.name}” ${TEMPLATE_TOOL_LABEL[t.tool]} şablonu.`);
    if (m.rule === 'centroid') {
      if (t.tool !== 'point' && t.tool !== 'text') out.push(`${at}: ağırlık merkezi üyesi bir nokta ya da yazı şablonu olmalı; “${item.name}” ${TEMPLATE_TOOL_LABEL[t.tool]} şablonu.`);
      else if (t.tool === 'text' && (typeof t.label !== 'string' || blank(t.label))) out.push(`${at}: “${item.name}” yazı şablonunun etiketi yok: ağırlık merkezine yazılacak ilk metni etiketine yazın.`);
    }
  });
  return out;
}

/**
 * Where a template draws in a drawing's layer tree (docs/adr/0176 §3): `found`, that layer; `locked`, that node is
 * locked by itself or a group above it and the template does not start; `open`, the drawing lacks it and it is opened
 * under `parent` (a group's id; null: at the top), inside the groups `create` names, made in this order.
 */
export type TemplateLayerAnswer = { readonly found: string } | { readonly locked: string } | { readonly open: { readonly parent: string | null; readonly create: readonly string[] } };

/**
 * Where a template with `layer` draws, in a tree whose `isLocked` says whether a node is locked by itself or a group
 * above it. The layer is found by its name: of several, the one under the template's path, else the first in the
 * tree. Without one, the path's groups are followed as far as the tree has them (the first group of each name), and
 * the rest are opened. Names are compared without the white space around them. The desktop's is
 * `kentos_interaction::templates::find_layer`; both pass fixtures/style/v1/template-layers.json.
 */
export function templateLayer(layers: { readonly tree: readonly LayerNode[]; isLocked(id: string): boolean }, layer: Pick<TemplateLayer, 'path' | 'name'>): TemplateLayerAnswer {
  const name = layer.name.trim();
  const path = layer.path.map((g) => g.trim());
  const found: { node: LayerNode; groups: string[] }[] = [];
  const walk = (nodes: readonly LayerNode[], groups: string[]): void => {
    for (const n of nodes) {
      if (n.type === 'group') walk(n.children, [...groups, n.name.trim()]);
      else if (n.name.trim() === name) found.push({ node: n, groups });
    }
  };
  walk(layers.tree, []);
  const first = found[0];
  if (first) {
    const chosen = (found.find((f) => f.groups.length === path.length && f.groups.every((g, i) => g === path[i])) ?? first).node;
    return layers.isLocked(chosen.id) ? { locked: chosen.id } : { found: chosen.id };
  }
  let level = layers.tree;
  let parent: LayerNode | null = null;
  let depth = 0;
  for (const g of path) {
    const next = level.find((n) => n.type === 'group' && n.name.trim() === g);
    if (!next) break;
    parent = next;
    level = next.children;
    depth++;
  }
  if (parent && layers.isLocked(parent.id)) return { locked: parent.id };
  return { open: { parent: parent?.id ?? null, create: path.slice(depth) } };
}

/** What is said when a template's layer, or the group it would be opened in, is locked: the node by its name, the template by its. */
export function lockedTemplateLayerText(node: Pick<LayerNode, 'name' | 'type'>, template: string): string {
  return `“${node.name}” ${node.type === 'group' ? 'grubu' : 'katmanı'} kilitli; “${template}” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın.`;
}

/** The tool a template of each drawable kind draws with (Seçili nesneden şablon); the other kinds have none. */
const TOOL_OF_KIND: Partial<Record<Entity['kind'], TemplateTool>> = { point: 'point', line: 'line', polyline: 'polyline', polygon: 'polygon', circle: 'circle', text: 'text', insert: 'blockInsert' };

/**
 * The template made from a drawn object (Seçili nesneden şablon, docs/adr/0176 §4), named after its layer, with no
 * category: the tool is the object's kind's; the layer is its own, with the groups above it and its look; the
 * object's own colour, line weight and symbol go with it when it has them, and so do its attributes (by name) and
 * label, but a point's label is the template's first name and its `Kod` its code (docs/adr/0152); a text gives its
 * height on paper in millimetres, as Yazı's Yükseklik is given (its height on the ground × 1000 / the drawing's
 * `plotScale`), its alignment and mask, an insert its block by name. Another kind is refused, said by its name. The
 * desktop's is `kentos_native_style::object_template::from_object`; both pass
 * fixtures/style/v1/template-from-object.json.
 */
export function templateFromObject(
  e: Entity,
  layers: { get(id: string): LayerNode | undefined; parentOf(id: string): LayerNode | null },
  blockName: (id: string) => string | undefined,
  plotScale: number,
): { readonly name: string; readonly template: ObjectTemplate } | { readonly refused: string } {
  const tool = TOOL_OF_KIND[e.kind];
  if (!tool) return { refused: `${ENTITY_KIND_LABEL[e.kind]} nesnesinden şablon yapılamaz: şablon nokta, çizgi, çoklu çizgi, kapalı alan, daire, yazı ya da blok çizer.` };
  const layer = layers.get(e.layerId);
  const path: string[] = [];
  for (let g = layer ? layers.parentOf(layer.id) : null; g; g = layers.parentOf(g.id)) path.unshift(g.name);
  const name = layer?.name ?? e.layerId;
  const attrs: Record<string, string> = { ...e.attrs };
  let label = e.label;
  let point: { name?: string; code?: string } | undefined;
  if (e.kind === 'point') {
    point = { ...(label && { name: label }), ...(attrs.Kod && { code: attrs.Kod }) };
    if (attrs.Kod) delete attrs.Kod;
    label = undefined;
  }
  const sorted = Object.fromEntries(Object.entries(attrs).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  const template: ObjectTemplate = {
    tool,
    layer: { path, name, ...(layer && { color: layer.style.color, lineType: layer.style.lineType, lineWeight: layer.style.lineWeight }) },
    ...(e.color !== undefined && { color: e.color }),
    ...(e.lineWeight !== undefined && { lineWeight: e.lineWeight }),
    ...(e.symbol !== undefined && { symbol: e.symbol }),
    ...(point && Object.keys(point).length > 0 && { point }),
    ...(Object.keys(sorted).length > 0 && { attrs: sorted }),
    ...(label && { label }),
    ...(e.kind === 'text' && { text: { height: (e.height * 1000) / plotScale, ...(e.align && { align: e.align }), ...(e.mask === true && { mask: true }) } }),
    ...(e.kind === 'insert' && { block: blockName(e.block) ?? e.block }),
  };
  return { name, template };
}
