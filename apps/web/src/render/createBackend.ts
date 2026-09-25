import type { BackendKind, RenderBackend } from './types';
import { WebGL2Backend } from './webgl2/WebGL2Backend';
import { webgpuSupported } from './webgpu/support';

/** WebGL2 is the default and comes with the app; WebGPU is loaded when it is chosen (CLAUDE.md §20). */
const factories: Record<BackendKind, () => Promise<RenderBackend>> = {
  webgpu: () => import('./webgpu/WebGPUBackend').then((m) => new m.WebGPUBackend()),
  webgl2: async () => new WebGL2Backend(),
};

/**
 * Tries the preferred backend first and falls back in order. Each attempt
 * gets a fresh canvas because a canvas cannot switch context types.
 */
export async function createBackend(
  host: HTMLElement,
  preferred: BackendKind[] = ['webgl2'],
  opts: { antialias?: boolean } = {},
): Promise<{ backend: RenderBackend; canvas: HTMLCanvasElement; errors: string[] }> {
  const errors: string[] = [];
  // WebGPU is only attempted where the browser exposes it; WebGL2 is the floor.
  const order = [...new Set<BackendKind>([...preferred, 'webgl2'])].filter((k) => k !== 'webgpu' || webgpuSupported());
  for (const kind of order) {
    const canvas = document.createElement('canvas');
    canvas.className = 'viewport__gl';
    let backend: RenderBackend;
    try {
      backend = await factories[kind]();
    } catch (err) {
      errors.push(`${kind}: yüklenemedi (${(err as Error).message})`);
      continue;
    }
    try {
      await backend.init(canvas, opts);
      host.prepend(canvas);
      return { backend, canvas, errors };
    } catch (err) {
      errors.push(`${kind}: ${(err as Error).message}`);
      backend.dispose();
    }
  }
  throw new Error(`Hiçbir çizim arka ucu başlatılamadı (${errors.join('; ')})`);
}
