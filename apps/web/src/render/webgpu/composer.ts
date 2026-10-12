import COMPOSE_WGSL from '../../../../../shaders/wgsl/compose/compose.wgsl?raw';
import { texSize, type ComposePlan, type ComposeStep, type Into, type MapPipe, type Reads, type Tex } from '../composePlan';

/**
 * Katman çizimi and draw effects on WebGPU (docs/adr/0226 §2–§4): runs a composed frame's steps (../composePlan.ts)
 * with the shader the desktop builds (shaders/wgsl/compose, its contract compose.layout.json). One bind group layout:
 * the pass's numbers at a dynamic offset, six pictures and a linear clamping sampler. The pictures are kept between
 * frames at one size and sample count and made as a frame first needs each. The steps into the drawing's picture run
 * on in one render pass, resolved only where the picture is copied next and at the end; so do a layer's, from its
 * clear.
 */

const BUFFER = { VERTEX: 0x20, UNIFORM: 0x40, COPY_DST: 0x08 } as const;
const TEXTURE = { COPY_SRC: 0x01, COPY_DST: 0x02, TEXTURE_BINDING: 0x04, RENDER_ATTACHMENT: 0x10 } as const;
const STAGE = { VERTEX: 0x1, FRAGMENT: 0x2 } as const;
/** Bytes of a pass's numbers (`Params`: eight `vec4f`) and of a mask box (`boxInstance`). */
const PARAMS_BYTES = 128;
const BOX_BYTES = 28;
/** Floats of a mask box. */
const BOX_FLOATS = 7;

const PREMULTIPLIED: GPUBlendState = {
  color: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
  alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
};
const LARGEST: GPUBlendState = {
  color: { srcFactor: 'one', dstFactor: 'one', operation: 'max' },
  alpha: { srcFactor: 'one', dstFactor: 'one', operation: 'max' },
};

/** What running the steps needs of the backend. */
export interface ComposeHost {
  /**
   * Begins a pass into the drawing's picture: the frame's first cleared with its ground and underlays drawn, the
   * others with what it holds loaded; resolved into the base at its end when `resolve`.
   */
  picture(encoder: GPUCommandEncoder, first: boolean, resolve: boolean): GPURenderPassEncoder;
  /** Copies the drawing's picture, resolved, into `to`. */
  copyPicture(encoder: GPUCommandEncoder, to: GPUTexture): void;
  /** Draws the frame's layer `layer`'s batches `from` … `to` (and its plain parts at its ends) into `pass`. */
  draw(pass: GPURenderPassEncoder, layer: number, from: number, to: number): void;
}

/** A picture drawn into: multisampled resolving into one sample, or one sample only. */
interface Drawn {
  msaa: GPUTexture | null;
  resolve: GPUTexture;
}

export class WebGPUComposer {
  private readonly device: GPUDevice;
  private readonly format: GPUTextureFormat;
  private readonly module: GPUShaderModule;
  private readonly layout: GPUBindGroupLayout;
  private readonly pipelineLayout: GPUPipelineLayout;
  private readonly picturePipes = new Map<number, { over: GPURenderPipeline; blend: GPURenderPipeline; effect: GPURenderPipeline }>();
  private readonly maps: Record<MapPipe, GPURenderPipeline>;
  private readonly boxes: GPURenderPipeline;
  private readonly sampler: GPUSampler;
  private readonly none: GPUTexture;
  /** Slots of the numbers' buffer: the device's dynamic offset alignment. */
  private readonly slot: number;
  private size: readonly [number, number] = [0, 0];
  private samples = 0;
  private layer: Drawn | null = null;
  private group: Drawn | null = null;
  private readonly plain = new Map<Tex, GPUTexture>();
  private params: GPUBuffer | null = null;
  private boxBuffer: GPUBuffer | null = null;

