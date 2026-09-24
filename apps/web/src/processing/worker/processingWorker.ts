import { initCoreFrom } from '../../wasm/core';
import { BUILTIN_TOOLS } from '../builtin';
import { handleJob } from './handleJob';
import type { WorkerReply, WorkerRequest } from './protocol';

/**
 * Entry of the processing Web Worker (started by app/processing.ts). It
 * carries the built-in tools; everything else is in handleJob.
 */
const tools = new Map(BUILTIN_TOOLS.map((t) => [t.id, t]));
const scope = self as unknown as { onmessage: ((e: MessageEvent<WorkerRequest>) => void) | null; postMessage(m: WorkerReply): void };

scope.onmessage = (e) => {
  if (e.data.type !== 'run') return;
  // The page's compiled geometry core comes with the first job; this worker starts its own copy of it.
  if (e.data.core) initCoreFrom(e.data.core);
  void handleJob(e.data, (reply) => scope.postMessage(reply), (id) => tools.get(id));
};
