import type { BackendKind, RenderBackend } from './types';
import { WebGL2Backend } from './webgl2/WebGL2Backend';
import { WebGPUBackend } from './webgpu/WebGPUBackend';

const factories: Record<BackendKind, () => RenderBackend> = {
  webgpu: () => new WebGPUBackend(),
  webgl2: () => new WebGL2Backend(),
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
  const order = [...new Set<BackendKind>([...preferred, 'webgl2'])].filter((k) => k !== 'webgpu' || WebGPUBackend.isSupported());
  for (const kind of order) {
    const canvas = document.createElement('canvas');
    canvas.className = 'viewport__gl';
    const backend = factories[kind]();
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
