import type { AtlasSource, FrameState, LineBatch, RenderBackend, RGBA, SceneLayer } from '../types';
import { FILL_FS, FILL_VS, LINE_FS, LINE_VS, POINT_FS, POINT_VS } from './shaders';
import { StyledRenderer, type GpuStyled } from './styledRenderer';

interface Program {
  program: WebGLProgram;
  uniforms: Record<string, WebGLUniformLocation | null>;
  attribs: Record<string, number>;
}

interface GpuBatch {
  vao: WebGLVertexArrayObject;
  buffers: WebGLBuffer[];
  count: number;
  color: RGBA;
  dash?: readonly number[] | null;
  size?: number;
  shape?: number;
}

interface GpuLayer {
  lines: GpuBatch[];
  fills: GpuBatch[];
  points: GpuBatch[];
  styled: GpuStyled[];
}

const SHAPES = { ring: 0, cross: 1, triangle: 2 } as const;

/** An off-screen colour target: a framebuffer with one renderbuffer (multisampled with anti-aliasing) or texture. */
interface Target {
  fbo: WebGLFramebuffer;
  rb: WebGLRenderbuffer | null;
  tex: WebGLTexture | null;
}

/** A full-view triangle that copies the kept base into the frame's target. */
const COPY_VS = `#version 300 es
out vec2 v_uv;
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  v_uv = p;
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}`;
const COPY_FS = `#version 300 es
precision mediump float;
uniform sampler2D u_base;
in vec2 v_uv;
out vec4 o;
void main() { o = texelFetch(u_base, ivec2(gl_FragCoord.xy), 0); }`;

/**
 * WebGL2 implementation of the RenderBackend contract. The underlays and
 * the persistent layers are drawn off-screen (`draw`, multisampled with
 * anti-aliasing) and resolved into a texture (`base`); a frame copies that
 * texture into `draw`, adds the overlays, resolves the result into `out`
 * and copies that onto the canvas (a multisampled copy needs identical
 * formats, and the canvas has no alpha). When only the overlays changed (a hover or a selection,
 * FrameState.keepBase) the base is not drawn again: on a large drawing a
 * pointer move used to redraw every segment for a new highlight.
 * Anti-aliasing is the target's own 4× MSAA; the context has none (a copy
 * cannot write into a multisampled framebuffer).
 */
export class WebGL2Backend implements RenderBackend {
  readonly kind = 'webgl2' as const;
  label = 'WebGL2';
  private gl!: WebGL2RenderingContext;
  private canvas!: HTMLCanvasElement;
  private dpr = 1;
  private line!: Program;
  private fill!: Program;
  private point!: Program;
  private styled!: StyledRenderer;
  private layers = new Map<string, GpuLayer>();
  private samples = 0;
  private copyProgram!: Program;
  private draw: Target | null = null;
  private base: Target | null = null;
  private out: Target | null = null;
  private targetSize: readonly [number, number] = [0, 0];
  /** What the base holds (view, scale, order, background), or '' when it must be drawn again. */
  private baseKey = '';
  /** The overlays of the last frame: uploading them leaves the base as it is. */
  private overlayIds: ReadonlySet<string> = new Set();

