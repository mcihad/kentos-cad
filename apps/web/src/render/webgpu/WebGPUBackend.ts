import { supportedSamples, type AtlasSource, type FrameState, type RenderBackend, type RGBA, type SceneLayer } from '../types';
import { WGSL } from './shaders';
import { WebGPUStyledRenderer, type GpuStyledLayer } from './styledRenderer';

/** The pipelines of one sample count: a multisampled pass needs pipelines made for its count. */
interface Pipelines {
  line: GPURenderPipeline;
  fill: GPURenderPipeline;
  point: GPURenderPipeline;
  copy: GPURenderPipeline;
}

/** Counts beyond 1 a texture may be asked to have; the device says which it takes. */
const CANDIDATE_SAMPLES = [2, 4, 8, 16];

/** One uploaded batch: its vertex buffer(s), vertex/instance count and style. */
interface GpuBatch {
  buffers: GPUBuffer[];
  count: number;
  style: GPUBuffer;
  bind: GPUBindGroup;
}

interface GpuLayer {
  lines: GpuBatch[];
  fills: GpuBatch[];
  points: GpuBatch[];
  styled: GpuStyledLayer;
}

const SHAPES = { ring: 0, cross: 1, triangle: 2 } as const;
/** Frame uniform: offset, scale, pxPerUnit, dpr, viewport (8 floats). */
const FRAME_BYTES = 32;
/** Style uniform: color, dash, size, shape, pad (12 × 4 bytes). */
const STYLE_BYTES = 48;
/**
 * WebGPU flag constants (spec values). lib.dom declares only the flag
 * types, not the GPUBufferUsage / GPUShaderStage / GPUTextureUsage globals.
 */
const BUFFER = { VERTEX: 0x20, UNIFORM: 0x40, COPY_DST: 0x08 } as const;
const STAGE = { VERTEX: 0x1, FRAGMENT: 0x2 } as const;
const RENDER_ATTACHMENT = 0x10;
const TEXTURE_BINDING = 0x04;

/** A full-view triangle that copies the kept base into the frame's target (as in WebGL2Backend). */
const COPY_WGSL = `
@group(0) @binding(0) var base: texture_2d<f32>;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  return textureLoad(base, vec2i(pos.xy), 0);
}`;

/**
 * WebGPU implementation of the RenderBackend contract. It mirrors
 * WebGL2Backend step by step — same scene data, same draw order (per pass:
 * fills, lines, points), same shaders in WGSL — so switching backends does
 * not change a pixel's meaning:
 *   lines  → line-list, vertex {pos, dist}, dash tested per fragment
 *   fills  → triangle-list
 *   points → instanced quads with the symbol drawn by distance functions
 * Anti-aliasing is MSAA at the sample count asked for (setSamples, one of the
 * counts this device takes for the canvas format; the spec allows 4), with
 * the pipelines made per count and kept. As in WebGL2Backend, the underlays
 * and persistent layers are kept in a texture (`base`) and a frame whose only
 * change is in the overlays (FrameState.keepBase) copies it instead of
 * drawing them again.
 */
export class WebGPUBackend implements RenderBackend {
  readonly kind = 'webgpu' as const;
  label = 'WebGPU';
  private device!: GPUDevice;
  private context!: GPUCanvasContext;
  private canvas!: HTMLCanvasElement;
  private format!: GPUTextureFormat;
  private dpr = 1;
  private frameBuffer!: GPUBuffer;
  private frameBind!: GPUBindGroup;
  private styleLayout!: GPUBindGroupLayout;
  private pipelineLayout!: GPUPipelineLayout;
  private module!: GPUShaderModule;
  private copyModule!: GPUShaderModule;
  private copyLayout!: GPUBindGroupLayout;
  /** Pipelines by sample count, made when a count is first drawn with. */
  private pipelines = new Map<number, Pipelines>();
  private styled!: WebGPUStyledRenderer;
  private msaa: GPUTexture | null = null;
  /** Samples per pixel (1: the pass draws straight into its target), and the last count that drew. */
  samples = 4;
  private working = 1;
  sampleCounts: readonly number[] = [1];
  onSamplesFailed: ((requested: number, working: number, error: string) => void) | null = null;
  private layers = new Map<string, GpuLayer>();
  private base: GPUTexture | null = null;
  private copyBind: GPUBindGroup | null = null;
  /** What the base holds (view, scale, order, background), or '' when it must be drawn again. */
  private baseKey = '';
  /** The overlays of the last frame: uploading them leaves the base as it is. */
  private overlayIds: ReadonlySet<string> = new Set();

  static isSupported(): boolean {
    return typeof navigator !== 'undefined' && 'gpu' in navigator;
  }

