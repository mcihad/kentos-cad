import type { LayerNode } from '../../model/layers';
import { resolveColor, type CanvasPalette } from '../../render/color';

/** CSS colour for a layer's swatch (theme tokens resolved). */
export const layerSwatch = (n: LayerNode, palette: CanvasPalette) => resolveColor(n.style.color, palette);
/** CSS colour for a colour value that may be a theme token (fg, fg-dim, ink). */
export const colorSwatch = (value: string, palette: CanvasPalette) => resolveColor(value, palette);
