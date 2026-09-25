import { initSvgCoreFrom } from './core';

/**
 * Vitest setup (vite.config.mjs): starts the SVG editor's Rust package
 * from its build before any test runs, as the editor does when it opens.
 * `pnpm test` builds the package first when its sources changed
 * (scripts/wasm/ensure.mjs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');
initSvgCoreFrom(new WebAssembly.Module(fs.readFileSync(new URL('./pkg/kentos_svg_wasm_bg.wasm', import.meta.url))));
