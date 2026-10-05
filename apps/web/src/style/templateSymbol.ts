import type { ObjectTemplate } from '../model/objectTemplate';
import type { Symbol } from '../model/style';
import type { StyleLibrary } from './library';

/**
 * What a template's card and preview show (docs/adr/0176): its own symbol when the library has it, else one made of the
 * template's colour and line weight (its layer's when it gives none) in its tool's shape: a dot for a point or a block, a
 * word for a text, a line for a line, an outline for an area. The desktop's is kentos_native_style's
 * `template::preview_symbol`.
 */
export function templateSymbol(template: ObjectTemplate, lib?: StyleLibrary): Symbol {
  const own = template.symbol ? lib?.symbol(template.symbol) : undefined;
  if (own) return own;
  const color = template.color ?? template.layer.color ?? 'ink';
  const width = template.lineWeight ?? template.layer.lineWeight ?? 0.35;
  switch (template.tool) {
    case 'point':
    case 'blockInsert':
      return { type: 'marker', layers: [{ id: 'p', type: 'shape', shape: 'circle', size: 2.4, fill: color }] };
    case 'text':
      return { type: 'marker', layers: [{ id: 't', type: 'text', text: 'Abc', size: 3.2, color }] };
    case 'line':
    case 'polyline':
      return { type: 'line', layers: [{ id: 'l', type: 'simpleLine', color, width }] };
    default:
      return { type: 'fill', layers: [{ id: 'l', type: 'simpleLine', color, width }] };
  }
}
