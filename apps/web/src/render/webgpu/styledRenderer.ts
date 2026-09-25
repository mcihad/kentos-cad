import { batchImage, batchImagePx, batchInView, batchLegible, MARKER_STRIDE, STROKE_STRIDE, type AtlasHit, type AtlasSource, type StyledBatch } from '../types';
import { dashValues, inScale, patternReach, type StyledFrame } from '../webgl2/styledRenderer';
import { SHAPE_IDS } from '../webgl2/styledShaders';
import { STYLED_WGSL } from './styledShaders';

/**
 * WebGPU side of the styled batches, twin of webgl2/styledRenderer.ts:
 * pipelines for strokes, solid/hatch/pattern/tile fills and markers; one
 * style uniform per batch written at upload (its atlas rectangle rewritten
 * when the image moves); the atlas as a one-level texture fed image by
 * image. A frame prepares (visibility, atlas lookups, rectangle writes)
 * before the render pass is encoded, since the whole pass sees the texture
 * as it is at submit.
 */

const BUFFER = { VERTEX: 0x20, UNIFORM: 0x40, COPY_DST: 0x08 } as const;
const TEXTURE = { COPY_DST: 0x02, TEXTURE_BINDING: 0x04, RENDER_ATTACHMENT: 0x10 } as const;
const STAGE = { VERTEX: 0x1, FRAGMENT: 0x2 } as const;
/** SStyle: 8 vec4f + 1 vec4u. */
const STYLE_BYTES = 144;
const SHAPE_INDEX = new Map<string, number>(SHAPE_IDS.map((s, i) => [s, i]));

interface GpuStyled {
  batch: StyledBatch;
  vertex: GPUBuffer;
  count: number;
  style: GPUBuffer;
  bind: GPUBindGroup;
  data: ArrayBuffer;
  /** Set by prepare: drawn this frame; the atlas rectangle last written into the style. */
  visible: boolean;
  hit: AtlasHit | null;
}

export interface GpuStyledLayer {
  list: GpuStyled[];
}

type StyledPipes = Record<'stroke' | 'solid' | 'hatch' | 'pattern' | 'tile' | 'marker', GPURenderPipeline>;

export class WebGPUStyledRenderer {
  private readonly device: GPUDevice;
  private readonly format: GPUTextureFormat;
  private readonly module: GPUShaderModule;
  private readonly layout: GPUPipelineLayout;
  private readonly styleLayout: GPUBindGroupLayout;
  private readonly atlasBind: GPUBindGroup;
  private readonly texture: GPUTexture;
  /** Pipelines by sample count (the backend's multisampled passes need their own). */
  private readonly pipes = new Map<number, StyledPipes>();
  private atlas: AtlasSource | null = null;

