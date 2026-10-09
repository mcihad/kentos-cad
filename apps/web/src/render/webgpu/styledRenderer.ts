import { batchImage, batchImagePx, batchInView, batchLegible, MARKER_STRIDE, STROKE_STRIDE, type AtlasHit, type AtlasSource, type StyledBatch } from '../types';
import { fitted, GREY, levelCanvas, levelCount } from '../pictures';
import { RasterQuads, RasterSlots, SLOT, UPLOADS_PER_FRAME, rasterAtlasSize, rasterQuads } from '../rasterPass';
import { positionTile, ServiceMeshes, serviceQuads, serviceVectors, type ServicePaint, type ServiceSource } from '../servicePass';
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
/** A batch's style block (WGSL `SStyle`, contract version 3): eight vec4f, the flags, then its tile's origin. */
const STYLE_BYTES = 160;
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
  /** A raster's quads this frame (docs/adr/0204 §5): written into `vertex` by prepare, grown as needed. */
  quads?: RasterQuads;
  vertexBytes?: number;
  /** A vector service's tiles this frame (docs/adr/0208 §9), drawn where it is. */
  vectors?: GpuStyledLayer[];
}

export interface GpuStyledLayer {
  list: GpuStyled[];
}

type StyledPipes = Record<'stroke' | 'solid' | 'hatch' | 'pattern' | 'tile' | 'marker' | 'gradient' | 'image' | 'raster', GPURenderPipeline>;

/** The raster atlas page (docs/adr/0204 §5): its texture, slots and group 0 by frame uniform and sampling. */
interface RasterPage {
  texture: GPUTexture;
  slots: RasterSlots;
  frame: { linear: GPUBindGroup; nearest: GPUBindGroup };
  lens: { linear: GPUBindGroup; nearest: GPUBindGroup };
}

/** A picture's texture with its group 0 for the frame and for the magnifier (docs/adr/0192 §3; contract version 5). */
interface GpuPicture {
  texture: GPUTexture;
  frame: GPUBindGroup;
  lens: GPUBindGroup;
}

export class WebGPUStyledRenderer {
  private readonly device: GPUDevice;
  private readonly format: GPUTextureFormat;
  private readonly module: GPUShaderModule;
  private readonly layout: GPUPipelineLayout;
  private readonly styleLayout: GPUBindGroupLayout;
  /** Group 0 of the styled pipelines: the frame uniform with the atlas beside it (contract version 3). */
  private readonly frameBind: GPUBindGroup;
  /** The same with the magnifier's frame uniform (docs/adr/0181 §5). */
  private readonly lensBind: GPUBindGroup;
  private readonly texture: GPUTexture;
  private readonly frameLayout: GPUBindGroupLayout;
  private readonly frameBuffer: GPUBuffer;
  private readonly lensBuffer: GPUBuffer;
  /** Pictures sample their mip levels (docs/adr/0192 §3). */
  private readonly pictureSampler: GPUSampler;
  /** The pictures' own textures by key; the grey stand-in under `''`. */
  private readonly pictures = new Map<string, GpuPicture>();
  /** Pipelines by sample count (the backend's multisampled passes need their own). */
  private readonly pipes = new Map<number, StyledPipes>();
  private atlas: AtlasSource | null = null;
  private rasterPage: RasterPage | null = null;
  /** The map services' tiles (docs/adr/0208 §3): their meshes and their vector tiles' buffers by tile. */
  private services: ServiceSource | null = null;
  private readonly meshes = new ServiceMeshes();
  private readonly vectorLayers = new Map<string, { layer: GpuStyledLayer; at: number }>();
  private vectorFrame = 0;