  async init(canvas: HTMLCanvasElement, opts: { antialias?: boolean } = {}): Promise<void> {
    const gl = canvas.getContext('webgl2', { antialias: false, alpha: false, premultipliedAlpha: false, powerPreference: 'high-performance' });
    if (!gl) throw new Error('WebGL2 desteklenmiyor');
    this.gl = gl;
    this.samples = opts.antialias === false ? 0 : Math.min(4, gl.getParameter(gl.MAX_SAMPLES) as number);
    this.canvas = canvas;
    this.line = this.createProgram(LINE_VS, LINE_FS, ['a_pos', 'a_dist'], ['u_offset', 'u_scale', 'u_color', 'u_dash', 'u_pxPerUnit']);
    this.fill = this.createProgram(FILL_VS, FILL_FS, ['a_pos'], ['u_offset', 'u_scale', 'u_color']);
    this.point = this.createProgram(POINT_VS, POINT_FS, ['a_pos'], ['u_offset', 'u_scale', 'u_color', 'u_size', 'u_dpr', 'u_shape']);
    this.copyProgram = this.createProgram(COPY_VS, COPY_FS, [], ['u_base']);
    this.styled = new StyledRenderer(gl, (vs, fs, a, u) => {
      const p = this.createProgram(vs, fs, a, u);
      return { program: p.program, a: p.attribs, u: p.uniforms };
    });
    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    const ext = gl.getExtension('WEBGL_debug_renderer_info');
    const renderer = ext ? String(gl.getParameter(ext.UNMASKED_RENDERER_WEBGL)) : '';
    this.label = renderer ? `WebGL2 · ${renderer.replace(/^ANGLE \((.*)\)$/, '$1').split(',')[1]?.trim() ?? renderer}` : 'WebGL2';
  }

  get antialiased(): boolean {
    return this.samples > 0;
  }

  useAtlas(atlas: AtlasSource): void {
    this.baseKey = '';
    this.styled.useAtlas(atlas);
  }

  resize(width: number, height: number, dpr: number): void {
    this.dpr = dpr;
    this.canvas.width = Math.max(1, Math.round(width * dpr));
    this.canvas.height = Math.max(1, Math.round(height * dpr));
    this.baseKey = '';
  }

  upload(layer: SceneLayer): void {
    this.remove(layer.id);
    if (!this.overlayIds.has(layer.id)) this.baseKey = '';
    const gl = this.gl;
    const g: GpuLayer = { lines: [], fills: [], points: [], styled: layer.styled?.length ? this.styled.upload(layer.styled) : [] };
    for (const b of layer.lines) {
      if (!b.positions.length) continue;
      g.lines.push(this.makeBatch(this.line, { a_pos: [b.positions, 2], a_dist: [b.distances, 1] }, b.positions.length / 2, b.color, b));
    }
    for (const f of layer.fills) {
      if (!f.positions.length) continue;
      g.fills.push(this.makeBatch(this.fill, { a_pos: [f.positions, 2] }, f.positions.length / 2, f.color));
    }
    for (const p of layer.points) {
      if (!p.positions.length) continue;
      const batch = this.makeBatch(this.point, { a_pos: [p.positions, 2] }, p.positions.length / 2, p.color);
      batch.size = p.size;
      batch.shape = SHAPES[p.shape];
      g.points.push(batch);
    }
    gl.bindVertexArray(null);
    this.layers.set(layer.id, g);
  }

  remove(id: string): void {
    const g = this.layers.get(id);
    if (!g) return;
    if (!this.overlayIds.has(id)) this.baseKey = '';
    for (const b of [...g.lines, ...g.fills, ...g.points]) {
      this.gl.deleteVertexArray(b.vao);
      b.buffers.forEach((buf) => this.gl.deleteBuffer(buf));
    }
    this.styled.release(g.styled);
    this.layers.delete(id);
  }

