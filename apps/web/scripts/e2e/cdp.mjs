// Minimal headless-Chrome driver over the DevTools protocol (no dependencies).
// Chrome binary: $CHROME_BIN or `google-chrome`. Screenshots go to scripts/e2e/out/.
import { spawn } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export const OUT = join(new URL('.', import.meta.url).pathname, 'out');
/**
 * DevTools port: 0 lets Chrome pick a free one (read back from the profile's
 * DevToolsActivePort file), so several browsers can run side by side. A
 * fixed port made a second browser attach to the first one's page.
 */
const PORT = Number(process.env.CDP_PORT ?? 0);

/**
 * Each browser's clean-up, run again when this script ends: one 'exit' listener for them all. A
 * listener per browser made Node warn past ten (MaxListenersExceededWarning) in visual.mjs, which
 * opens a browser per page it compares.
 */
const atExit = [];
process.once('exit', () => atExit.forEach((f) => f()));

/**
 * Flags that give headless Chrome a working WebGPU device on SwiftShader.
 * With --enable-unsafe-webgpu alone an adapter is found but the device is
 * dropped at the first submit; Vulkan through ANGLE keeps it alive.
 */
export const WEBGPU_ARGS = ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--use-vulkan=swiftshader', '--use-webgpu-adapter=swiftshader'];

export async function launch(url, { width = 1600, height = 900, args = [] } = {}) {
  mkdirSync(OUT, { recursive: true });
  const profile = mkdtempSync(join(tmpdir(), 'kentos-e2e-'));
  const proc = spawn(process.env.CHROME_BIN ?? 'google-chrome', [
    '--headless=new',
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--enable-unsafe-swiftshader',
    '--use-angle=swiftshader',
    `--window-size=${width},${height}`,
    ...args,
    'about:blank',
  ], { stdio: 'ignore' });
  // The profile (about 150 MB) goes when Chrome has closed, and at the latest when this
  // script ends, whether or not a check failed first: /tmp is small (a RAM disk).
  const removeProfile = () => {
    try {
      rmSync(profile, { recursive: true, force: true, maxRetries: 3 });
    } catch {}
  };
  proc.once('exit', removeProfile);
  atExit.push(() => {
    proc.kill('SIGKILL');
    removeProfile();
  });
  let port = PORT;
  let targets;
  for (let i = 0; i < 100; i++) {
    try {
      if (!port) port = Number(readFileSync(join(profile, 'DevToolsActivePort'), 'utf8').split('\n')[0]);
      targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      if (targets.some((t) => t.type === 'page')) break;
    } catch {}
    await sleep(100);
  }
  const page = targets.find((t) => t.type === 'page');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener('open', r, { once: true }));
  let id = 0;
  const pending = new Map();
  const consoleLog = [];
  const listeners = new Map();
  ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.method) for (const fn of listeners.get(msg.method) ?? []) fn(msg.params);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
    } else if (msg.method === 'Runtime.consoleAPICalled') {
      consoleLog.push(`${msg.params.type}: ${msg.params.args.map((a) => a.value ?? a.description).join(' ')}`);
    } else if (msg.method === 'Runtime.exceptionThrown') {
      consoleLog.push(`EXCEPTION: ${msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text}`);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const mid = ++id;
      pending.set(mid, { resolve, reject });
      ws.send(JSON.stringify({ id: mid, method, params }));
    });
  await send('Runtime.enable');
  await send('Page.enable');
  await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url });
  const api = {
    send,
    consoleLog,
    /** The browser process (scripts/perf/interaction.mjs sums its process tree's memory). */
    pid: proc.pid,
    /** Subscribe to a DevTools event (e.g. Page.screencastFrame). */
    on(method, fn) {
      if (!listeners.has(method)) listeners.set(method, []);
      listeners.get(method).push(fn);
    },
    async eval(expr) {
      const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
      return r.result.value;
    },
    async waitFor(expr, ms = 10000) {
      const t0 = Date.now();
      while (Date.now() - t0 < ms) {
        if (await api.eval(`!!(${expr})`).catch(() => false)) return;
        await sleep(100);
      }
      throw new Error(`timeout waiting for ${expr}`);
    },
    async move(x, y) {
      await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none' });
    },
    async click(x, y, { clickCount = 1, modifiers = 0 } = {}) {
      await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y, button: 'none', modifiers });
      await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount, modifiers });
      await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount, modifiers });
      await sleep(40);
    },
    async drag(x1, y1, x2, y2) {
      await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x1, y: y1, button: 'none' });
      await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: x1, y: y1, button: 'left', clickCount: 1 });
      for (let i = 1; i <= 8; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: x1 + ((x2 - x1) * i) / 8, y: y1 + ((y2 - y1) * i) / 8, button: 'left', buttons: 1 });
      await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: x2, y: y2, button: 'left', clickCount: 1 });
      await sleep(40);
    },
    async key(key, { shift = false, alt = false, ctrl = false } = {}) {
      const named = {
        Enter: [13, 'Enter', '\r'],
        Escape: [27, 'Escape'],
        ' ': [32, 'Space', ' '],
        Delete: [46, 'Delete'],
        F1: [112, 'F1'],
        F10: [121, 'F10'],
        F6: [117, 'F6'],
        Alt: [18, 'AltLeft'],
        F3: [114, 'F3'],
        F8: [119, 'F8'],
        // Printable keys whose char code is not their virtual key (46 would be Delete).
        '.': [190, 'Period', '.'],
        ',': [188, 'Comma', ','],
        '-': [189, 'Minus', '-'],
      };
      let vk, code, text;
      if (named[key]) [vk, code, text] = named[key];
      else {
        vk = key.toUpperCase().charCodeAt(0);
        code = /[a-z]/i.test(key) ? `Key${key.toUpperCase()}` : `Digit${key}`;
        text = shift ? key.toUpperCase() : key;
      }
      const modifiers = (alt ? 1 : 0) | (ctrl ? 2 : 0) | (shift ? 8 : 0);
      const k = shift && key.length === 1 ? key.toUpperCase() : key;
      await send('Input.dispatchKeyEvent', { type: text && !ctrl && !alt ? 'keyDown' : 'rawKeyDown', key: k, code, windowsVirtualKeyCode: vk, text: ctrl || alt ? undefined : text, modifiers });
      await send('Input.dispatchKeyEvent', { type: 'keyUp', key: k, code, windowsVirtualKeyCode: vk, modifiers });
      await sleep(30);
    },
    async type(text) {
      await send('Input.insertText', { text });
      await sleep(30);
    },
    async shot(name, clip, dir = OUT) {
      await sleep(120);
      const r = await send('Page.captureScreenshot', { format: 'png', ...(clip ? { clip: { ...clip, scale: 1 } } : {}) });
      const file = join(dir, `${name}.png`);
      writeFileSync(file, Buffer.from(r.data, 'base64'));
      return file;
    },
    close() {
      ws.close();
      proc.kill();
    },
  };
  return api;
}

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
