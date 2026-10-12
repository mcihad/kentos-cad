import { texSize, type ComposePlan, type ComposeStep, type MapPipe, type Reads, type Tex } from '../composePlan';
import {
  COMPOSE_BLEND_FS,
  COMPOSE_BLUR_WIDE_FS,
  COMPOSE_BOX_FS,
  COMPOSE_BOX_VS,
  COMPOSE_COLUMNS_FS,
  COMPOSE_DOWN_FS,
  COMPOSE_DOWN_WIDE_FS,
  COMPOSE_EFFECT_FS,
  COMPOSE_FULL_VS,
  COMPOSE_H_FS,
  COMPOSE_OVER_FS,
  COMPOSE_PACK_FS,
  COMPOSE_ROWS_FS,
  COMPOSE_V_FS,
} from './composeShaders';

/**
 * Katman çizimi and draw effects on WebGL2 (docs/adr/0226 §2–§4): runs a composed frame's steps (../composePlan.ts)
 * with the GLSL twin of the shared shader (./composeShaders.ts). The layer's and the group's pictures are drawn
 * multisampled like the drawing and resolved before they are read; the maps (packed, four pixels a texel; read
 * between texels by a linear filter), the mask and the backdrop are textures. Kept between frames at one size and
 * sample count.
 */

export interface GlProgram {
  program: WebGLProgram;
  a: Record<string, number>;
  u: Record<string, WebGLUniformLocation | null>;
}

export type MakeProgram = (vs: string, fs: string, attribs: string[], uniforms: string[]) => GlProgram;

/** What running the steps needs of the backend. */
export interface GlComposeHost {
  /** Binds the drawing's picture as the framebuffer drawn into. */
  picture(): void;
  /** Resolves the drawing's picture into the framebuffer `into` (a texture's). */
  copyPicture(into: WebGLFramebuffer): void;
  /** Draws the frame's layer `layer`'s batches `from` … `to` (and its plain parts at its ends) into the bound framebuffer. */
  draw(layer: number, from: number, to: number): void;
}

/** A picture drawn into (a multisampled renderbuffer, or the texture itself) and the texture it is read from. */
interface Drawn {
  fbo: WebGLFramebuffer;
  rb: WebGLRenderbuffer | null;
  read: WebGLFramebuffer;
  tex: WebGLTexture;
  /** Drawn into since it was last resolved. */
  dirty: boolean;
}

/** A texture with its framebuffer. */
interface Plain {
  fbo: WebGLFramebuffer;
  tex: WebGLTexture;
  size: [number, number];
}

/** The pass's numbers (`Params`' eight vec4s) and its six pictures. */
const NUMBERS = ['u_a', 'u_b', 'u_c', 'u_d', 'u_e', 'u_f', 'u_g', 'u_h'] as const;
const PICTURES = ['u_srcA', 'u_srcB', 'u_srcC', 'u_srcD', 'u_srcE', 'u_srcF'] as const;
const UNIFORMS = [...NUMBERS, ...PICTURES];

export class WebGL2Composer {
  private readonly gl: WebGL2RenderingContext;
  private readonly over: GlProgram;
  private readonly blend: GlProgram;
  private readonly effect: GlProgram;
  private readonly maps: Record<MapPipe, GlProgram>;
  private readonly box: GlProgram;
  private readonly boxVao: WebGLVertexArrayObject;
  private readonly boxBuffer: WebGLBuffer;
  private readonly none: WebGLTexture;
  private size: readonly [number, number] = [0, 0];
  private samples = 0;
  private layer: Drawn | null = null;
  private group: Drawn | null = null;
  private readonly plain = new Map<Tex, Plain>();

