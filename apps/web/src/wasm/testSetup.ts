import { initCoreFrom } from './core';

/**
 * Vitest setup (vite.config.mjs): starts the Rust geometry core from the
 * built package before any test runs, as src/main.ts does in the page.
 * `pnpm test` builds the package first when its sources changed
 * (scripts/wasm/ensure.mjs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
initCoreFrom(new WebAssembly.Module(fs.readFileSync(new URL('./pkg/kentos_geometry_wasm_bg.wasm', import.meta.url))));
