import { batchImage, batchImagePx, batchInView, batchLegible, MARKER_STRIDE, STROKE_STRIDE, type AtlasHit, type AtlasSource, type RGBA, type ScaleRange, type StyledBatch } from '../types';
import { AREA_VS, HATCH_FS, MARKER_FS, MARKER_VS, PATTERN_FS, SHAPE_IDS, STROKE_FS, STROKE_VS, TILE_FS } from './styledShaders';

/**
 * WebGL2 side of the styled batches (docs/STYLE.md §6): creates the vertex
 * arrays on upload and draws a layer's batches in their symbol-level order.
 * Zoom-dependent sizes are computed in the shaders from the frame
 * uniforms, so nothing is rebuilt while panning or zooming.
 *
 * A frame runs in two steps: `prepare` decides for every batch whether it
 * shows (scale range, view box) and asks the atlas for the images it needs
 * at their shown size (new images arrive in the texture as they are drawn);
 * `draw` then only issues draw calls, switching programs and blend modes
 * only when they change.
 */

interface Program {
  program: WebGLProgram;
  u: Record<string, WebGLUniformLocation | null>;
  a: Record<string, number>;
  /** Frame whose shared uniforms this program already has. */
  frame: number;
}

export interface GpuStyled {
  batch: StyledBatch;
  vao: WebGLVertexArrayObject;
  buffers: WebGLBuffer[];
  count: number;
  /** Set by prepare: drawn this frame, and where its image is. */
  visible: boolean;
  hit: AtlasHit | null;
}

export interface StyledFrame {
  cam: readonly [number, number];
  pxPerM: number;
  dpr: number;
  viewPx: readonly [number, number];
  scaleDenominator: number;
}

const FRAME_UNIFORMS = ['u_cam', 'u_pxPerM', 'u_dpr', 'u_viewPx'];
const DASH_UNIFORMS = ['u_dash0', 'u_dash1', 'u_dashTotal', 'u_dashOn', 'u_dashOffset'];
const PATTERN_UNIFORMS = ['u_size', 'u_rot', 'u_shift', 'u_stagger', 'u_jitter', 'u_coverage', 'u_seed', 'u_reach', 'u_unit', 'u_shape', 'u_sp', 'u_half', 'u_markOff', 'u_markRot', 'u_fill', 'u_stroke', 'u_strokeW', 'u_tint', 'u_opacity'];
const SHAPE_INDEX = new Map<string, number>(SHAPE_IDS.map((s, i) => [s, i]));
const NONE: RGBA = [0, 0, 0, 0];

export const inScale = (b: ScaleRange, den: number) => !((b.minScale !== undefined && den < b.minScale) || (b.maxScale !== undefined && den > b.maxScale));

/** Dash uniforms: an odd pattern is repeated to become even (as in SVG). */
export function dashValues(dash: readonly number[] | null): { d: number[]; total: number; on: number } {
  if (!dash?.length) return { d: new Array(8).fill(0), total: 0, on: 1 };
  const even = dash.length % 2 ? [...dash, ...dash] : [...dash];
  const d = even.slice(0, 8);
  while (d.length < 8) d.push(0);
  const total = d.reduce((s, v) => s + v, 0);
  const on = d.reduce((s, v, i) => (i % 2 ? s : s + v), 0);
  return { d, total, on: total > 0 ? on / total : 1 };
}

/**
 * Per-cell random shift range and whether neighbour cells must be looked at
 * (the shape, its offset or its shift reach past half a cell).
 */
export function patternReach(p: Extract<Extract<StyledBatch, { kind: 'fill' }>['paint'], { kind: 'pattern' }>): { jitter: [number, number]; reach: number } {
  const ext = Math.hypot(p.half[0], p.half[1]) + p.strokeWidth / 2;
  const jitter: [number, number] = [Math.max(0, p.size[0] - 2 * ext) * p.jitter, Math.max(0, p.size[1] - 2 * ext) * p.jitter];
  const out = ext + Math.hypot(p.markOffset[0], p.markOffset[1]) + Math.max(jitter[0], jitter[1]) / 2;
  return { jitter, reach: out > Math.min(p.size[0], p.size[1]) / 2 ? 1 : 0 };
}

export class StyledRenderer {
  private readonly gl: WebGL2RenderingContext;
  private readonly stroke: Program;
  private readonly hatch: Program;
  private readonly tile: Program;
  private readonly solid: Program;
  private readonly pattern: Program;
  private readonly marker: Program;
  private atlas: AtlasSource | null = null;
  private texture: WebGLTexture | null = null;
  private frameNo = 0;
  private current: Program | null = null;
  private premul: boolean | null = null;

