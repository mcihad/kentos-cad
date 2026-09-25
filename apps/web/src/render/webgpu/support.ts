/**
 * Whether the browser offers WebGPU, without loading the WebGPU backend: the backend is a chunk of its own
 * (CLAUDE.md §20), fetched when WebGPU is chosen, while menus and settings ask this at once.
 */
export function webgpuSupported(): boolean {
  return typeof navigator !== 'undefined' && 'gpu' in navigator;
}
