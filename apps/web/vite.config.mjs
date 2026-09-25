// Vite configuration. Workers are ES modules (the processing worker starts
// its own copy of the Rust geometry core, src/wasm/core.ts); tests start the
// core first (src/wasm/testSetup.ts). Requests under /v1/
// go to the local KentOS API (apps/api, `pnpm api`), in both `vite` and
// `vite preview`, and so does the project WebSocket (/v1/ws). When the API is
// not running the answer is a quiet 503, so the app shows "Sunucu: yok"
// without filling the terminal with proxy errors (Vite's own proxy logs every
// refused connection).
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import { extname, join, resolve } from 'node:path';
import { brotliCompressSync, constants as zc, gzipSync } from 'node:zlib';
import { defineConfig } from 'vite';

const API_PORT = Number(process.env.KENTOS_API_PORT ?? 8787);

/** Forwards /v1/* to 127.0.0.1:KENTOS_API_PORT. */
function kentosApi() {
  const forward = (req, res, next) => {
    if (!req.url?.startsWith('/v1/')) return next();
    const upstream = http.request({ host: '127.0.0.1', port: API_PORT, path: req.url, method: req.method, headers: { ...req.headers, host: `127.0.0.1:${API_PORT}` } }, (r) => {
      res.writeHead(r.statusCode ?? 502, r.headers);
      r.pipe(res);
    });
    upstream.setTimeout(10000, () => upstream.destroy(new Error('timeout')));
    upstream.on('error', () => {
      if (!res.headersSent) res.writeHead(503, { 'content-type': 'application/json', 'cache-control': 'no-store' });
      res.end('{"error":"api-unreachable"}');
    });
    req.pipe(upstream);
  };
  // WebSocket upgrades never reach the middleware: /v1/ws is passed through as raw TCP. Vite's own HMR socket is left alone.
  const upgrade = (httpServer) =>
    httpServer?.on('upgrade', (req, socket, head) => {
      if (!req.url?.startsWith('/v1/ws')) return;
      const upstream = net.connect(API_PORT, '127.0.0.1', () => {
        const lines = [`${req.method} ${req.url} HTTP/${req.httpVersion}`];
        for (let i = 0; i < req.rawHeaders.length; i += 2) {
          const name = req.rawHeaders[i];
          lines.push(`${name}: ${name.toLowerCase() === 'host' ? `127.0.0.1:${API_PORT}` : req.rawHeaders[i + 1]}`);
        }
        upstream.write(`${lines.join('\r\n')}\r\n\r\n`);
        if (head?.length) upstream.write(head);
        upstream.pipe(socket);
        socket.pipe(upstream);
      });
      upstream.on('error', () => socket.destroy());
      socket.on('error', () => upstream.destroy());
    });
  return {
    name: 'kentos-api',
    configureServer: (server) => {
      server.middlewares.use(forward);
      upgrade(server.httpServer);
    },
    configurePreviewServer: (server) => {
      server.middlewares.use(forward);
      upgrade(server.httpServer);
    },
  };
}

/** Files worth compressing, and the type each is served with. */
const COMPRESSED = { '.wasm': 'application/wasm', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.svg': 'image/svg+xml', '.html': 'text/html' };

/**
 * Compressed copies of the build (CLAUDE.md §20): every script, style sheet
 * and WASM module of dist/ gets a Brotli (.br) and a gzip (.gz) sibling, so a
 * static server sends them as they are (nginx `brotli_static`/`gzip_static`,
 * Caddy `precompressed`) instead of compressing on each request or not at
 * all: the geometry core is 1.1 MB raw and under 400 KB compressed, and
 * `vite preview`'s own compression leaves `application/wasm` out. The preview
 * server serves the siblings the same way, so what it measures is what a
 * deployment sends. Node's zlib, no dependency.
 */
function kentosCompress() {
  let outDir = 'dist';
  const walk = (dir) => readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? walk(join(dir, e.name)) : [join(dir, e.name)]));
  return {
    name: 'kentos-compress',
    configResolved: (c) => {
      outDir = resolve(c.root, c.build.outDir);
    },
    closeBundle: {
      order: 'post',
      handler() {
        if (!existsSync(outDir)) return;
        for (const file of walk(outDir)) {
          if (!(extname(file) in COMPRESSED)) continue;
          const raw = readFileSync(file);
          if (raw.length < 1024) continue;
          writeFileSync(`${file}.br`, brotliCompressSync(raw, { params: { [zc.BROTLI_PARAM_QUALITY]: 11, [zc.BROTLI_PARAM_SIZE_HINT]: raw.length } }));
          writeFileSync(`${file}.gz`, gzipSync(raw, { level: 9 }));
        }
      },
    },
    configurePreviewServer: (server) => {
      const root = resolve(server.config.root, server.config.build.outDir);
      server.middlewares.use((req, res, next) => {
        const path = decodeURIComponent((req.url ?? '/').split('?')[0]);
        const type = COMPRESSED[extname(path)];
        if (!type || (req.method !== 'GET' && req.method !== 'HEAD')) return next();
        const file = join(root, path);
        if (!file.startsWith(root)) return next();
        const accept = String(req.headers['accept-encoding'] ?? '');
        const pick = [['br', '.br'], ['gzip', '.gz']].find(([enc, ext]) => accept.includes(enc) && existsSync(file + ext));
        if (!pick) return next();
        const body = file + pick[1];
        res.writeHead(200, {
          'content-type': type,
          'content-encoding': pick[0],
          'content-length': statSync(body).size,
          vary: 'Accept-Encoding',
          'cache-control': path.startsWith('/assets/') ? 'public, max-age=31536000, immutable' : 'no-cache',
        });
        res.end(req.method === 'HEAD' ? undefined : readFileSync(body));
      });
    },
  };
}

export default defineConfig({
  plugins: [kentosApi(), kentosCompress()],
  worker: { format: 'es' },
  // Agents' git worktrees live under .claude/worktrees: neither watched nor tested from here.
  server: { watch: { ignored: ['**/.claude/**'] } },
  test: {
    setupFiles: ['src/wasm/testSetup.ts', 'src/style/svg/testSetup.ts'],
    exclude: ['**/node_modules/**', '**/dist/**', '**/.claude/**'],
    // Vitest empties stylesheets; these two are read as text by app/appearance.test.ts.
    css: { include: [/styles\/(accents|fonts)\.css/] },
  },
});