  /** `frame`: the backend's frame uniform, which the styled pipelines read beside their atlas; `lens`: the magnifier's. */
  constructor(device: GPUDevice, format: GPUTextureFormat, frame: GPUBuffer, lens: GPUBuffer) {
    this.device = device;
    this.format = format;
    this.module = device.createShaderModule({ code: STYLED_WGSL });
    this.styleLayout = device.createBindGroupLayout({ entries: [{ binding: 0, visibility: STAGE.VERTEX | STAGE.FRAGMENT, buffer: { type: 'uniform' } }] });
    // shaders/wgsl/styled.layout.json v3: group 0 is the frame, the atlas texture and its sampler; group 1 the batch's style.
    const frameLayout = (this.frameLayout = device.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: STAGE.VERTEX | STAGE.FRAGMENT, buffer: { type: 'uniform' } },
        { binding: 1, visibility: STAGE.FRAGMENT, texture: { sampleType: 'float' } },
        { binding: 2, visibility: STAGE.FRAGMENT, sampler: { type: 'filtering' } },
      ],
    }));
    this.frameBuffer = frame;
    this.lensBuffer = lens;
    this.pictureSampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear', mipmapFilter: 'linear', addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' });
    this.texture = device.createTexture({ size: [2048, 2048], format: 'rgba8unorm', usage: TEXTURE.COPY_DST | TEXTURE.TEXTURE_BINDING | TEXTURE.RENDER_ATTACHMENT });
    const sampler = device.createSampler({ magFilter: 'linear', minFilter: 'linear', addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' });
    const bindFor = (buffer: GPUBuffer) =>
      device.createBindGroup({
        layout: frameLayout,
        entries: [
          { binding: 0, resource: { buffer } },
          { binding: 1, resource: this.texture.createView() },
          { binding: 2, resource: sampler },
        ],
      });
    this.frameBind = bindFor(frame);
    this.lensBind = bindFor(lens);
    this.layout = device.createPipelineLayout({ bindGroupLayouts: [frameLayout, this.styleLayout] });
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
      // docs/adr/0186 §3: the contract's version 4.
      gradient: pipe('areaVs', 'gradientFs', [area], straight),
      pattern: pipe('areaVs', 'patternFs', [area], premul),
      tile: pipe('areaVs', 'tileFs', [area], premul),
      // docs/adr/0192 §3: the contract's version 5.
      image: pipe('areaVs', 'imageFs', [area], premul),
      // docs/adr/0204 §5: the contract's version 6.
      raster: pipe('rasterVs', 'rasterFs', [{ arrayStride: 16, attributes: [{ shaderLocation: 0, offset: 0, format: 'float32x4' }] }], premul),
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

  /** The map services' tiles (render/serviceHub.ts). */
  useServices(services: ServiceSource | null): void {
    this.services = services;
  }

  // ── Upload ───────────────────────────────────────────────────────────

  upload(batches: readonly StyledBatch[]): GpuStyledLayer {
    const list: GpuStyled[] = [];
    for (const b of batches) {
      if (b.kind === 'fill' && (b.paint.kind === 'raster' || b.paint.kind === 'service')) {
        // A raster's (and a map service's) quads change every frame (docs/adr/0204 §5): written by prepare.
        const vertex = this.device.createBuffer({ size: 1024, usage: BUFFER.VERTEX | BUFFER.COPY_DST });
        const data = this.styleData(b);
        const style = this.device.createBuffer({ size: STYLE_BYTES, usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
        this.device.queue.writeBuffer(style, 0, data);
        const bind = this.device.createBindGroup({ layout: this.styleLayout, entries: [{ binding: 0, resource: { buffer: style } }] });
        list.push({ batch: b, vertex, count: 0, style, bind, data, visible: false, hit: null, quads: new RasterQuads(), vertexBytes: 1024 });
        continue;
      }
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
    // The tile the batch's numbers are from (docs/adr/0157): whole multiples of 2¹⁶ m, exact in float32.
    f.set([b.origin?.[0] ?? 0, b.origin?.[1] ?? 0, 0, 0], 36);
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
        // A family's stagger (docs/adr/0186 §3).
        f.set([p.stagger ?? 0, 0, 0, 0], 16);
        f.set([Math.cos(p.angle), Math.sin(p.angle), p.spacing, p.width], 20);
        f.set([p.offset, v.total, v.on, p.dashOffset], 24);
        u[32] = p.unit === 'world' ? 0 : 1;
      } else if (p.kind === 'gradient') {
        f.set(p.color, 0);
        f.set(p.color2, 4);
        f.set([Math.cos(p.dir), Math.sin(p.dir), p.from, p.to], 20);
        f.set([p.centre[0], p.centre[1], p.radius, 0], 24);
        u.set([0, p.shape, p.inverted ? 1 : 0, 0], 32);
      } else if (p.kind === 'image') {
        f.set([p.corner[0], p.corner[1], p.size[0], p.size[1]], 20);
        f.set([Math.cos(p.angle), Math.sin(p.angle), p.opacity, p.mirror ? 1 : 0], 24);
      } else if (p.kind === 'raster' || p.kind === 'service') {
        // b.z: the raster's (or the map service's) opacity (docs/adr/0204 §5, docs/adr/0208 §3).
        f.set([0, 0, p.opacity, 0], 24);
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

  /**
   * A picture's texture and group 0 (docs/adr/0192 §3): made once from its decoded pixels, premultiplied, its mip
   * levels drawn by the canvas; the grey stand-in while the atlas has none (looked for again next frame).
   */
  private picture(key: string, url: string | null): GpuPicture {
    const kept = this.pictures.get(key);
    if (kept) return kept;
    const img = this.atlas?.picture(key, url) ?? null;
    const name = img ? key : '';
    const known = this.pictures.get(name);
    if (known) return known;
    const { device } = this;
    let texture: GPUTexture;
    if (!img) {
      texture = device.createTexture({ size: [1, 1], format: 'rgba8unorm', usage: TEXTURE.COPY_DST | TEXTURE.TEXTURE_BINDING });
      device.queue.writeTexture({ texture }, new Uint8Array(GREY), { bytesPerRow: 4 }, [1, 1]);
    } else {
      const p = fitted(img, device.limits.maxTextureDimension2D);
      const levels = levelCount(p.width, p.height);
      texture = device.createTexture({ size: [p.width, p.height], mipLevelCount: levels, format: 'rgba8unorm', usage: TEXTURE.COPY_DST | TEXTURE.TEXTURE_BINDING | TEXTURE.RENDER_ATTACHMENT });
      for (let i = 0; i < levels; i++) {
        const source = i === 0 ? p.source : levelCanvas(p, i);
        const size = i === 0 ? [p.width, p.height] : [(source as HTMLCanvasElement).width, (source as HTMLCanvasElement).height];
        device.queue.copyExternalImageToTexture({ source: source as GPUCopyExternalImageSource }, { texture, mipLevel: i, premultipliedAlpha: true }, size);
      }
    }
    const bindFor = (buffer: GPUBuffer) =>
      device.createBindGroup({
        layout: this.frameLayout,
        entries: [
          { binding: 0, resource: { buffer } },
          { binding: 1, resource: texture.createView() },
          { binding: 2, resource: this.pictureSampler },
        ],
      });
    const made = { texture, frame: bindFor(this.frameBuffer), lens: bindFor(this.lensBuffer) };
    this.pictures.set(name, made);
    return made;
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
    this.prepareRasters(layers, f, view);
    for (let pass = 0; pass < 2; pass++) {
      const generation = atlas?.generation ?? 0;
      for (const layer of layers)
        for (const s of layer.list) {
          const b = s.batch;
          if (s.quads) continue;
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

  /** The raster atlas page, its samplers and groups, made the first time a raster draws. */
  private rasters(): RasterPage {
    if (this.rasterPage) return this.rasterPage;
    const { device } = this;
    const [w, h] = rasterAtlasSize(device.limits.maxTextureDimension2D);
    const texture = device.createTexture({ size: [w, h], format: 'rgba8unorm', usage: TEXTURE.COPY_DST | TEXTURE.TEXTURE_BINDING });
    const sampler = (filter: GPUFilterMode) => device.createSampler({ magFilter: filter, minFilter: filter, addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' });
    const [linear, nearest] = [sampler('linear'), sampler('nearest')];
    const group = (buffer: GPUBuffer, s: GPUSampler) =>
      device.createBindGroup({
        layout: this.frameLayout,
        entries: [
          { binding: 0, resource: { buffer } },
          { binding: 1, resource: texture.createView() },
          { binding: 2, resource: s },
        ],
      });
    return (this.rasterPage = {
      texture,
      slots: new RasterSlots(w, h),
      frame: { linear: group(this.frameBuffer, linear), nearest: group(this.frameBuffer, nearest) },
      lens: { linear: group(this.lensBuffer, linear), nearest: group(this.lensBuffer, nearest) },
    });
  }

  /**
   * Rasters' tiles this frame (docs/adr/0204 §5): each raster in view picks its tiles, new ones go into the atlas
   * page (at most `UPLOADS_PER_FRAME`), its quads into its vertex buffer, before the pass is encoded.
   */
  private prepareRasters(layers: readonly GpuStyledLayer[], f: StyledFrame, view: readonly [number, number, number, number]): void {
    const atlas = this.atlas;
    let started = false;
    const budget = { left: UPLOADS_PER_FRAME };
    for (const layer of layers)
      for (const s of layer.list) {
        if (!s.quads) continue;
        const b = s.batch;
        s.visible = false;
        if (b.kind === 'fill' && b.paint.kind === 'service') {
          if (!started) {
            started = true;
            atlas?.rasterFrame();
            this.rasters().slots.beginFrame();
            this.meshes.beginFrame();
            this.services?.frame();
          }
          this.prepareService(s, b.paint, f, view, budget);
          continue;
        }
        if (!atlas || b.kind !== 'fill' || b.paint.kind !== 'raster' || !inScale(b, f.scaleDenominator) || !batchInView(b, view, f.pxPerM, f.dpr)) continue;
        const page = this.rasters();
        if (!started) {
          started = true;
          page.slots.beginFrame();
          atlas.rasterFrame();
        }
        const put = (slot: number, rgba: Uint8Array) => {
          const [x, y] = page.slots.origin(slot);
          this.device.queue.writeTexture({ texture: page.texture, origin: { x, y } }, rgba as Uint8Array<ArrayBuffer>, { bytesPerRow: SLOT * 4, rowsPerImage: SLOT }, [SLOT, SLOT]);
        };
        rasterQuads(b.paint, b.origin ?? [0, 0], view, f.pxPerM, page.slots, atlas, put, budget, s.quads);
        s.count = s.quads.count;
        s.visible = s.count > 0;
        if (!s.visible) continue;
        const bytes = s.count * 16;
        if (bytes > (s.vertexBytes ?? 0)) {
          s.vertex.destroy();
          let size = s.vertexBytes ?? 1024;
          while (size < bytes) size *= 2;
          s.vertex = this.device.createBuffer({ size, usage: BUFFER.VERTEX | BUFFER.COPY_DST });
          s.vertexBytes = size;
        }
        this.device.queue.writeBuffer(s.vertex, 0, s.quads.data.buffer, s.quads.data.byteOffset, bytes);
      }
  }

  /**
   * A map service's batch this frame (docs/adr/0208 §3): its picture tiles' quads from the view's position tile (its
   * style's origin written again when it moves), or its vector tiles' batches uploaded once and shown as a layer's are.
   */
  private prepareService(s: GpuStyled, paint: ServicePaint, f: StyledFrame, view: readonly [number, number, number, number], budget: { left: number }): void {
    const services = this.services;
    s.vectors = undefined;
    if (!services || !s.quads) return;
    const grid = services.grid(paint.service);
    if (!grid) return;
    if (grid.vector) {
      const tiles = serviceVectors(paint, view, f.pxPerM / f.dpr, services);
      this.vectorFrame++;
      const layers: GpuStyledLayer[] = [];
      for (const t of tiles) {
        let held = this.vectorLayers.get(t.id);
        if (!held) {
          held = { layer: this.upload(t.batches), at: this.vectorFrame };
          this.vectorLayers.set(t.id, held);
        }
        held.at = this.vectorFrame;
        for (const v of held.layer.list) {
          const vb = v.batch;
          v.visible = inScale(vb, f.scaleDenominator) && batchInView(vb, view, f.pxPerM, f.dpr) && batchLegible(vb, f.pxPerM, f.dpr);
          const image = v.visible ? batchImage(vb) : null;
          if (!image) continue;
          const hit = this.atlas?.lookup(image, batchImagePx(vb, f.pxPerM, f.dpr)) ?? null;
          if (!hit) {
            v.visible = false;
            continue;
          }
          if (hit === v.hit) continue;
          v.hit = hit;
          const data = new Float32Array(v.data);
          data.set(hit.uv, 16);
          if (vb.kind === 'marker') data[24] = hit.aspect;
          this.device.queue.writeBuffer(v.style, 0, v.data);
        }
        layers.push(held.layer);
      }
      if (this.vectorLayers.size > 256)
        for (const [id, h] of this.vectorLayers)
          if (h.at < this.vectorFrame - 64) {
            this.release(h.layer);
            this.vectorLayers.delete(id);
          }
      s.vectors = layers;
      s.visible = layers.length > 0;
      return;
    }
    const page = this.rasters();
    const put = (slot: number, rgba: Uint8Array) => {
      const [x, y] = page.slots.origin(slot);
      this.device.queue.writeTexture({ texture: page.texture, origin: { x, y } }, rgba as Uint8Array<ArrayBuffer>, { bytesPerRow: SLOT * 4, rowsPerImage: SLOT }, [SLOT, SLOT]);
    };
    // The quads from the view's position tile (docs/adr/0157): the style's origin follows it.
    const origin = positionTile(f.cam[0], f.cam[1]);
    const b = s.batch as { origin?: [number, number] };
    if (b.origin?.[0] !== origin[0] || b.origin?.[1] !== origin[1]) {
      b.origin = origin;
      new Float32Array(s.data).set([origin[0], origin[1], 0, 0], 36);
      this.device.queue.writeBuffer(s.style, 0, s.data);
    }
    serviceQuads(paint, origin, view, f.pxPerM, page.slots, services, this.meshes, put, budget, s.quads);
    s.count = s.quads.count;
    s.visible = s.count > 0;
    if (!s.visible) return;
    const bytes = s.count * 16;
    if (bytes > (s.vertexBytes ?? 0)) {
      s.vertex.destroy();
      let size = s.vertexBytes ?? 1024;
      while (size < bytes) size *= 2;
      s.vertex = this.device.createBuffer({ size, usage: BUFFER.VERTEX | BUFFER.COPY_DST });
      s.vertexBytes = size;
    }
    this.device.queue.writeBuffer(s.vertex, 0, s.quads.data.buffer, s.quads.data.byteOffset, bytes);
  }

  /**
   * Draws a layer's visible batches into a pass of `samples` per pixel; with `lens`, through the magnifier's frame
   * uniform, only those reaching its view (docs/adr/0181).
   * True when it set its own group 0 (the frame with the atlas): the
   * plain pipelines drawn after it need theirs set again.
   */
  draw(pass: GPURenderPassEncoder, layer: GpuStyledLayer, samples: number, lens?: { view: readonly [number, number, number, number]; pxPerM: number; dpr: number }): boolean {
    if (!layer.list.some((s) => s.visible)) return false;
    const pipes = this.pipelinesFor(samples);
    pass.setBindGroup(0, lens ? this.lensBind : this.frameBind);
    let current: GPURenderPipeline | null = null;
    for (const s of layer.list) {
      if (!s.visible || (lens && !batchInView(s.batch, lens.view, lens.pxPerM, lens.dpr))) continue;
      const b = s.batch;
      // A vector service's tiles where it is, as layers (docs/adr/0208 §9); then on with this layer.
      if (s.vectors) {
        for (const v of s.vectors) this.draw(pass, v, samples, lens);
        pass.setBindGroup(0, lens ? this.lensBind : this.frameBind);
        current = null;
        continue;
      }
      const pipe = b.kind === 'stroke' ? pipes.stroke : b.kind === 'marker' ? pipes.marker : b.paint.kind === 'service' ? pipes.raster : pipes[b.paint.kind];
      if (pipe !== current) {
        pass.setPipeline(pipe);
        current = pipe;
      }
      pass.setBindGroup(1, s.bind);
      pass.setVertexBuffer(0, s.vertex);
      if (b.kind === 'fill' && (b.paint.kind === 'raster' || b.paint.kind === 'service')) {
        // The raster atlas in group 0, sampled as its look says (a service's linearly); then the atlas again.
        const page = this.rasters();
        const groups = lens ? page.lens : page.frame;
        pass.setBindGroup(0, b.paint.kind === 'raster' && b.paint.nearest ? groups.nearest : groups.linear);
        pass.draw(s.count);
        pass.setBindGroup(0, lens ? this.lensBind : this.frameBind);
      } else if (b.kind === 'fill' && b.paint.kind === 'image') {
        // Its own texture in group 0, then the atlas again for the batches after it.
        const picture = this.picture(b.paint.image, b.paint.url);
        pass.setBindGroup(0, lens ? picture.lens : picture.frame);
        pass.draw(s.count);
        pass.setBindGroup(0, lens ? this.lensBind : this.frameBind);
      } else if (b.kind === 'fill') pass.draw(s.count);
      else pass.draw(6, s.count);
    }
    return true;
  }

  dispose(): void {
    for (const h of this.vectorLayers.values()) this.release(h.layer);
    this.vectorLayers.clear();
    this.texture.destroy();
    this.rasterPage?.texture.destroy();
    for (const p of this.pictures.values()) p.texture.destroy();
    this.pictures.clear();
  }
}
