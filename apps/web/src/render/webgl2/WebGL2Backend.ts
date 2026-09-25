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

  async init(canvas: HTMLCanvasElement, opts: { antialias?: boolean } = {}): Promise<void> {
    const gl = canvas.getContext('webgl2', { antialias: opts.antialias !== false, alpha: false, premultipliedAlpha: false, powerPreference: 'high-performance' });
    if (!gl) throw new Error('WebGL2 desteklenmiyor');
    this.gl = gl;
    this.canvas = canvas;
    this.line = this.createProgram(LINE_VS, LINE_FS, ['a_pos', 'a_dist'], ['u_offset', 'u_scale', 'u_color', 'u_dash', 'u_pxPerUnit']);
    this.fill = this.createProgram(FILL_VS, FILL_FS, ['a_pos'], ['u_offset', 'u_scale', 'u_color']);
    this.point = this.createProgram(POINT_VS, POINT_FS, ['a_pos'], ['u_offset', 'u_scale', 'u_color', 'u_size', 'u_dpr', 'u_shape']);
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

  useAtlas(atlas: AtlasSource): void {
    this.styled.useAtlas(atlas);
  }

  resize(width: number, height: number, dpr: number): void {
    this.dpr = dpr;
    this.canvas.width = Math.max(1, Math.round(width * dpr));
    this.canvas.height = Math.max(1, Math.round(height * dpr));
  }

  upload(layer: SceneLayer): void {
    this.remove(layer.id);
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
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    const [r, g, b] = frame.clearColor;
    gl.clearColor(r, g, b, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);

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
    const drawn = [...frame.underlays, ...frame.order, ...frame.overlays].flatMap((id) => this.layers.get(id)?.styled ?? []);
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

    pass(frame.underlays);
    pass(frame.order);
    pass(frame.overlays);
    gl.bindVertexArray(null);
  }

  dispose(): void {
    for (const id of [...this.layers.keys()]) this.remove(id);
    for (const p of [this.line, this.fill, this.point]) if (p) this.gl.deleteProgram(p.program);
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