  constructor(gl: WebGL2RenderingContext, make: MakeProgram) {
    this.gl = gl;
    const full = (fs: string) => make(COMPOSE_FULL_VS, fs, [], UNIFORMS);
    this.over = full(COMPOSE_OVER_FS);
    this.blend = full(COMPOSE_BLEND_FS);
    this.effect = full(COMPOSE_EFFECT_FS);
    this.maps = {
      pack: full(COMPOSE_PACK_FS),
      rows: full(COMPOSE_ROWS_FS),
      columns: full(COMPOSE_COLUMNS_FS),
      h: full(COMPOSE_H_FS),
      v: full(COMPOSE_V_FS),
      down: full(COMPOSE_DOWN_FS),
      downWide: full(COMPOSE_DOWN_WIDE_FS),
      blurWide: full(COMPOSE_BLUR_WIDE_FS),
    };
    this.box = make(COMPOSE_BOX_VS, COMPOSE_BOX_FS, ['a_centre', 'a_half', 'a_dir', 'a_grow'], ['u_a']);
    this.boxVao = gl.createVertexArray()!;
    this.boxBuffer = gl.createBuffer()!;
    gl.bindVertexArray(this.boxVao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.boxBuffer);
    // compose.layout.json's boxInstance: 28 bytes.
    (
      [
        ['a_centre', 2, 0],
        ['a_half', 2, 8],
        ['a_dir', 2, 16],
        ['a_grow', 1, 24],
      ] as const
    ).forEach(([name, size, offset]) => {
      const loc = this.box.a[name];
      if (loc < 0) return;
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 28, offset);
      gl.vertexAttribDivisor(loc, 1);
    });
    gl.bindVertexArray(null);
    this.none = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, this.none);
    gl.texStorage2D(gl.TEXTURE_2D, 1, gl.R8, 1, 1);
    gl.bindTexture(gl.TEXTURE_2D, null);
  }

  /** The pictures for a new size or sample count. */
  private ensure(size: readonly [number, number], samples: number): void {
    if (this.size[0] === size[0] && this.size[1] === size[1] && this.samples === samples) return;
    this.release();
    this.size = [size[0], size[1]];
    this.samples = samples;
  }

  private texture(format: number, w: number, h: number, linear: boolean): { fbo: WebGLFramebuffer; tex: WebGLTexture } {
    const gl = this.gl;
    const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texStorage2D(gl.TEXTURE_2D, 1, format, w, h);
    const filter = linear ? gl.LINEAR : gl.NEAREST;
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer()!;
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    gl.bindTexture(gl.TEXTURE_2D, null);
    return { fbo, tex };
  }

  private drawn(): Drawn {
    const gl = this.gl;
    const [w, h] = this.size;
    const read = this.texture(gl.RGBA8, w, h, false);
    if (this.samples <= 1) return { fbo: read.fbo, rb: null, read: read.fbo, tex: read.tex, dirty: false };
    const rb = gl.createRenderbuffer()!;
    gl.bindRenderbuffer(gl.RENDERBUFFER, rb);
    gl.renderbufferStorageMultisample(gl.RENDERBUFFER, this.samples, gl.RGBA8, w, h);
    const fbo = gl.createFramebuffer()!;
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.RENDERBUFFER, rb);
    gl.bindRenderbuffer(gl.RENDERBUFFER, null);
    return { fbo, rb, read: read.fbo, tex: read.tex, dirty: false };
  }

  private plainOf(t: Tex): Plain {
    let p = this.plain.get(t);
    if (!p) {
      const gl = this.gl;
      const size = texSize(t, this.size);
      // The maps are read between texels (a blur's paired taps, a blurred picture grown).
      const map = t !== 'backdrop' && t !== 'mask';
      p = { ...this.texture(t === 'mask' ? gl.R8 : gl.RGBA8, size[0], size[1], map), size };
      this.plain.set(t, p);
    }
    return p;
  }

  /** A drawn picture resolved into the texture it is read from. */
  private resolved(d: Drawn): WebGLTexture {
    if (d.dirty && d.rb) {
      const gl = this.gl;
      const [w, h] = this.size;
      gl.bindFramebuffer(gl.READ_FRAMEBUFFER, d.fbo);
      gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, d.read);
      gl.blitFramebuffer(0, 0, w, h, 0, 0, w, h, gl.COLOR_BUFFER_BIT, gl.NEAREST);
    }
    d.dirty = false;
    return d.tex;
  }

  private read(t: Tex): WebGLTexture {
    if (t === 'none') return this.none;
    if (t === 'layer') return this.resolved((this.layer ??= this.drawn()));
    if (t === 'group') return this.resolved((this.group ??= this.drawn()));
    return this.plainOf(t).tex;
  }

  /**
   * A full-target pass of `p` with the step's numbers and pictures; `flipV` turns the shadows' vertical offsets (device
   * pixels, y down) to GL's rows.
   */
  private full(p: GlProgram, s: ComposeStep & { reads: Reads; params: Float32Array }, flipV = false): void {
    const gl = this.gl;
    const textures = s.reads.map((t) => this.read(t));
    gl.useProgram(p.program);
    const a = s.params;
    gl.uniform4f(p.u.u_a, a[0], flipV ? -a[1] : a[1], a[2], flipV ? -a[3] : a[3]);
    NUMBERS.forEach((name, k) => {
      if (k > 0) gl.uniform4f(p.u[name], a[4 * k], a[4 * k + 1], a[4 * k + 2], a[4 * k + 3]);
    });
    textures.forEach((tex, i) => {
      gl.activeTexture(gl.TEXTURE0 + i);
      gl.bindTexture(gl.TEXTURE_2D, tex);
    });
    PICTURES.forEach((name, i) => gl.uniform1i(p.u[name], i));
    gl.bindVertexArray(null);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    // Nothing left bound that a later pass draws into.
    for (let i = textures.length - 1; i >= 0; i--) {
      gl.activeTexture(gl.TEXTURE0 + i);
      gl.bindTexture(gl.TEXTURE_2D, null);
    }
  }

  private premultiplied(): void {
    const gl = this.gl;
    gl.enable(gl.BLEND);
    gl.blendEquation(gl.FUNC_ADD);
    gl.blendFuncSeparate(gl.ONE, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  }

  /** Binds the layer's picture to draw into. */
  private intoLayer(): Drawn {
    const d = (this.layer ??= this.drawn());
    this.gl.bindFramebuffer(this.gl.FRAMEBUFFER, d.fbo);
    d.dirty = true;
    return d;
  }

  /**
   * Runs `plan`'s steps after the picture's first pass (its ground and underlays, drawn by the backend), for a
   * target of `size` device pixels and `samples` per pixel; leaves the backend's blending and viewport as it set them.
   */
  record(plan: ComposePlan, size: readonly [number, number], samples: number, host: GlComposeHost): void {
    const gl = this.gl;
    this.ensure(size, samples);
    const [w, h] = this.size;
    const steps = plan.steps;
    for (let i = 0; i < steps.length; i++) {
      const s = steps[i];
      switch (s.op) {
        case 'batches': {
          if (s.into === 'picture') host.picture();
          else this.intoLayer();
          host.draw(s.layer, s.from, s.to);
          while (i + 1 < steps.length) {
            const next = steps[i + 1];
            if (next.op !== 'batches' || next.into !== s.into) break;
            host.draw(next.layer, next.from, next.to);
            i++;
          }
          break;
        }
        case 'layerClear':
          this.intoLayer();
          gl.clearColor(0, 0, 0, 0);
          gl.clear(gl.COLOR_BUFFER_BIT);
          break;
        case 'group': {
          const d = (this.group ??= this.drawn());
          gl.bindFramebuffer(gl.FRAMEBUFFER, d.fbo);
          gl.clearColor(0, 0, 0, 0);
          gl.clear(gl.COLOR_BUFFER_BIT);
          host.draw(s.layer, s.from, s.to);
          d.dirty = true;
          break;
        }
        case 'map': {
          // The pictures it reads first: resolving binds framebuffers of its own.
          s.reads.forEach((t) => this.read(t));
          const target = this.plainOf(s.target);
          gl.bindFramebuffer(gl.FRAMEBUFFER, target.fbo);
          gl.viewport(0, 0, target.size[0], target.size[1]);
          gl.disable(gl.BLEND);
          this.full(this.maps[s.pipe], s);
          gl.viewport(0, 0, w, h);
          break;
        }
        case 'effect': {
          // The pictures it reads first: resolving binds framebuffers of its own.
          s.reads.forEach((t) => this.read(t));
          if (s.into === 'picture') host.picture();
          else this.intoLayer();
          this.premultiplied();
          this.full(this.effect, s, true);
          break;
        }
        case 'boxes': {
          const mask = this.plainOf('mask');
          gl.bindFramebuffer(gl.FRAMEBUFFER, mask.fbo);
          gl.clearColor(0, 0, 0, 0);
          gl.clear(gl.COLOR_BUFFER_BIT);
          gl.enable(gl.BLEND);
          gl.blendEquation(gl.MAX);
          gl.blendFunc(gl.ONE, gl.ONE);
          gl.useProgram(this.box.program);
          gl.uniform4f(this.box.u.u_a, s.params[0], s.params[1], 0, 0);
          gl.bindVertexArray(this.boxVao);
          gl.bindBuffer(gl.ARRAY_BUFFER, this.boxBuffer);
          gl.bufferData(gl.ARRAY_BUFFER, s.boxes, gl.DYNAMIC_DRAW);
          gl.drawArraysInstanced(gl.TRIANGLES, 0, 6, s.boxes.length / 7);
          gl.bindVertexArray(null);
          break;
        }
        case 'backdrop':
          host.copyPicture(this.plainOf('backdrop').fbo);
          break;
        case 'layer': {
          s.reads.forEach((t) => this.read(t));
          host.picture();
          if (s.blend) gl.disable(gl.BLEND);
          else this.premultiplied();
          this.full(s.blend ? this.blend : this.over, s);
          break;
        }
      }
    }
    // The backend's own blending (straight colour, "over" alpha) and viewport.
    gl.enable(gl.BLEND);
    gl.blendEquation(gl.FUNC_ADD);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.viewport(0, 0, w, h);
  }

  private release(): void {
    const gl = this.gl;
    for (const d of [this.layer, this.group]) {
      if (!d) continue;
      if (d.fbo !== d.read) gl.deleteFramebuffer(d.fbo);
      gl.deleteFramebuffer(d.read);
      if (d.rb) gl.deleteRenderbuffer(d.rb);
      gl.deleteTexture(d.tex);
    }
    for (const p of this.plain.values()) {
      gl.deleteFramebuffer(p.fbo);
      gl.deleteTexture(p.tex);
    }
    this.layer = this.group = null;
    this.plain.clear();
  }

  dispose(): void {
    const gl = this.gl;
    this.release();
    for (const p of [this.over, this.blend, this.effect, this.box, ...Object.values(this.maps)]) gl.deleteProgram(p.program);
    gl.deleteVertexArray(this.boxVao);
    gl.deleteBuffer(this.boxBuffer);
    gl.deleteTexture(this.none);
  }
}
