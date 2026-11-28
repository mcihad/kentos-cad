// The cloud portal's Vite configuration (docs/adr/9000 §8). Requests under /v1/ go to the KentOS API
// (KENTOS_API_PORT, 8787 by default) in both `vite` and `vite preview`; when it is not running the answer is a quiet
// 503 and the portal says the server cannot be reached.
import http from 'node:http';
import { defineConfig } from 'vite';

const API_PORT = Number(process.env.KENTOS_API_PORT ?? 8787);

/** Forwards /v1/* to 127.0.0.1:KENTOS_API_PORT. */
function kentosApi() {
  const forward = (req, res, next) => {
    if (!req.url?.startsWith('/v1/')) return next();
    const upstream = http.request(
      { host: '127.0.0.1', port: API_PORT, path: req.url, method: req.method, headers: { ...req.headers, host: `127.0.0.1:${API_PORT}` } },
      (r) => {
        res.writeHead(r.statusCode ?? 502, r.headers);
        r.pipe(res);
      },
    );
    upstream.setTimeout(600000, () => upstream.destroy(new Error('timeout')));
    upstream.on('error', () => {
      if (!res.headersSent) res.writeHead(503, { 'content-type': 'application/json', 'cache-control': 'no-store' });
      res.end('{"error":"api-unreachable","message":"Sunucuya ulaşılamıyor."}');
    });
    req.pipe(upstream);
  };
  return {
    name: 'kentos-api',
    // Nothing returned: a function returned here would be taken for a hook to run after Vite's own middlewares.
    configureServer: (server) => {
      server.middlewares.use(forward);
    },
    configurePreviewServer: (server) => {
      server.middlewares.use(forward);
    },
  };
}

export default defineConfig({
  plugins: [kentosApi()],
  server: { port: Number(process.env.PORTAL_PORT ?? 5180), strictPort: true },
  preview: { port: Number(process.env.PORTAL_PORT ?? 5180), strictPort: true },
  build: { target: 'es2023', outDir: 'dist', emptyOutDir: true },
});