  async init(canvas: HTMLCanvasElement, opts: { samples?: number } = {}): Promise<void> {
    if (!WebGPUBackend.isSupported()) throw new Error('Tarayıcı WebGPU sunmuyor');
    const adapter = await navigator.gpu.requestAdapter({ powerPreference: 'high-performance' });
    if (!adapter) throw new Error('WebGPU bağdaştırıcısı bulunamadı');
    const device = await adapter.requestDevice();
    const context = canvas.getContext('webgpu') as GPUCanvasContext | null;
    if (!context) throw new Error('WebGPU tuval bağlamı alınamadı');
    this.device = device;
    this.context = context;
    this.canvas = canvas;
    this.format = navigator.gpu.getPreferredCanvasFormat();
    context.configure({ device, format: this.format, alphaMode: 'opaque' });
    device.addEventListener('uncapturederror', (e) => console.error(`WebGPU: ${(e as GPUUncapturedErrorEvent).error.message}`));
    void device.lost.then((info) => {
      if (info.reason !== 'destroyed') console.error(`WebGPU aygıtı kayboldu: ${info.message}`);
    });
    const info = adapter.info;
    const name = info?.description || [info?.vendor, info?.architecture].filter(Boolean).join(' ');
    this.label = name ? `WebGPU · ${name}` : 'WebGPU';
    this.sampleCounts = await probeSampleCounts(device, this.format);
    this.samples = this.working = supportedSamples(this.sampleCounts, opts.samples ?? 4);
    this.createShared();
    this.pipelinesFor(this.samples);
  }

