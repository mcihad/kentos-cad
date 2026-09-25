import type { AtlasSource, FrameState, RenderBackend, RGBA, SceneLayer } from '../types';
import { WGSL } from './shaders';
import { WebGPUStyledRenderer, type GpuStyledLayer } from './styledRenderer';

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
/** Multisampling when anti-aliasing is on (Çizim kalitesi: Yüksek). */
const SAMPLES = 4;
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

/**
 * WebGPU implementation of the RenderBackend contract. It mirrors
 * WebGL2Backend step by step — same scene data, same draw order (per pass:
 * fills, lines, points), same shaders in WGSL — so switching backends does
 * not change a pixel's meaning:
 *   lines  → line-list, vertex {pos, dist}, dash tested per fragment
 *   fills  → triangle-list
 *   points → instanced quads with the symbol drawn by distance functions
 * Anti-aliasing is 4× MSAA like the WebGL2 context (none at the lower drawing qualities).
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
  private linePipe!: GPURenderPipeline;
  private fillPipe!: GPURenderPipeline;
  private pointPipe!: GPURenderPipeline;
  private styled!: WebGPUStyledRenderer;
  private msaa: GPUTexture | null = null;
  /** 4 with anti-aliasing, 1 without (then the pass draws straight into the canvas). */
  private samples = SAMPLES;
  private layers = new Map<string, GpuLayer>();

  static isSupported(): boolean {
    return typeof navigator !== 'undefined' && 'gpu' in navigator;
  }

  async init(canvas: HTMLCanvasElement, opts: { antialias?: boolean } = {}): Promise<void> {
    this.samples = opts.antialias === false ? 1 : SAMPLES;
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
    this.createPipelines();
  }

  private createPipelines(): void {
    const device = this.device;
    const module = device.createShaderModule({ code: WGSL });
    const uniformEntry = (binding: number): GPUBindGroupLayoutEntry => ({ binding, visibility: STAGE.VERTEX | STAGE.FRAGMENT, buffer: { type: 'uniform' } });
    const frameLayout = device.createBindGroupLayout({ entries: [uniformEntry(0)] });
    this.styleLayout = device.createBindGroupLayout({ entries: [uniformEntry(0)] });
    const layout = device.createPipelineLayout({ bindGroupLayouts: [frameLayout, this.styleLayout] });
    this.frameBuffer = device.createBuffer({ size: FRAME_BYTES, usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
    this.frameBind = device.createBindGroup({ layout: frameLayout, entries: [{ binding: 0, resource: { buffer: this.frameBuffer } }] });

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
        layout,
        vertex: { module, entryPoint: vs, buffers },
        fragment: { module, entryPoint: fs, targets: [target] },
        primitive: { topology },
        multisample: { count: this.samples },
      });
    const vec2 = (location: number, stepMode: GPUVertexStepMode = 'vertex'): GPUVertexBufferLayout => ({ arrayStride: 8, stepMode, attributes: [{ shaderLocation: location, offset: 0, format: 'float32x2' }] });
    this.linePipe = pipeline('lineVs', 'lineFs', [vec2(0), { arrayStride: 4, attributes: [{ shaderLocation: 1, offset: 0, format: 'float32' }] }], 'line-list');
    this.fillPipe = pipeline('fillVs', 'fillFs', [vec2(0)], 'triangle-list');
    this.pointPipe = pipeline('pointVs', 'pointFs', [vec2(0, 'instance')], 'triangle-list');
    this.styled = new WebGPUStyledRenderer(device, this.format, frameLayout, this.samples);
  }

  useAtlas(atlas: AtlasSource): void {
    this.styled.useAtlas(atlas);
  }

  resize(width: number, height: number, dpr: number): void {
    this.dpr = dpr;
    this.canvas.width = Math.max(1, Math.round(width * dpr));
    this.canvas.height = Math.max(1, Math.round(height * dpr));
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
    const multisampled = this.samples > 1;
    if (multisampled && (!this.msaa || this.msaa.width !== w || this.msaa.height !== h)) {
      this.msaa?.destroy();
      this.msaa = device.createTexture({ size: [w, h], sampleCount: this.samples, format: this.format, usage: RENDER_ATTACHMENT });
    }
    device.queue.writeBuffer(
      this.frameBuffer,
      0,
      new Float32Array([view.center.x, view.center.y, (2 * view.scale) / view.width, (2 * view.scale) / view.height, view.scale * this.dpr, this.dpr, w, h]),
    );
    const [r, g, b] = frame.clearColor;
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        multisampled
          ? { view: this.msaa!.createView(), resolveTarget: this.context.getCurrentTexture().createView(), clearValue: { r, g, b, a: 1 }, loadOp: 'clear', storeOp: 'discard' }
          : { view: this.context.getCurrentTexture().createView(), clearValue: { r, g, b, a: 1 }, loadOp: 'clear', storeOp: 'store' },
      ],
    });
    pass.setBindGroup(0, this.frameBind);
    // Visibility and atlas images for the whole frame before anything is drawn.
    this.styled.prepare(
      [...frame.underlays, ...frame.order, ...frame.overlays].flatMap((id) => this.layers.get(id)?.styled ?? []),
      { cam: [view.center.x, view.center.y], pxPerM: view.scale * this.dpr, dpr: this.dpr, viewPx: [w, h], scaleDenominator: frame.scaleDenominator },
    );
    const drawPass = (ids: readonly string[]) => {
      const layers = ids.map((id) => this.layers.get(id)).filter((l): l is GpuLayer => !!l);
      // Same order as WebGL2: per layer its plain fills then its styled symbols; then plain lines and points.
      for (const l of layers) {
        if (l.fills.length) {
          pass.setPipeline(this.fillPipe);
          for (const f of l.fills) {
            pass.setBindGroup(1, f.bind);
            pass.setVertexBuffer(0, f.buffers[0]);
            pass.draw(f.count);
          }
        }
        this.styled.draw(pass, l.styled);
      }
      pass.setPipeline(this.linePipe);
      for (const l of layers)
        for (const ln of l.lines) {
          pass.setBindGroup(1, ln.bind);
          pass.setVertexBuffer(0, ln.buffers[0]);
          pass.setVertexBuffer(1, ln.buffers[1]);
          pass.draw(ln.count);
        }
      pass.setPipeline(this.pointPipe);
      for (const l of layers)
        for (const p of l.points) {
          pass.setBindGroup(1, p.bind);
          pass.setVertexBuffer(0, p.buffers[0]);
          pass.draw(6, p.count);
        }
    };
    drawPass(frame.underlays);
    drawPass(frame.order);
    drawPass(frame.overlays);
    pass.end();
    device.queue.submit([encoder.finish()]);
  }

  dispose(): void {
    for (const id of [...this.layers.keys()]) this.remove(id);
    this.msaa?.destroy();
    this.frameBuffer?.destroy();
    this.styled?.dispose();
    this.device?.destroy();
  }
}