  constructor(device: GPUDevice, format: GPUTextureFormat) {
    this.device = device;
    this.format = format;
    this.module = device.createShaderModule({ label: 'kentos.compose', code: COMPOSE_WGSL });
    const texture = (binding: number): GPUBindGroupLayoutEntry => ({ binding, visibility: STAGE.FRAGMENT, texture: { sampleType: 'float' } });
    this.layout = device.createBindGroupLayout({
      label: 'kentos.compose',
      entries: [
        { binding: 0, visibility: STAGE.VERTEX | STAGE.FRAGMENT, buffer: { type: 'uniform', hasDynamicOffset: true, minBindingSize: PARAMS_BYTES } },
        texture(1),
        texture(2),
        texture(3),
        texture(4),
        texture(5),
        texture(6),
        { binding: 7, visibility: STAGE.FRAGMENT, sampler: { type: 'filtering' } },
      ],
    });
    this.pipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [this.layout] });
    const map = (fragment: string) => this.pipeline('fullVs', fragment, 'rgba8unorm', undefined, 1, []);
    this.maps = {
      pack: map('packFs'),
      rows: map('rowsFs'),
      columns: map('columnsFs'),
      h: map('hFs'),
      v: map('vFs'),
      down: map('downFs'),
      downWide: map('downWideFs'),
      blurWide: map('blurWideFs'),
    };
    this.boxes = this.pipeline('boxVs', 'boxFs', 'r8unorm', LARGEST, 1, [
      {
        arrayStride: BOX_BYTES,
        stepMode: 'instance',
        attributes: [
          { shaderLocation: 0, offset: 0, format: 'float32x2' },
          { shaderLocation: 1, offset: 8, format: 'float32x2' },
          { shaderLocation: 2, offset: 16, format: 'float32x2' },
          { shaderLocation: 3, offset: 24, format: 'float32' },
        ],
      },
    ]);
    this.sampler = device.createSampler({ label: 'kentos.compose', magFilter: 'linear', minFilter: 'linear', addressModeU: 'clamp-to-edge', addressModeV: 'clamp-to-edge' });
    this.none = device.createTexture({ label: 'kentos.compose.none', size: [1, 1], format: 'r8unorm', usage: TEXTURE.TEXTURE_BINDING });
    this.slot = Math.max(PARAMS_BYTES, device.limits.minUniformBufferOffsetAlignment);
  }

  private pipeline(vertex: string, fragment: string, format: GPUTextureFormat, blend: GPUBlendState | undefined, samples: number, buffers: GPUVertexBufferLayout[]): GPURenderPipeline {
    return this.device.createRenderPipeline({
      label: `kentos.compose.${fragment}`,
      layout: this.pipelineLayout,
      vertex: { module: this.module, entryPoint: vertex, buffers },
      fragment: { module: this.module, entryPoint: fragment, targets: [{ format, ...(blend && { blend }) }] },
      primitive: { topology: 'triangle-list' },
      multisample: { count: samples },
    });
  }

  private pipes(samples: number) {
    let p = this.picturePipes.get(samples);
    if (!p) {
      p = {
        over: this.pipeline('fullVs', 'overFs', this.format, PREMULTIPLIED, samples, []),
        blend: this.pipeline('fullVs', 'blendFs', this.format, undefined, samples, []),
        effect: this.pipeline('fullVs', 'effectFs', this.format, PREMULTIPLIED, samples, []),
      };
      this.picturePipes.set(samples, p);
    }
    return p;
  }

  /** Drops the pipelines of a sample count the device refused. */
  forget(samples: number): void {
    this.picturePipes.delete(samples);
  }

  /** The pictures for a new size or sample count: the old ones go once the GPU is done with them. */
  private ensure(size: readonly [number, number], samples: number): void {
    if (this.size[0] === size[0] && this.size[1] === size[1] && this.samples === samples) return;
    const old = [this.layer?.msaa, this.layer?.resolve, this.group?.msaa, this.group?.resolve, ...this.plain.values()];
    void this.device.queue.onSubmittedWorkDone().then(() => old.forEach((t) => t?.destroy()));
    this.layer = this.group = null;
    this.plain.clear();
    this.size = [size[0], size[1]];
    this.samples = samples;
  }

  private drawn(label: string): Drawn {
    const [w, h] = this.size;
    return {
      msaa: this.samples > 1 ? this.device.createTexture({ label, size: [w, h], sampleCount: this.samples, format: this.format, usage: TEXTURE.RENDER_ATTACHMENT }) : null,
      resolve: this.device.createTexture({ label, size: [w, h], format: this.format, usage: TEXTURE.RENDER_ATTACHMENT | TEXTURE.TEXTURE_BINDING }),
    };
  }

  /** A picture's texture, made the first time a frame needs it. */
  private texture(t: Tex): GPUTexture {
    if (t === 'none') return this.none;
    if (t === 'layer') return (this.layer ??= this.drawn('kentos.compose.layer')).resolve;
    if (t === 'group') return (this.group ??= this.drawn('kentos.compose.group')).resolve;
    let made = this.plain.get(t);
    if (!made) {
      const [w, h] = texSize(t, this.size);
      const format: GPUTextureFormat = t === 'backdrop' ? this.format : t === 'mask' ? 'r8unorm' : 'rgba8unorm';
      const usage = TEXTURE.TEXTURE_BINDING | (t === 'backdrop' ? TEXTURE.COPY_DST : TEXTURE.RENDER_ATTACHMENT);
      made = this.device.createTexture({ label: `kentos.compose.${t}`, size: [w, h], format, usage });
      this.plain.set(t, made);
    }
    return made;
  }

  /** A pass into a drawn picture: cleared to transparent, or loaded; the multisampled texture kept. */
  private attachment(d: Drawn, clear: boolean): GPURenderPassColorAttachment {
    const load = clear ? ('clear' as const) : ('load' as const);
    return d.msaa
      ? { view: d.msaa.createView(), resolveTarget: d.resolve.createView(), clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: load, storeOp: 'store' }
      : { view: d.resolve.createView(), clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: load, storeOp: 'store' };
  }

  /**
   * Records `plan`'s steps after the picture's first pass (its ground and underlays, drawn by the backend) into
   * `encoder`, for a target of `size` device pixels and `samples` per pixel.
   */
  record(encoder: GPUCommandEncoder, plan: ComposePlan, size: readonly [number, number], samples: number, host: ComposeHost): void {
    const device = this.device;
    this.ensure(size, samples);
    for (const t of plan.needs) this.texture(t);
    const pipes = this.pipes(samples);
    // The passes' numbers in one buffer at dynamic offsets, the masks' boxes in another.
    const numbered = plan.steps.filter((s): s is Extract<ComposeStep, { params: Float32Array }> => 'params' in s);
    const need = Math.max(1, numbered.length) * this.slot;
    if (!this.params || this.params.size < need) {
      this.params?.destroy();
      this.params = device.createBuffer({ label: 'kentos.compose.params', size: 2 ** Math.ceil(Math.log2(need)), usage: BUFFER.UNIFORM | BUFFER.COPY_DST });
    }
    const bytes = new Float32Array((numbered.length * this.slot) / 4);
    const offsets = new Map<ComposeStep, number>();
    numbered.forEach((s, i) => {
      bytes.set(s.params, (i * this.slot) / 4);
      offsets.set(s, i * this.slot);
    });
    if (bytes.length) device.queue.writeBuffer(this.params, 0, bytes);
    const boxSteps = plan.steps.filter((s): s is Extract<ComposeStep, { op: 'boxes' }> => s.op === 'boxes');
    const boxCount = boxSteps.reduce((n, s) => n + s.boxes.length / BOX_FLOATS, 0);
    const boxRanges = new Map<ComposeStep, [number, number]>();
    if (boxCount) {
      const need = boxCount * BOX_BYTES;
      if (!this.boxBuffer || this.boxBuffer.size < need) {
        this.boxBuffer?.destroy();
        this.boxBuffer = device.createBuffer({ label: 'kentos.compose.boxes', size: 2 ** Math.ceil(Math.log2(need)), usage: BUFFER.VERTEX | BUFFER.COPY_DST });
      }
      const all = new Float32Array(boxCount * BOX_FLOATS);
      let at = 0;
      for (const s of boxSteps) {
        all.set(s.boxes, at * BOX_FLOATS);
        boxRanges.set(s, [at, s.boxes.length / BOX_FLOATS]);
        at += s.boxes.length / BOX_FLOATS;
      }
      device.queue.writeBuffer(this.boxBuffer, 0, all);
    }
    const params = this.params;
    const bind = (reads: Reads) =>
      device.createBindGroup({
        layout: this.layout,
        entries: [
          { binding: 0, resource: { buffer: params, size: PARAMS_BYTES } },
          ...reads.map((t, k) => ({ binding: 1 + k, resource: this.texture(t).createView() })),
          { binding: 7, resource: this.sampler },
        ],
      });
    const full = (pass: GPURenderPassEncoder, pipeline: GPURenderPipeline, s: ComposeStep & { reads: Reads }) => {
      pass.setPipeline(pipeline);
      pass.setBindGroup(0, bind(s.reads), [offsets.get(s) ?? 0]);
      pass.draw(3);
    };
    // A step of a run into the picture or the layer.
    const step = (pass: GPURenderPassEncoder, s: ComposeStep): void => {
      if (s.op === 'batches') host.draw(pass, s.layer, s.from, s.to);
      else if (s.op === 'effect') full(pass, pipes.effect, s);
      else if (s.op === 'layer') full(pass, s.blend ? pipes.blend : pipes.over, s);
    };
    const steps = plan.steps;
    let first = true;
    let i = 0;
    for (;;) {
      const s = steps[i];
      const where = s ? drawnInto(s) : null;
      if (where === 'picture') {
        // The picture's run: on until a step of another place; resolved when the drawing is copied before its next run.
        const start = i;
        while (i < steps.length && drawnInto(steps[i]) === 'picture') i++;
        let copied = false;
        let last = true;
        for (let k = i; k < steps.length; k++) {
          if (drawnInto(steps[k]) === 'picture') {
            last = false;
            break;
          }
          if (steps[k].op === 'backdrop') copied = true;
        }
        const pass = host.picture(encoder, first, last || copied);
        for (let k = start; k < i; k++) step(pass, steps[k]);
        pass.end();
        first = false;
      } else if (where === 'layer') {
        // The layer's run: cleared when it starts with its clear, on while it draws into the layer.
        const clear = s.op === 'layerClear';
        if (clear) i++;
        const start = i;
        while (i < steps.length && drawnInto(steps[i]) === 'layer' && steps[i].op !== 'layerClear') i++;
        const pass = encoder.beginRenderPass({ label: 'kentos.compose.layer', colorAttachments: [this.attachment((this.layer ??= this.drawn('kentos.compose.layer')), clear)] });
        for (let k = start; k < i; k++) step(pass, steps[k]);
        pass.end();
      } else if (!s) {
        // A frame that never drew into the picture still has its ground.
        if (first) host.picture(encoder, true, true).end();
        break;
      } else if (s.op === 'backdrop' && first) {
        // The ground, before the drawing is first copied.
        host.picture(encoder, true, true).end();
        first = false;
      } else {
        this.own(encoder, s, { bind, full, offsets, boxRanges, host });
        i++;
      }
    }
  }

  /** A step into a picture of its own: an effect group's, a map, the mask; or the backdrop's copy. */
  private own(encoder: GPUCommandEncoder, s: ComposeStep, at: Recording): void {
    const cleared = (view: GPUTextureView): GPURenderPassColorAttachment => ({ view, clearValue: { r: 0, g: 0, b: 0, a: 0 }, loadOp: 'clear', storeOp: 'store' });
    if (s.op === 'group') {
      const pass = encoder.beginRenderPass({ label: 'kentos.compose.group', colorAttachments: [this.attachment((this.group ??= this.drawn('kentos.compose.group')), true)] });
      at.host.draw(pass, s.layer, s.from, s.to);
      pass.end();
    } else if (s.op === 'map') {
      const pass = encoder.beginRenderPass({ label: 'kentos.compose.map', colorAttachments: [cleared(this.texture(s.target).createView())] });
      at.full(pass, this.maps[s.pipe], s);
      pass.end();
    } else if (s.op === 'boxes') {
      const range = at.boxRanges.get(s);
      const pass = encoder.beginRenderPass({ label: 'kentos.compose.mask', colorAttachments: [cleared(this.texture('mask').createView())] });
      if (range && this.boxBuffer) {
        pass.setPipeline(this.boxes);
        pass.setBindGroup(0, at.bind(NONE), [at.offsets.get(s) ?? 0]);
        pass.setVertexBuffer(0, this.boxBuffer);
        pass.draw(6, range[1], 0, range[0]);
      }
      pass.end();
    } else if (s.op === 'backdrop') at.host.copyPicture(encoder, this.texture('backdrop'));
  }

  /** Bytes of the pictures held, for the frame's statistics. */
  get bytes(): number {
    const [w, h] = this.size;
    let n = 0;
    for (const d of [this.layer, this.group]) if (d) n += w * h * 4 * (this.samples + (d.msaa ? 1 : 0));
    for (const [t, tex] of this.plain) n += tex.width * tex.height * (t === 'mask' ? 1 : 4);
    return n;
  }

  dispose(): void {
    for (const t of [this.layer?.msaa, this.layer?.resolve, this.group?.msaa, this.group?.resolve, ...this.plain.values(), this.none, this.params, this.boxBuffer]) t?.destroy();
    this.plain.clear();
    this.layer = this.group = null;
  }
}

/** No pictures read. */
const NONE: Reads = ['none', 'none', 'none', 'none', 'none', 'none'];

/** A frame's recording as its steps into pictures of their own need it. */
interface Recording {
  bind(reads: Reads): GPUBindGroup;
  full(pass: GPURenderPassEncoder, pipeline: GPURenderPipeline, s: ComposeStep & { reads: Reads }): void;
  offsets: Map<ComposeStep, number>;
  boxRanges: Map<ComposeStep, [number, number]>;
  host: ComposeHost;
}

/** Where a step draws: the picture or the layer (whose run a clear starts), or null: a picture of its own. */
function drawnInto(s: ComposeStep): Into | null {
  if (s.op === 'batches' || s.op === 'effect') return s.into;
  if (s.op === 'layer') return 'picture';
  if (s.op === 'layerClear') return 'layer';
  return null;
}
