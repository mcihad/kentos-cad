// Vite configuration. Workers are ES modules (the processing worker starts
// its own copy of the Rust geometry core, src/wasm/core.ts); tests start the
// core first (src/wasm/testSetup.ts). Requests under /v1/
// go to the local KentOS API (apps/api, `pnpm api`), in both `vite` and
// `vite preview`, and so does the project WebSocket (/v1/ws). When the API is
// not running the answer is a quiet 503, so the app shows "Sunucu: yok"
// without filling the terminal with proxy errors (Vite's own proxy logs every
// refused connection).
import http from 'node:http';
import net from 'node:net';
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

export default defineConfig({
  plugins: [kentosApi()],
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