  /** What every sample count shares: the shader modules, the bind group layouts, the frame uniform. */
  private createShared(): void {
    const device = this.device;
    this.module = device.createShaderModule({ code: WGSL });
    const uniformEntry = (binding: number): GPUBindGroupLayoutEntry => ({ binding, visibility: STAGE.VERTEX | STAGE.FRAGMENT, buffer: { type: 'uniform' } });
    const frameLayout = device.createBindGroupLayout({ entries: [uniformEntry(0)] });
    this.styleLayout = device.createBindGroupLayout({ entries: [uniformEntry(0)] });
    this.pipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [frameLayout, this.styleLayout] });
    this.frameBuffer = device.createBuffer({ size: FRAME_BYTES, usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
    this.frameBind = device.createBindGroup({ layout: frameLayout, entries: [{ binding: 0, resource: { buffer: this.frameBuffer } }] });
    this.styled = new WebGPUStyledRenderer(device, this.format, frameLayout);
    this.copyModule = device.createShaderModule({ code: COPY_WGSL });
    // An explicit layout: one bind group of the kept base serves the copy pipeline of every count.
    this.copyLayout = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: STAGE.FRAGMENT, texture: { sampleType: 'float' } }] });
  }

  /** The pipelines for `count` samples (TODOS.md AA-01: pipelines are keyed by sample count). */
  private pipelinesFor(count: number): Pipelines {
    const kept = this.pipelines.get(count);
    if (kept) return kept;
    const device = this.device;
    const module = this.module;
    // Same blending as WebGL2: straight alpha for colour, "over" for alpha.
    const target: GPUColorTargetState = {
      format: this.format,
      blend: {
        color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
        alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      },
    };
    const pipeline = (vs: string, fs: string, buffers: GPUVertexBufferLayout[], topology: GPUPrimitiveTopology) =>
      device.createRenderPipeline({
        layout: this.pipelineLayout,
        vertex: { module, entryPoint: vs, buffers },
        fragment: { module, entryPoint: fs, targets: [target] },
        primitive: { topology },
        multisample: { count },
      });
    const vec2 = (location: number, stepMode: GPUVertexStepMode = 'vertex'): GPUVertexBufferLayout => ({ arrayStride: 8, stepMode, attributes: [{ shaderLocation: location, offset: 0, format: 'float32x2' }] });
    const made: Pipelines = {
      line: pipeline('lineVs', 'lineFs', [vec2(0), { arrayStride: 4, attributes: [{ shaderLocation: 1, offset: 0, format: 'float32' }] }], 'line-list'),
      fill: pipeline('fillVs', 'fillFs', [vec2(0)], 'triangle-list'),
      point: pipeline('pointVs', 'pointFs', [vec2(0, 'instance')], 'triangle-list'),
      copy: device.createRenderPipeline({
        layout: device.createPipelineLayout({ bindGroupLayouts: [this.copyLayout] }),
        vertex: { module: this.copyModule, entryPoint: 'vs' },
        fragment: { module: this.copyModule, entryPoint: 'fs', targets: [{ format: this.format }] },
        primitive: { topology: 'triangle-list' },
        multisample: { count },
      }),
    };
    this.styled.pipelinesFor(count);
    this.pipelines.set(count, made);
    return made;
  }

  setSamples(count: number): number {
    const n = supportedSamples(this.sampleCounts, count);
    if (n === this.samples) return n;
    const device = this.device;
    // The new count's pipelines and target are made under error scopes: if the device refuses them
    // (out of memory), the last working count takes over again.
    device.pushErrorScope('out-of-memory');
    device.pushErrorScope('validation');
    this.samples = n;
    this.pipelinesFor(n);
    this.ensureMsaa(this.canvas.width, this.canvas.height);
    const settled = Promise.all([device.popErrorScope(), device.popErrorScope()]);
    void settled.then(([validation, memory]) => {
      const error = validation ?? memory;
      if (!error) {
        if (this.samples === n) this.working = n;
        return;
      }
      this.pipelines.delete(n);
      this.styled.forget(n);
      if (this.samples === n) {
        this.samples = n === this.working ? 1 : this.working;
        this.baseKey = '';
      }
      this.onSamplesFailed?.(n, this.samples, error.message);
    });
    this.baseKey = '';
    return n;
  }

  /**
   * The multisampled colour target at this size and count. The one it
   * replaces is destroyed only once the GPU has finished the work already
   * submitted with it (TODOS.md AA-02).
   */
  private ensureMsaa(w: number, h: number): void {
    const want = this.samples > 1;
    const m = this.msaa;
    if (m && want && m.width === w && m.height === h && m.sampleCount === this.samples) return;
    if (m) {
      this.msaa = null;
      void this.device.queue.onSubmittedWorkDone().then(() => m.destroy());
    }
    if (want) this.msaa = this.device.createTexture({ size: [w, h], sampleCount: this.samples, format: this.format, usage: RENDER_ATTACHMENT });
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

  private vertexBuffer(data: Float32Array): GPUBuffer {
    const buf = this.device.createBuffer({ size: Math.max(4, data.byteLength), usage: BUFFER.VERTEX | BUFFER.COPY_DST });
    this.device.queue.writeBuffer(buf, 0, data.buffer, data.byteOffset, data.byteLength);
    return buf;
  }

  private styleBind(color: RGBA, dash: readonly number[] | null, size = 0, shape = 0): { style: GPUBuffer; bind: GPUBindGroup } {
    const data = new ArrayBuffer(STYLE_BYTES);
    const f = new Float32Array(data);
    f.set(color, 0);
    f.set([dash?.[0] ?? 0, dash?.[1] ?? 0, dash?.[2] ?? 0, dash?.[3] ?? 0], 4);
    f[8] = size;
    new Uint32Array(data)[9] = shape;
    const style = this.device.createBuffer({ size: STYLE_BYTES, usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
    this.device.queue.writeBuffer(style, 0, data);
    const bind = this.device.createBindGroup({ layout: this.styleLayout, entries: [{ binding: 0, resource: { buffer: style } }] });
    return { style, bind };
  }

  upload(layer: SceneLayer): void {
    this.remove(layer.id);
    if (!this.overlayIds.has(layer.id)) this.baseKey = '';
    const g: GpuLayer = { lines: [], fills: [], points: [], styled: this.styled.upload(layer.styled ?? []) };
    for (const b of layer.lines) {
      if (!b.positions.length) continue;
      g.lines.push({ buffers: [this.vertexBuffer(b.positions), this.vertexBuffer(b.distances)], count: b.positions.length / 2, ...this.styleBind(b.color, b.dash) });
    }
    for (const f of layer.fills) {
      if (!f.positions.length) continue;
      g.fills.push({ buffers: [this.vertexBuffer(f.positions)], count: f.positions.length / 2, ...this.styleBind(f.color, null) });
    }
    for (const p of layer.points) {
      if (!p.positions.length) continue;
      g.points.push({ buffers: [this.vertexBuffer(p.positions)], count: p.positions.length / 2, ...this.styleBind(p.color, null, p.size, SHAPES[p.shape]) });
    }
    this.layers.set(layer.id, g);
  }

  remove(id: string): void {
    const g = this.layers.get(id);
    if (!g) return;
    if (!this.overlayIds.has(id)) this.baseKey = '';
    for (const b of [...g.lines, ...g.fills, ...g.points]) {
      b.buffers.forEach((buf) => buf.destroy());
      b.style.destroy();
    }
    this.styled.release(g.styled);
    this.layers.delete(id);
  }

  render(frame: FrameState): void {
    const { device } = this;
    const { view } = frame;
    const w = this.canvas.width;
    const h = this.canvas.height;
    const pipes = this.pipelinesFor(this.samples);
    const multisampled = this.samples > 1;
    this.ensureMsaa(w, h);
    if (!this.base || this.base.width !== w || this.base.height !== h) {
      const old = this.base;
      if (old) void device.queue.onSubmittedWorkDone().then(() => old.destroy());
      this.base = device.createTexture({ size: [w, h], format: this.format, usage: RENDER_ATTACHMENT | TEXTURE_BINDING });
      this.copyBind = device.createBindGroup({ layout: this.copyLayout, entries: [{ binding: 0, resource: this.base.createView() }] });
      this.baseKey = '';
    }
    this.overlayIds = new Set(frame.overlays);
    const key = `${view.center.x},${view.center.y},${view.scale},${w}x${h},${frame.scaleDenominator},${frame.clearColor.join()},${frame.underlays.join()}|${frame.order.join()}`;
    const drawBase = !frame.keepBase || key !== this.baseKey;
    device.queue.writeBuffer(
      this.frameBuffer,
      0,
      new Float32Array([view.center.x, view.center.y, (2 * view.scale) / view.width, (2 * view.scale) / view.height, view.scale * this.dpr, this.dpr, w, h]),
    );
    const [r, g, b] = frame.clearColor;
    const encoder = device.createCommandEncoder();
    // The base pass resolves into the kept texture; the frame pass copies it and draws the overlays into the canvas.
    const begin = (target: GPUTexture) =>
      encoder.beginRenderPass({
        colorAttachments: [
          multisampled
            ? { view: this.msaa!.createView(), resolveTarget: target.createView(), clearValue: { r, g, b, a: 1 }, loadOp: 'clear', storeOp: 'discard' }
            : { view: target.createView(), clearValue: { r, g, b, a: 1 }, loadOp: 'clear', storeOp: 'store' },
        ],
      });
    // Visibility and atlas images for the whole frame before anything is drawn.
    this.styled.prepare(
      [...(drawBase ? [...frame.underlays, ...frame.order] : []), ...frame.overlays].flatMap((id) => this.layers.get(id)?.styled ?? []),
      { cam: [view.center.x, view.center.y], pxPerM: view.scale * this.dpr, dpr: this.dpr, viewPx: [w, h], scaleDenominator: frame.scaleDenominator },
    );
    let pass!: GPURenderPassEncoder;
    const drawPass = (ids: readonly string[]) => {
      const layers = ids.map((id) => this.layers.get(id)).filter((l): l is GpuLayer => !!l);
      // Same order as WebGL2: per layer its plain fills then its styled symbols; then plain lines and points.
      for (const l of layers) {
        if (l.fills.length) {
          pass.setPipeline(pipes.fill);
          for (const f of l.fills) {
            pass.setBindGroup(1, f.bind);
            pass.setVertexBuffer(0, f.buffers[0]);
            pass.draw(f.count);
          }
        }
        this.styled.draw(pass, l.styled, this.samples);
      }
      pass.setPipeline(pipes.line);
      for (const l of layers)
        for (const ln of l.lines) {
          pass.setBindGroup(1, ln.bind);
          pass.setVertexBuffer(0, ln.buffers[0]);
          pass.setVertexBuffer(1, ln.buffers[1]);
          pass.draw(ln.count);
        }
      pass.setPipeline(pipes.point);
      for (const l of layers)
        for (const p of l.points) {
          pass.setBindGroup(1, p.bind);
          pass.setVertexBuffer(0, p.buffers[0]);
          pass.draw(6, p.count);
        }
    };
    if (drawBase) {
      pass = begin(this.base);
      pass.setBindGroup(0, this.frameBind);
      drawPass(frame.underlays);
      drawPass(frame.order);
      pass.end();
      this.baseKey = key;
    }
    pass = begin(this.context.getCurrentTexture());
    pass.setPipeline(pipes.copy);
    pass.setBindGroup(0, this.copyBind!);
    pass.draw(3);
    pass.setBindGroup(0, this.frameBind);
    drawPass(frame.overlays);
    pass.end();
    device.queue.submit([encoder.finish()]);
  }

  dispose(): void {
    for (const id of [...this.layers.keys()]) this.remove(id);
    this.msaa?.destroy();
    this.base?.destroy();
    this.frameBuffer?.destroy();
    this.styled?.dispose();
    this.device?.destroy();
  }
}

/**
 * The sample counts a render target of `format` takes on this device,
 * ascending, 1 first: each candidate is created once, 1 × 1, inside a
 * validation scope, and kept if the device accepts it (TODOS.md AA-01).
 * WebGPU allows 1 and 4; a browser that validates more is believed, one
 * that validates less too.
 */
export async function probeSampleCounts(device: GPUDevice, format: GPUTextureFormat): Promise<number[]> {
  const counts = [1];
  for (const n of CANDIDATE_SAMPLES) {
    device.pushErrorScope('validation');
    const texture = device.createTexture({ size: [1, 1], sampleCount: n, format, usage: RENDER_ATTACHMENT });
    const error = await device.popErrorScope();
    texture.destroy();
    if (!error) counts.push(n);
  }
  return counts;
}
