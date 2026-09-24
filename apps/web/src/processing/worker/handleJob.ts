import { EntitySnapshot, runJob } from '../job';
import type { Feedback, ProcessingTool } from '../types';
import type { WorkerReply, WorkerRequest } from './protocol';

/** Least time between two progress messages (the page redraws at most this often). */
const PROGRESS_MS = 50;

/**
 * Runs one job inside the worker: rebuild the document from the copied
 * objects and run it as the page does (`runJob`: expressions compiled, the
 * objects it reads in a geometry store of the worker's own), posting
 * progress, log lines and the result. Kept apart from the worker entry so
 * tests can drive it without a Worker.
 */
export async function handleJob(msg: WorkerRequest, post: (reply: WorkerReply) => void, lookup: (id: string) => ProcessingTool | undefined): Promise<void> {
  const { id, job } = msg;
  const tool = lookup(job.toolId);
  if (!tool) {
    post({ type: 'error', id, message: `“${job.toolId}” arka planda çalıştırılamıyor: bu araç arka plan çalıştırıcısında yok.` });
    return;
  }
  const doc = new EntitySnapshot(msg.entities);
  let last = 0;
  const feedback: Feedback = {
    progress: (fraction, label) => {
      const now = Date.now();
      if (now - last < PROGRESS_MS && fraction < 1) return;
      last = now;
      post({ type: 'progress', id, fraction, label: label ?? '' });
    },
    info: (message) => post({ type: 'log', id, level: 'info', message }),
    warn: (message) => post({ type: 'log', id, level: 'warn', message }),
    // The page stops a worker job by terminating the worker; nothing to poll here.
    canceled: false,
    yield: () => Promise.resolve(),
  };
  try {
    const result = await runJob(tool, job, doc, feedback);
    post({ type: 'done', id, result });
  } catch (err) {
    post({ type: 'error', id, message: (err as Error).message ?? String(err) });
  }
}
