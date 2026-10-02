import layoutText from '../../../../../shaders/wgsl/styled.layout.json?raw';
import common from '../../../../../shaders/wgsl/styled/common.wgsl?raw';
import fill from '../../../../../shaders/wgsl/styled/fill.wgsl?raw';
import marker from '../../../../../shaders/wgsl/styled/marker.wgsl?raw';
import shapes from '../../../../../shaders/wgsl/styled/shapes.wgsl?raw';
import stroke from '../../../../../shaders/wgsl/styled/stroke.wgsl?raw';

/**
 * WGSL twins of the styled WebGL2 shaders (../webgl2/styledShaders.ts):
 * strokes, solid/hatch/tile/pattern fills and markers, with the same
 * distance fields, dash test and unit handling. The sources are shared with
 * the native renderer (shaders/wgsl/styled, its contract
 * styled.layout.json): joined here in the contract's order, each under a
 * `// ── path ──` line, the way scripts/wgsl/browser-check.mjs joins them.
 * Group 0 is the frame with the atlas beside it (the styled pipelines' own
 * group; contract version 3), group 1 the batch style with its tile's origin. The atlas has one level (images are drawn at
 * their shown size), so it is sampled with textureSampleLevel, which needs
 * no uniform control flow.
 */
const SOURCES: Record<string, string> = {
  'styled/common.wgsl': common,
  'styled/shapes.wgsl': shapes,
  'styled/stroke.wgsl': stroke,
  'styled/fill.wgsl': fill,
  'styled/marker.wgsl': marker,
};

const layout = JSON.parse(layoutText) as { sources: string[] };

export const STYLED_WGSL = layout.sources
  .map((path) => {
    const text = SOURCES[path];
    if (text === undefined) throw new Error(`styled.layout.json lists ${path}, which is not imported`);
    return `// ── ${path} ──\n${text}\n`;
  })
  .join('');
