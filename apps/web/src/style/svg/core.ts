import { readResult, writeArgs } from '../../wasm/core';
import { callOp, initSync, opId } from './pkg/kentos_svg_wasm.js';
import wasmUrl from './pkg/kentos_svg_wasm_bg.wasm?url';

/**
 * The SVG editor's geometry in Rust (crates/shared/svg-core, docs/adr/0008
 * “SVG düzenleyicisi”): a WASM package of its own, fetched and compiled
 * when the editor opens (CLAUDE.md §20), so the app's start does not pay
 * for it. After `initSvgCore` every call is synchronous, as the editor's
 * pointer handlers need. Calls cross as JSON, as in the geometry core
 * (`svgOp(name)`, NaN and ±∞ kept as "#NaN" / "#Inf").
 */

let started: Promise<void> | null = null;
let ready = false;

/** Fetches, compiles and starts the package (once; the editor awaits it before it opens). */
export function initSvgCore(): Promise<void> {
  started ??= (async () => {
    const res = await fetch(wasmUrl);
    if (!res.ok) throw new Error(`SVG düzenleyicisinin çekirdeği indirilemedi (${res.status}).`);
    // compileStreaming needs the application/wasm type; a server that sends another falls back to bytes.
    const module = await WebAssembly.compileStreaming(res.clone()).catch(async () => WebAssembly.compile(await res.arrayBuffer()));
    initSvgCoreFrom(module);
  })().catch((err: unknown) => {
    // A failed start can be tried again (the next time the editor opens).
    started = null;
    throw err;
  });
  return started;
}

/** Starts the package from a compiled module (the tests). */
export function initSvgCoreFrom(module: WebAssembly.Module): void {
  if (ready) return;
  initSync({ module });
  ready = true;
}

/** A caller for the SVG core's operation `name`. */
export function svgOp<F extends (...args: never[]) => unknown>(name: string): F {
  let id = -1;
  const call = (...args: unknown[]): unknown => {
    if (id < 0) {
      if (!ready) throw new Error('SVG düzenleyicisinin çekirdeği henüz başlatılmadı.');
      id = opId(name);
      if (id < 0) throw new Error(`SVG çekirdeğinde “${name}” işlemi yok; WASM paketi eski olabilir (pnpm wasm).`);
    }
    return readResult(callOp(id, writeArgs(args)));
  };
  return call as unknown as F;
}