  render(frame: FrameState): void {
    const gl = this.gl;
    const { view } = frame;
    const w = this.canvas.width;
    const h = this.canvas.height;
    this.ensureTargets(w, h);
    this.overlayIds = new Set(frame.overlays);
    const key = `${view.center.x},${view.center.y},${view.scale},${w}x${h},${frame.scaleDenominator},${frame.clearColor.join()},${frame.underlays.join()}|${frame.order.join()}`;
    const drawBase = !frame.keepBase || key !== this.baseKey;
    gl.viewport(0, 0, w, h);

    const sx = (2 * view.scale) / view.width;
    const sy = (2 * view.scale) / view.height;
    const pxPerUnit = view.scale * this.dpr;
    const offset = [view.center.x, view.center.y] as const;

    const setCommon = (p: Program) => {
      gl.useProgram(p.program);
      gl.uniform2f(p.uniforms.u_offset, offset[0], offset[1]);
      gl.uniform2f(p.uniforms.u_scale, sx, sy);
    };

    const styledFrame = {
      cam: offset,
      pxPerM: pxPerUnit,
      dpr: this.dpr,
      viewPx: [this.canvas.width, this.canvas.height] as const,
      scaleDenominator: frame.scaleDenominator,
    };
    // Visibility and atlas images for the whole frame first, then the draw calls.
    const drawn = [...(drawBase ? [...frame.underlays, ...frame.order] : []), ...frame.overlays].flatMap((id) => this.layers.get(id)?.styled ?? []);
    this.styled.prepare(drawn.length ? [drawn] : [], styledFrame);
    const pass = (ids: readonly string[]) => {
      const layers = ids.map((id) => this.layers.get(id)).filter((l): l is GpuLayer => !!l);
      // Each layer draws its styled symbols whole (symbol levels), under the plain lines and points.
      for (const l of layers) {
        if (l.fills.length) {
          setCommon(this.fill);
          for (const f of l.fills) {
            gl.uniform4fv(this.fill.uniforms.u_color, f.color);
            gl.bindVertexArray(f.vao);
            gl.drawArrays(gl.TRIANGLES, 0, f.count);
          }
        }
        this.styled.draw(l.styled, styledFrame);
      }
      setCommon(this.line);
      gl.uniform1f(this.line.uniforms.u_pxPerUnit, pxPerUnit);
      for (const l of layers)
        for (const ln of l.lines) {
          gl.uniform4fv(this.line.uniforms.u_color, ln.color);
          const d = ln.dash ?? [];
          gl.uniform4f(this.line.uniforms.u_dash, (d[0] ?? 0) * this.dpr, (d[1] ?? 0) * this.dpr, (d[2] ?? 0) * this.dpr, (d[3] ?? 0) * this.dpr);
          gl.bindVertexArray(ln.vao);
          gl.drawArrays(gl.LINES, 0, ln.count);
        }
      setCommon(this.point);
      gl.uniform1f(this.point.uniforms.u_dpr, this.dpr);
      for (const l of layers)
        for (const p of l.points) {
          gl.uniform4fv(this.point.uniforms.u_color, p.color);
          gl.uniform1f(this.point.uniforms.u_size, (p.size ?? 7) * this.dpr);
          gl.uniform1i(this.point.uniforms.u_shape, p.shape ?? 0);
          gl.bindVertexArray(p.vao);
          gl.drawArrays(gl.POINTS, 0, p.count);
        }
    };

    const draw = this.draw!;
    const base = this.base!;
    gl.bindFramebuffer(gl.FRAMEBUFFER, draw.fbo);
    if (drawBase) {
      const [r, g, b] = frame.clearColor;
      gl.clearColor(r, g, b, 1);
      gl.clear(gl.COLOR_BUFFER_BIT);
      pass(frame.underlays);
      pass(frame.order);
      this.copy(draw.fbo, base.fbo, w, h);
      this.baseKey = key;
    } else {
      // Every sample of a pixel takes the resolved colour: the same picture the base was resolved from.
      this.drawTexture(base.tex!, draw.fbo);
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, draw.fbo);
    pass(frame.overlays);
    gl.bindVertexArray(null);
    this.copy(draw.fbo, this.out!.fbo, w, h);
    this.drawTexture(this.out!.tex!, null);
  }

  /** Draws a target-sized texture over a framebuffer, pixel for pixel. */
  private drawTexture(tex: WebGLTexture, into: WebGLFramebuffer | null): void {
    const gl = this.gl;
    gl.bindFramebuffer(gl.FRAMEBUFFER, into);
    gl.disable(gl.BLEND);
    gl.useProgram(this.copyProgram.program);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.uniform1i(this.copyProgram.uniforms.u_base, 0);
    gl.bindVertexArray(null);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.enable(gl.BLEND);
  }

