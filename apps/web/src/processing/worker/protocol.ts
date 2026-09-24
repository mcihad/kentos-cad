import type { Entity } from '../../model/entities';
import type { RunJob } from '../job';
import type { RunResult } from '../types';

/**
 * Messages between the page and the processing worker. Everything here is
 * structured-clone data: objects, job values, results. One job runs at a
 * time per worker; ids pair replies with requests.
 */

/** `core`: the compiled geometry core, sent with a new worker's first job (src/wasm/core.ts). */
export type WorkerRequest = { type: 'run'; id: number; job: RunJob; entities: readonly Entity[]; core?: WebAssembly.Module };

export type WorkerReply =
  | { type: 'progress'; id: number; fraction: number; label: string }
  | { type: 'log'; id: number; level: 'info' | 'warn'; message: string }
  | { type: 'done'; id: number; result: RunResult }
  | { type: 'error'; id: number; message: string };
