// The shared WGSL in a real browser's WebGPU (TODOS.md REN-05, docs/adr/0019).
// The native side is checked by naga (crates/render/wgpu/tests/wgsl_contract.rs);
// this runs the same module through Chrome's own compiler (Tint, in Dawn):
//
//   1. the module is joined from shaders/wgsl as cad2d.layout.json lists it,
//      the way crates/render/wgpu/src/shader.rs joins it, and compiled;
//      any error from getCompilationInfo fails the check;
//   2. every pipeline of the contract is built from the JSON alone (bind
//      group, vertex buffers, blend) inside a validation error scope: the
//      browser holds the entry points to the contract's layouts;
//   3. a frame is drawn at Turkish TM coordinates (E 487 000, N 4 420 000)
//      from buffers written at the JSON's offsets, with float64 split into
//      float32 high/low parts in JavaScript, and read back: the background,
//      a fill and a line whose position must match float64 to 0.05 px.
//
// Headless Chrome gets WebGPU on SwiftShader with the flags of
// apps/web/scripts/e2e/cdp.mjs (a correctness check, not a GPU benchmark,
// CLAUDE.md §9.5). The canvas is saved as .run/viewport-browser-wgsl.png.
//
//   node scripts/wgsl/browser-check.mjs
import { mkdirSync, readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { WEBGPU_ARGS, launch } from '../../apps/web/scripts/e2e/cdp.mjs';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const WGSL = join(ROOT, 'shaders/wgsl');
const layout = JSON.parse(readFileSync(join(WGSL, 'cad2d.layout.json'), 'utf8'));
const source = layout.sources.map((path) => `// ── ${path} ──\n${readFileSync(join(WGSL, path), 'utf8')}\n`).join('');

const PAGE = `<!doctype html><meta charset="utf-8"><title>KentOS WGSL</title>
<style>body{margin:0;background:#222}canvas{width:640px;height:400px;image-rendering:pixelated}</style>
<canvas id="c" width="320" height="200"></canvas>`;

/** Runs in the page: compiles, builds the pipelines, draws and reads back. */
async function check(source, layout) {
  const result = { adapter: '', messages: [], pipelineError: null, readback: null, fatal: null };
  if (!navigator.gpu) return { ...result, fatal: 'navigator.gpu yok: güvenli bağlam ya da WebGPU bayrakları eksik' };
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) return { ...result, fatal: 'WebGPU bağdaştırıcısı bulunamadı' };
  const info = adapter.info ?? {};
  result.adapter = [info.vendor, info.architecture, info.description].filter(Boolean).join(' ') || 'bilinmiyor';
  const device = await adapter.requestDevice();

  // 1. Compile.
  const module = device.createShaderModule({ label: layout.module, code: source });
  const compiled = await module.getCompilationInfo();
  result.messages = compiled.messages.map((m) => ({ type: m.type, text: m.message, line: m.lineNum, column: m.linePos }));
  if (result.messages.some((m) => m.type === 'error')) return result;

  // 2. The contract's pipelines, from the JSON alone.
  const canvas = document.getElementById('c');
  const context = canvas.getContext('webgpu');
  const format = navigator.gpu.getPreferredCanvasFormat();
  context.configure({ device, format, alphaMode: 'opaque', usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
  const stage = (names) => (names.includes('vertex') ? GPUShaderStage.VERTEX : 0) | (names.includes('fragment') ? GPUShaderStage.FRAGMENT : 0);
  device.pushErrorScope('validation');
  const group = layout.bindGroups[0];
  const bindLayout = device.createBindGroupLayout({
    entries: group.bindings.map((b) => ({ binding: b.binding, visibility: stage(b.visibility), buffer: { type: b.type, minBindingSize: layout.structs[b.struct].size } })),
  });
  const pipelineLayout = device.createPipelineLayout({ bindGroupLayouts: [bindLayout] });
  const pipelines = {};
  for (const p of layout.pipelines) {
    pipelines[p.name] = device.createRenderPipeline({
      label: p.name,
      layout: pipelineLayout,
      vertex: {
        module,
        entryPoint: p.vertex,
        buffers: p.buffers.map((b) => ({
          arrayStride: b.arrayStride,
          stepMode: b.stepMode,
          attributes: b.attributes.map((a) => ({ shaderLocation: a.location, offset: a.offset, format: a.format })),
        })),
      },
      fragment: { module, entryPoint: p.fragment, targets: [{ format, blend: layout.blends[p.blend] ?? undefined }] },
      primitive: { topology: p.topology },
    });
  }
  const invalid = await device.popErrorScope();
  if (invalid) return { ...result, pipelineError: invalid.message };

  // 3. A frame at TM coordinates, buffers written at the contract's offsets.
  const split = (v) => {
    const hi = Math.fround(v);
    return [hi, Math.fround(v - hi)];
  };
  const origin = [486512.34, 4420187.52];
  const E = 487012.346;
  const N = 4420187.521;
  const W = canvas.width;
  const H = canvas.height;
  const scale = 40; // px per metre
  const camera = [E, N];
  const field = (struct, name) => layout.structs[struct].fields.find((f) => f.name === name).offset;
  const frame = new ArrayBuffer(layout.structs.Frame.size);
  const f32 = new Float32Array(frame);
  const u32 = new Uint32Array(frame);
  const put = (name, values) => f32.set(values, field('Frame', name) / 4);
  const [cxh, cxl] = split(camera[0] - origin[0]);
  const [cyh, cyl] = split(camera[1] - origin[1]);
  put('center_hi', [cxh, cyh]);
  put('center_lo', [cxl, cyl]);
  put('viewport', [W, H]);
  put('px_per_unit', [scale]);
  put('dpi', [1]);
  put('background', [0x14 / 255, 0x1a / 255, 0x21 / 255, 1]);
  put('line_width', [1]);
  u32[field('Frame', 'srgb_target') / 4] = format.endsWith('-srgb') ? 1 : 0;
  const uniform = device.createBuffer({ size: frame.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
  device.queue.writeBuffer(uniform, 0, frame);
  const bind = device.createBindGroup({ layout: bindLayout, entries: [{ binding: 0, resource: { buffer: uniform } }] });

  const pipeline = (name) => layout.pipelines.find((p) => p.name === name);
  /** A vertex buffer of `records`, each a map from attribute name to values, at the contract's offsets. */
  const vertices = (name, records) => {
    const buffer = pipeline(name).buffers[0];
    const bytes = new ArrayBuffer(buffer.arrayStride * records.length);
    const view = new DataView(bytes);
    records.forEach((record, i) => {
      for (const a of buffer.attributes) {
        const at = i * buffer.arrayStride + a.offset;
        const value = record[a.name];
        if (a.format === 'float32x2') value.forEach((v, k) => view.setFloat32(at + 4 * k, v, true));
        else if (a.format === 'float32') view.setFloat32(at, value, true);
        else if (a.format === 'uint32') view.setUint32(at, value, true);
        else if (a.format === 'unorm8x4') value.forEach((v, k) => view.setUint8(at + k, v));
      }
    });
    const gpu = device.createBuffer({ size: bytes.byteLength, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
    device.queue.writeBuffer(gpu, 0, bytes);
    return gpu;
  };
  const parts = (x, y) => {
    const [xh, xl] = split(x - origin[0]);
    const [yh, yl] = split(y - origin[1]);
    return { hi: [xh, yh], lo: [xl, yl] };
  };
  const red = [0xe0, 0x6c, 0x75, 0xff];
  const white = [0xff, 0xff, 0xff, 0xff];
  // A filled triangle left of the centre, a vertical line 0.0003 m right of it (0.012 px), a mark.
  const tri = [parts(E - 3.5, N - 1.5), parts(E - 0.5, N - 1.5), parts(E - 2, N + 1.5)].map((p) => ({ ...p, color: red }));
  const lineX = E + 1.0003;
  const a = parts(lineX, N - 2);
  const b = parts(lineX, N + 2);
  const segment = { a_hi: a.hi, a_lo: a.lo, b_hi: b.hi, b_lo: b.lo, color: white };
  const mark = { ...parts(E + 2.5, N), color: white, size: 9, shape: 0 };
  const fills = vertices('fill', tri);
  const segments = vertices('line', [segment]);
  const marks = vertices('marker', [mark]);

  const target = context.getCurrentTexture();
  const encoder = device.createCommandEncoder();
  const pass = encoder.beginRenderPass({ colorAttachments: [{ view: target.createView(), loadOp: 'clear', clearValue: { r: 1, g: 0, b: 0, a: 1 }, storeOp: 'store' }] });
  pass.setBindGroup(0, bind);
  pass.setPipeline(pipelines.background);
  pass.draw(pipeline('background').vertexCount);
  pass.setPipeline(pipelines.fill);
  pass.setVertexBuffer(0, fills);
  pass.draw(3);
  pass.setPipeline(pipelines.line);
  pass.setVertexBuffer(0, segments);
  pass.draw(pipeline('line').vertexCount, 1);
  pass.setPipeline(pipelines.marker);
  pass.setVertexBuffer(0, marks);
  pass.draw(pipeline('marker').vertexCount, 1);
  pass.end();
  const row = Math.ceil((W * 4) / 256) * 256;
  const read = device.createBuffer({ size: row * H, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ });
  encoder.copyTextureToBuffer({ texture: target }, { buffer: read, bytesPerRow: row }, [W, H]);
  device.queue.submit([encoder.finish()]);
  await read.mapAsync(GPUMapMode.READ);
  const data = new Uint8Array(read.getMappedRange().slice(0));
  read.unmap();
  // RGBA whatever the canvas stores (bgra8unorm swaps red and blue).
  const bgra = format.startsWith('bgra');
  const pixel = (x, y) => {
    const i = y * row + x * 4;
    return bgra ? [data[i + 2], data[i + 1], data[i], data[i + 3]] : [data[i], data[i + 1], data[i + 2], data[i + 3]];
  };
  // The line's column: coverage-weighted centre of row H/2 around it.
  const y = Math.floor(H / 2);
  let sum = 0;
  let weight = 0;
  for (let x = Math.floor(W / 2) + 30; x < Math.floor(W / 2) + 50; x++) {
    const v = (pixel(x, y)[1] - 0x1a) / (0xff - 0x1a);
    if (v > 0) {
      sum += (x + 0.5) * v;
      weight += v;
    }
  }
  result.readback = {
    format,
    background: pixel(4, 4),
    fill: pixel(Math.round(W / 2 - 2 * scale), Math.round(H / 2)),
    lineCentre: weight > 0 ? sum / weight : null,
    lineExpected: W / 2 + (lineX - E) * scale,
  };
  return result;
}

const server = createServer((req, res) => {
  res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  res.end(PAGE);
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/`;
const browser = await launch(url, { width: 700, height: 440, args: WEBGPU_ARGS });
let failed = [];
try {
  await browser.waitFor(`document.getElementById('c')`);
  const result = await browser.eval(`(${check.toString()})(${JSON.stringify(source)}, ${JSON.stringify(layout)})`);
  console.log(`WebGPU: ${result.adapter || '—'}; ${layout.module} v${layout.version}, ${layout.sources.length} dosya, ${source.split('\n').length} satır`);
  for (const m of result.messages) console.log(`  ${m.type}: ${m.text} (satır ${m.line}:${m.column})`);
  if (result.fatal) failed.push(result.fatal);
  if (result.messages.some((m) => m.type === 'error')) failed.push('WGSL derlenmedi (getCompilationInfo hataları yukarıda)');
  if (result.pipelineError) failed.push(`sözleşmedeki boru hatları kurulamadı: ${result.pipelineError}`);
  const r = result.readback;
  if (r) {
    const near = (p, q, d = 3) => p.every((v, i) => Math.abs(v - q[i]) <= d);
    console.log(`  hedef ${r.format}; zemin ${r.background}; dolgu ${r.fill}; çizgi ${r.lineCentre?.toFixed(4)} px (float64: ${r.lineExpected.toFixed(4)})`);
    if (!near(r.background, [0x14, 0x1a, 0x21, 0xff])) failed.push(`zemin rengi yanlış: ${r.background}`);
    if (!near(r.fill, [0xe0, 0x6c, 0x75, 0xff])) failed.push(`dolgu rengi yanlış: ${r.fill}`);
    if (r.lineCentre === null || Math.abs(r.lineCentre - r.lineExpected) > 0.05) failed.push(`çizgi yerinde değil: ${r.lineCentre} px, beklenen ${r.lineExpected}`);
    mkdirSync(join(ROOT, '.run'), { recursive: true });
    const file = await browser.shot('viewport-browser-wgsl', { x: 0, y: 0, width: 640, height: 400 }, join(ROOT, '.run'));
    console.log(`  görüntü: ${file}`);
  }
  for (const line of browser.consoleLog) if (/error|warn/i.test(line)) console.log(`  konsol: ${line}`);
} catch (error) {
  failed.push(String(error));
} finally {
  browser.close();
  server.close();
}
if (failed.length) {
  console.error(`WGSL tarayıcı denetimi düştü:\n${failed.map((f) => `  ${f}`).join('\n')}`);
  process.exit(1);
}
console.log('WGSL tarayıcıda derlendi, sözleşmedeki boru hatları kuruldu, kare doğru çizildi.');