  /** The off-screen targets at the canvas's size (made again when it changes). */
  private ensureTargets(w: number, h: number): void {
    if (this.base && this.targetSize[0] === w && this.targetSize[1] === h) return;
    const gl = this.gl;
    this.releaseTargets();
    const rb = gl.createRenderbuffer()!;
    gl.bindRenderbuffer(gl.RENDERBUFFER, rb);
    if (this.samples > 0) gl.renderbufferStorageMultisample(gl.RENDERBUFFER, this.samples, gl.RGBA8, w, h);
    else gl.renderbufferStorage(gl.RENDERBUFFER, gl.RGBA8, w, h);
    const drawFbo = gl.createFramebuffer()!;
    gl.bindFramebuffer(gl.FRAMEBUFFER, drawFbo);
    gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.RENDERBUFFER, rb);
    this.draw = { fbo: drawFbo, rb, tex: null };
    const textured = (): Target => {
      const tex = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.texStorage2D(gl.TEXTURE_2D, 1, gl.RGBA8, w, h);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      const fbo = gl.createFramebuffer()!;
      gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
      return { fbo, rb: null, tex };
    };
    this.base = textured();
    this.out = textured();
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindRenderbuffer(gl.RENDERBUFFER, null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    this.targetSize = [w, h];
    this.baseKey = '';
  }

  /** Copies a whole target (a multisampled one into the canvas resolves it). */
  private copy(from: WebGLFramebuffer, to: WebGLFramebuffer | null, w: number, h: number): void {
    const gl = this.gl;
    gl.bindFramebuffer(gl.READ_FRAMEBUFFER, from);
    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, to);
    gl.blitFramebuffer(0, 0, w, h, 0, 0, w, h, gl.COLOR_BUFFER_BIT, gl.NEAREST);
  }

  private releaseTargets(): void {
    for (const t of [this.draw, this.base, this.out]) {
      if (!t) continue;
      this.gl.deleteFramebuffer(t.fbo);
      if (t.rb) this.gl.deleteRenderbuffer(t.rb);
      if (t.tex) this.gl.deleteTexture(t.tex);
    }
    this.draw = this.base = this.out = null;
  }

  dispose(): void {
    for (const id of [...this.layers.keys()]) this.remove(id);
    this.releaseTargets();
    for (const p of [this.line, this.fill, this.point, this.copyProgram]) if (p) this.gl.deleteProgram(p.program);
    this.styled?.dispose();
  }

  private makeBatch(
    prog: Program,
    attributes: Record<string, [Float32Array, number]>,
    count: number,
    color: RGBA,
    line?: LineBatch,
  ): GpuBatch {
    const gl = this.gl;
    const vao = gl.createVertexArray()!;
    gl.bindVertexArray(vao);
    const buffers: WebGLBuffer[] = [];
    for (const [name, [data, size]] of Object.entries(attributes)) {
      const loc = prog.attribs[name];
      if (loc < 0) continue;
      const buf = gl.createBuffer()!;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf);
      gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 0, 0);
      buffers.push(buf);
    }
    return { vao, buffers, count, color, dash: line?.dash ?? null };
  }

  private createProgram(vs: string, fs: string, attribs: string[], uniforms: string[]): Program {
    const gl = this.gl;
    const compile = (type: number, src: string) => {
      const s = gl.createShader(type)!;
      gl.shaderSource(s, src);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(`Shader derlenemedi: ${gl.getShaderInfoLog(s)}`);
      return s;
    };
    const program = gl.createProgram()!;
    gl.attachShader(program, compile(gl.VERTEX_SHADER, vs));
    gl.attachShader(program, compile(gl.FRAGMENT_SHADER, fs));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(`Program bağlanamadı: ${gl.getProgramInfoLog(program)}`);
    return {
      program,
      attribs: Object.fromEntries(attribs.map((a) => [a, gl.getAttribLocation(program, a)])),
      uniforms: Object.fromEntries(uniforms.map((u) => [u, gl.getUniformLocation(program, u)])),
    };
  }
}