  constructor(device: GPUDevice, format: GPUTextureFormat, frameLayout: GPUBindGroupLayout) {
    this.device = device;
    this.format = format;
    this.module = device.createShaderModule({ code: STYLED_WGSL });
    this.styleLayout = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: STAGE.VERTEX | STAGE.FRAGMENT, buffer: { type: 'uniform' } }] });
    const atlasLayout = device.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: STAGE.FRAGMENT, texture: { sampleType: 'float' } },
        { binding: 1, visibility: STAGE.FRAGMENT, sampler: { type: 'filtering' } },
      ],
    });
    this.texture = device.createTexture({ size: [2048, 2048], format: 'rgba8unorm', usage: TEXTURE.COPY_DST | TEXTURE.TEXTURE_BINDING | TEXTURE.RENDER_ATTACHMENT });
    const sampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear', addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' });
    this.atlasBind = device.createBindGroup({ layout: atlasLayout, entries: [{ binding: 0, resource: this.texture.createView() }, { binding: 1, resource: sampler }] });
    this.layout = device.createPipelineLayout({ bindGroupLayouts: [frameLayout, this.styleLayout, atlasLayout] });
  }

  /** The pipelines for `samples` per pixel, made the first time that count draws. */
  pipelinesFor(samples: number): StyledPipes {
    const kept = this.pipes.get(samples);
    if (kept) return kept;
    const { device, module, layout, format } = this;
    const straight: GPUBlendState = { color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' }, alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' } };
    const premul: GPUBlendState = { color: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' }, alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' } };
    const pipe = (vs: string, fs: string, buffers: GPUVertexBufferLayout[], blend: GPUBlendState) =>
      device.createRenderPipeline({
        layout,
        vertex: { module, entryPoint: vs, buffers },
        fragment: { module, entryPoint: fs, targets: [{ format, blend }] },
        primitive: { topology: 'triangle-list' },
        multisample: { count: samples },
      });
    const area: GPUVertexBufferLayout = { arrayStride: 8, attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x2' }] };
    const made: StyledPipes = {
      stroke: pipe(
        'strokeVs',
        'strokeFs',
        [
          {
            arrayStride: STROKE_STRIDE * 4,
            stepMode: 'instance',
            attributes: [
              { shaderLocation: 0, offset: 0, format: 'float32x4' },
              { shaderLocation: 1, offset: 16, format: 'float32x2' },
            ],
          },
        ],
        straight,
      ),
      solid: pipe('areaVs', 'solidFs', [area], straight),
      hatch: pipe('areaVs', 'hatchFs', [area], straight),
      pattern: pipe('areaVs', 'patternFs', [area], premul),
      tile: pipe('areaVs', 'tileFs', [area], premul),
      marker: pipe(
        'markerVs',
        'markerFs',
        [
          {
            arrayStride: MARKER_STRIDE * 4,
            stepMode: 'instance',
            attributes: [
              { shaderLocation: 0, offset: 0, format: 'float32x4' },
              { shaderLocation: 1, offset: 16, format: 'float32' },
            ],
          },
        ],
        premul,
      ),
    };
    this.pipes.set(samples, made);
    return made;
  }

  /** Drops the pipelines of a count the device refused (they are made again if it is asked for again). */
  forget(samples: number): void {
    this.pipes.delete(samples);
  }

  useAtlas(atlas: AtlasSource): void {
    this.atlas = atlas;
    atlas.attach((source, x, y) => {
      this.device.queue.copyExternalImageToTexture({ source }, { texture: this.texture, origin: { x, y }, premultipliedAlpha: true }, [source.width, source.height]);
    });
  }

  // ── Upload ───────────────────────────────────────────────────────────

  upload(batches: readonly StyledBatch[]): GpuStyledLayer {
    const list: GpuStyled[] = [];
    for (const b of batches) {
      const src = b.kind === 'stroke' ? b.segments : b.kind === 'marker' ? b.instances : b.positions;
      const vertex = this.device.createBuffer({ size: Math.max(4, src.byteLength), usage: BUFFER.VERTEX | BUFFER.COPY_DST });
      this.device.queue.writeBuffer(vertex, 0, src.buffer, src.byteOffset, src.byteLength);
      const count = b.kind === 'stroke' ? src.length / STROKE_STRIDE : b.kind === 'marker' ? src.length / MARKER_STRIDE : src.length / 2;
      const data = this.styleData(b);
      const style = this.device.createBuffer({ size: STYLE_BYTES, usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
      this.device.queue.writeBuffer(style, 0, data);
      const bind = this.device.createBindGroup({ layout: this.styleLayout, entries: [{ binding: 0, resource: { buffer: style } }] });
      list.push({ batch: b, vertex, count, style, bind, data, visible: false, hit: null });
    }
    return { list };
  }

  release(layer: GpuStyledLayer): void {
    for (const s of layer.list) {
      s.vertex.destroy();
      s.style.destroy();
    }
  }

  /** The style uniform of a batch; atlas rectangles are filled in at draw time. */
  private styleData(b: StyledBatch): ArrayBuffer {
    const data = new ArrayBuffer(STYLE_BYTES);
    const f = new Float32Array(data);
    const u = new Uint32Array(data);
    const dash = (d: readonly number[] | null) => {
      const v = dashValues(d);
      f.set(v.d, 8);
      return v;
    };
    if (b.kind === 'stroke') {
      f.set(b.color, 0);
      const v = dash(b.dash);
      f.set([b.width, v.total, v.on, b.dashOffset], 20);
      f.set([b.blur, 0, 0, 0], 24);
      u.set([b.unit === 'world' ? 0 : 1, b.cap === 'round' ? 1 : b.cap === 'square' ? 2 : 0, 0, 0], 32);
    } else if (b.kind === 'fill') {
      const p = b.paint;
      if (p.kind === 'solid') f.set(p.color, 0);
      else if (p.kind === 'hatch') {
        f.set(p.color, 0);
        const v = dash(p.dash);
        f.set([Math.cos(p.angle), Math.sin(p.angle), p.spacing, p.width], 20);
        f.set([p.offset, v.total, v.on, p.dashOffset], 24);
        u[32] = p.unit === 'world' ? 0 : 1;
      } else if (p.kind === 'pattern') {
        const r = patternReach(p);
        f.set(p.fill ?? [0, 0, 0, 0], 0);
        f.set(p.stroke ?? [0, 0, 0, 0], 4);
        f.set([p.size[0], p.size[1], Math.cos(p.angle), Math.sin(p.angle)], 8);
        f.set([p.offset[0], p.offset[1], r.jitter[0], r.jitter[1]], 12);
        f.set([p.half[0], p.half[1], p.markOffset[0], p.markOffset[1]], 16);
        f.set([Math.cos(p.markRotation), Math.sin(p.markRotation), p.coverage, p.seed], 20);
        f.set([p.strokeWidth, p.tint, p.opacity, 0], 24);
        f.set(p.params, 28);
        u.set([p.unit === 'world' ? 0 : 1, p.stagger ? 1 : 0, SHAPE_INDEX.get(p.shape) ?? 0, r.reach], 32);
      } else {
        f.set([p.size[0], p.size[1], Math.cos(p.angle), Math.sin(p.angle)], 20);
        f.set([p.offset[0], p.offset[1], p.opacity, 0], 24);
        u[32] = p.unit === 'world' ? 0 : 1;
      }
    } else {
      const look = b.look;
      f.set([b.offset[0], b.offset[1], b.anchor[0], b.anchor[1]], 20);
      if (look.kind === 'shape') {
        f.set(look.fill ?? [0, 0, 0, 0], 0);
        f.set(look.stroke ?? [0, 0, 0, 0], 4);
        f.set([1, look.strokeWidth, b.opacity, 0], 24);
        f.set(look.params, 28);
        u.set([b.unit === 'world' ? 0 : 1, 0, SHAPE_INDEX.get(look.shape) ?? 0, 0], 32);
      } else {
        f.set([1, 0, b.opacity, 0], 24);
        u.set([b.unit === 'world' ? 0 : 1, 1, 0, look.fit === 'width' ? 1 : 2], 32);
      }
    }
    return data;
  }

  // ── Drawing ──────────────────────────────────────────────────────────

  /**
   * Decides what shows this frame and places its images in the atlas,
   * writing moved rectangles into the batch styles. If the atlas starts
   * over midway, everything is looked up once more.
   */
  prepare(layers: readonly GpuStyledLayer[], f: StyledFrame): void {
    const hw = f.viewPx[0] / 2 / f.pxPerM;
    const hh = f.viewPx[1] / 2 / f.pxPerM;
    const view = [f.cam[0] - hw, f.cam[1] - hh, f.cam[0] + hw, f.cam[1] + hh] as const;
    const atlas = this.atlas;
    atlas?.beginFrame();
    for (let pass = 0; pass < 2; pass++) {
      const generation = atlas?.generation ?? 0;
      for (const layer of layers)
        for (const s of layer.list) {
          const b = s.batch;
          s.visible = inScale(b, f.scaleDenominator) && batchInView(b, view, f.pxPerM, f.dpr) && batchLegible(b, f.pxPerM, f.dpr);
          if (!s.visible) continue;
          const image = batchImage(b);
          if (!image) continue;
          const hit = atlas?.lookup(image, batchImagePx(b, f.pxPerM, f.dpr)) ?? null;
          if (!hit) {
            s.visible = false;
            continue;
          }
          if (hit === s.hit) continue;
          s.hit = hit;
          const data = new Float32Array(s.data);
          data.set(hit.uv, 16);
          if (b.kind === 'marker') data[24] = hit.aspect;
          this.device.queue.writeBuffer(s.style, 0, s.data);
        }
      if ((atlas?.generation ?? 0) === generation) break;
    }
  }

  /** Draws a layer's visible batches into a pass of `samples` per pixel. */
  draw(pass: GPURenderPassEncoder, layer: GpuStyledLayer, samples: number): void {
    if (!layer.list.length) return;
    const pipes = this.pipelinesFor(samples);
    pass.setBindGroup(2, this.atlasBind);
    let current: GPURenderPipeline | null = null;
    for (const s of layer.list) {
      if (!s.visible) continue;
      const b = s.batch;
      const pipe = b.kind === 'stroke' ? pipes.stroke : b.kind === 'marker' ? pipes.marker : pipes[b.paint.kind];
      if (pipe !== current) {
        pass.setPipeline(pipe);
        current = pipe;
      }
      pass.setBindGroup(1, s.bind);
      pass.setVertexBuffer(0, s.vertex);
      if (b.kind === 'fill') pass.draw(s.count);
      else pass.draw(6, s.count);
    }
  }

  dispose(): void {
    this.texture.destroy();
  }
}