  constructor(gl: WebGL2RenderingContext, compile: (vs: string, fs: string, attribs: string[], uniforms: string[]) => Omit<Program, 'frame'>) {
    this.gl = gl;
    const make = (vs: string, fs: string, attribs: string[], uniforms: string[]): Program => ({ ...compile(vs, fs, attribs, uniforms), frame: -1 });
    this.stroke = make(STROKE_VS, STROKE_FS, ['a_seg', 'a_meta'], [...FRAME_UNIFORMS, ...DASH_UNIFORMS, 'u_width', 'u_blur', 'u_unit', 'u_color', 'u_cap']);
    const SOLID_FS = `#version 300 es
precision highp float;
precision highp int;
in vec2 v_world;
uniform vec4 u_color;
out vec4 outColor;
void main() { outColor = u_color; }`;
    this.solid = make(AREA_VS, SOLID_FS, ['a_pos'], [...FRAME_UNIFORMS, 'u_color']);
    this.hatch = make(AREA_VS, HATCH_FS, ['a_pos'], [...FRAME_UNIFORMS, ...DASH_UNIFORMS, 'u_color', 'u_dir', 'u_spacing', 'u_width', 'u_offset', 'u_unit']);
    this.tile = make(AREA_VS, TILE_FS, ['a_pos'], [...FRAME_UNIFORMS, 'u_atlas', 'u_rect', 'u_tile', 'u_rot', 'u_shift', 'u_opacity', 'u_unit']);
    this.pattern = make(AREA_VS, PATTERN_FS, ['a_pos'], [...FRAME_UNIFORMS, ...PATTERN_UNIFORMS]);
    this.marker = make(MARKER_VS, MARKER_FS, ['a_i0', 'a_i1'], [...FRAME_UNIFORMS, 'u_unit', 'u_offset', 'u_anchor', 'u_fit', 'u_aspect', 'u_strokeW', 'u_kind', 'u_shape', 'u_sp', 'u_fill', 'u_stroke', 'u_atlas', 'u_rect', 'u_opacity']);
  }

  useAtlas(atlas: AtlasSource): void {
    const gl = this.gl;
    this.atlas = atlas;
    if (!this.texture) {
      this.texture = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, this.texture);
      gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, atlas.size, atlas.size);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    atlas.attach((source, x, y) => {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.texture);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
      gl.texSubImage2D(gl.TEXTURE_2D, 0, x, y, gl.RGBA, gl.UNSIGNED_BYTE, source);
      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    });
  }

  // ── Upload ───────────────────────────────────────────────────────────

  upload(batches: readonly StyledBatch[]): GpuStyled[] {
    const gl = this.gl;
    const out: GpuStyled[] = [];
    for (const b of batches) {
      const vao = gl.createVertexArray()!;
      gl.bindVertexArray(vao);
      const buf = gl.createBuffer()!;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      const common = { batch: b, vao, buffers: [buf], visible: false, hit: null };
      if (b.kind === 'stroke') {
        gl.bufferData(gl.ARRAY_BUFFER, b.segments, gl.STATIC_DRAW);
        const stride = STROKE_STRIDE * 4;
        this.attrib(this.stroke.a.a_seg, 4, stride, 0, 1);
        this.attrib(this.stroke.a.a_meta, 2, stride, 16, 1);
        out.push({ ...common, count: b.segments.length / STROKE_STRIDE });
      } else if (b.kind === 'marker') {
        gl.bufferData(gl.ARRAY_BUFFER, b.instances, gl.STATIC_DRAW);
        const stride = MARKER_STRIDE * 4;
        this.attrib(this.marker.a.a_i0, 4, stride, 0, 1);
        this.attrib(this.marker.a.a_i1, 1, stride, 16, 1);
        out.push({ ...common, count: b.instances.length / MARKER_STRIDE });
      } else {
        gl.bufferData(gl.ARRAY_BUFFER, b.positions, gl.STATIC_DRAW);
        // Every area program reads a_pos at the same location (compiled from one vertex shader).
        this.attrib(this.solid.a.a_pos, 2, 0, 0, 0);
        out.push({ ...common, count: b.positions.length / 2 });
      }
    }
    gl.bindVertexArray(null);
    return out;
  }

  private attrib(loc: number, size: number, stride: number, offset: number, divisor: number): void {
    if (loc < 0) return;
    const gl = this.gl;
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, size, gl.FLOAT, false, stride, offset);
    gl.vertexAttribDivisor(loc, divisor);
  }

  release(list: readonly GpuStyled[]): void {
    for (const s of list) {
      this.gl.deleteVertexArray(s.vao);
      s.buffers.forEach((b) => this.gl.deleteBuffer(b));
    }
  }

  // ── Frame ────────────────────────────────────────────────────────────

  /**
   * Decides what shows this frame and places its images in the atlas. If
   * the atlas starts over midway (its page filled up), the looked-up
   * rectangles are void: look everything up once more.
   */
  prepare(layers: readonly (readonly GpuStyled[])[], f: StyledFrame): void {
    this.frameNo++;
    this.current = null;
    this.premul = null;
    const hw = f.viewPx[0] / 2 / f.pxPerM;
    const hh = f.viewPx[1] / 2 / f.pxPerM;
    const view = [f.cam[0] - hw, f.cam[1] - hh, f.cam[0] + hw, f.cam[1] + hh] as const;
    const atlas = this.atlas;
    atlas?.beginFrame();
    for (let pass = 0; pass < 2; pass++) {
      const generation = atlas?.generation ?? 0;
      for (const list of layers)
        for (const s of list) {
          const b = s.batch;
          s.visible = inScale(b, f.scaleDenominator) && batchInView(b, view, f.pxPerM, f.dpr) && batchLegible(b, f.pxPerM, f.dpr);
          s.hit = null;
          if (!s.visible) continue;
          const image = batchImage(b);
          if (image) {
            s.hit = atlas?.lookup(image, batchImagePx(b, f.pxPerM, f.dpr)) ?? null;
            if (!s.hit) s.visible = false;
          }
        }
      if ((atlas?.generation ?? 0) === generation) break;
    }
  }

  private use(p: Program, f: StyledFrame): void {
    const gl = this.gl;
    if (this.current !== p) {
      gl.useProgram(p.program);
      this.current = p;
    }
    if (p.frame === this.frameNo) return;
    p.frame = this.frameNo;
    gl.uniform2f(p.u.u_cam, f.cam[0], f.cam[1]);
    gl.uniform1f(p.u.u_pxPerM, f.pxPerM);
    gl.uniform1f(p.u.u_dpr, f.dpr);
    gl.uniform2f(p.u.u_viewPx, f.viewPx[0], f.viewPx[1]);
  }

  private dash(p: Program, dash: readonly number[] | null, offset: number): void {
    const gl = this.gl;
    const v = dashValues(dash);
    gl.uniform4f(p.u.u_dash0, v.d[0], v.d[1], v.d[2], v.d[3]);
    gl.uniform4f(p.u.u_dash1, v.d[4], v.d[5], v.d[6], v.d[7]);
    gl.uniform1f(p.u.u_dashTotal, v.total);
    gl.uniform1f(p.u.u_dashOn, v.on);
    gl.uniform1f(p.u.u_dashOffset, offset);
  }

  private premultiplied(on: boolean): void {
    if (this.premul === on) return;
    this.premul = on;
    const gl = this.gl;
    if (on) gl.blendFuncSeparate(gl.ONE, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    else gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  }

  /** Draws a layer's prepared batches. Other programs may run between layers: state is re-set per layer. */
  draw(list: readonly GpuStyled[], f: StyledFrame): void {
    if (!list.length) return;
    const gl = this.gl;
    this.current = null;
    this.premul = null;
    if (this.texture) {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.texture);
    }
    for (const s of list) {
      if (!s.visible) continue;
      const b = s.batch;
      gl.bindVertexArray(s.vao);
      if (b.kind === 'stroke') {
        const p = this.stroke;
        this.use(p, f);
        this.premultiplied(false);
        gl.uniform1f(p.u.u_width, b.width);
        gl.uniform1f(p.u.u_blur, b.blur);
        gl.uniform1i(p.u.u_unit, b.unit === 'world' ? 0 : 1);
        gl.uniform4fv(p.u.u_color, b.color);
        gl.uniform1i(p.u.u_cap, b.cap === 'round' ? 1 : b.cap === 'square' ? 2 : 0);
        this.dash(p, b.dash, b.dashOffset);
        gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, s.count);
      } else if (b.kind === 'fill') {
        const paint = b.paint;
        if (paint.kind === 'solid') {
          this.use(this.solid, f);
          this.premultiplied(false);
          gl.uniform4fv(this.solid.u.u_color, paint.color);
        } else if (paint.kind === 'hatch') {
          const p = this.hatch;
          this.use(p, f);
          this.premultiplied(false);
          gl.uniform4fv(p.u.u_color, paint.color);
          gl.uniform2f(p.u.u_dir, Math.cos(paint.angle), Math.sin(paint.angle));
          gl.uniform1f(p.u.u_spacing, paint.spacing);
          gl.uniform1f(p.u.u_width, paint.width);
          gl.uniform1f(p.u.u_offset, paint.offset);
          gl.uniform1i(p.u.u_unit, paint.unit === 'world' ? 0 : 1);
          this.dash(p, paint.dash, paint.dashOffset);
        } else if (paint.kind === 'pattern') {
          const p = this.pattern;
          this.use(p, f);
          this.premultiplied(true);
          const r = patternReach(paint);
          gl.uniform2f(p.u.u_size, paint.size[0], paint.size[1]);
          gl.uniform2f(p.u.u_rot, Math.cos(paint.angle), Math.sin(paint.angle));
          gl.uniform2f(p.u.u_shift, paint.offset[0], paint.offset[1]);
          gl.uniform1i(p.u.u_stagger, paint.stagger ? 1 : 0);
          gl.uniform2f(p.u.u_jitter, r.jitter[0], r.jitter[1]);
          gl.uniform1f(p.u.u_coverage, paint.coverage);
          gl.uniform1f(p.u.u_seed, paint.seed);
          gl.uniform1i(p.u.u_reach, r.reach);
          gl.uniform1i(p.u.u_unit, paint.unit === 'world' ? 0 : 1);
          gl.uniform1i(p.u.u_shape, SHAPE_INDEX.get(paint.shape) ?? 0);
          gl.uniform4fv(p.u.u_sp, paint.params);
          gl.uniform2f(p.u.u_half, paint.half[0], paint.half[1]);
          gl.uniform2f(p.u.u_markOff, paint.markOffset[0], paint.markOffset[1]);
          gl.uniform2f(p.u.u_markRot, Math.cos(paint.markRotation), Math.sin(paint.markRotation));
          gl.uniform4fv(p.u.u_fill, paint.fill ?? NONE);
          gl.uniform4fv(p.u.u_stroke, paint.stroke ?? NONE);
          gl.uniform1f(p.u.u_strokeW, paint.strokeWidth);
          gl.uniform1f(p.u.u_tint, paint.tint);
          gl.uniform1f(p.u.u_opacity, paint.opacity);
        } else {
          const hit = s.hit!;
          const p = this.tile;
          this.use(p, f);
          this.premultiplied(true);
          gl.uniform1i(p.u.u_atlas, 0);
          gl.uniform4f(p.u.u_rect, hit.uv[0], hit.uv[1], hit.uv[2], hit.uv[3]);
          gl.uniform2f(p.u.u_tile, paint.size[0], paint.size[1]);
          gl.uniform2f(p.u.u_rot, Math.cos(paint.angle), Math.sin(paint.angle));
          gl.uniform2f(p.u.u_shift, paint.offset[0], paint.offset[1]);
          gl.uniform1f(p.u.u_opacity, paint.opacity);
          gl.uniform1i(p.u.u_unit, paint.unit === 'world' ? 0 : 1);
        }
        gl.drawArrays(gl.TRIANGLES, 0, s.count);
      } else {
        const p = this.marker;
        const look = b.look;
        this.use(p, f);
        this.premultiplied(true);
        gl.uniform1i(p.u.u_unit, b.unit === 'world' ? 0 : 1);
        gl.uniform2f(p.u.u_offset, b.offset[0], b.offset[1]);
        gl.uniform2f(p.u.u_anchor, b.anchor[0], b.anchor[1]);
        gl.uniform1f(p.u.u_opacity, b.opacity);
        gl.uniform1i(p.u.u_atlas, 0);
        if (look.kind === 'shape') {
          gl.uniform4f(p.u.u_rect, 0, 0, 0, 0);
          gl.uniform1f(p.u.u_aspect, 1);
          gl.uniform1i(p.u.u_kind, 0);
          gl.uniform1i(p.u.u_fit, 0);
          gl.uniform1i(p.u.u_shape, SHAPE_INDEX.get(look.shape) ?? 0);
          gl.uniform4fv(p.u.u_sp, look.params);
          gl.uniform4fv(p.u.u_fill, look.fill ?? NONE);
          gl.uniform4fv(p.u.u_stroke, look.stroke ?? NONE);
          gl.uniform1f(p.u.u_strokeW, look.strokeWidth);
        } else {
          const hit = s.hit!;
          gl.uniform4f(p.u.u_rect, hit.uv[0], hit.uv[1], hit.uv[2], hit.uv[3]);
          gl.uniform1f(p.u.u_aspect, hit.aspect);
          gl.uniform1i(p.u.u_kind, 1);
          gl.uniform1i(p.u.u_fit, look.fit === 'width' ? 1 : 2);
          gl.uniform1f(p.u.u_strokeW, 0);
        }
        gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, s.count);
      }
    }
    this.premultiplied(false);
    this.current = null;
    gl.bindVertexArray(null);
  }

  dispose(): void {
    const gl = this.gl;
    // The atlas is not detached: after a backend switch it already feeds the new backend.
    for (const p of [this.stroke, this.hatch, this.tile, this.solid, this.pattern, this.marker]) gl.deleteProgram(p.program);
    if (this.texture) gl.deleteTexture(this.texture);
  }
}
