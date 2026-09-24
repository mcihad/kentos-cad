import { coreModule } from '../../wasm/core';
import type { Executor, RunJob } from '../job';
import type { DocumentSnapshot, Feedback, ProcessingTool, RunResult } from '../types';
import type { WorkerReply, WorkerRequest } from './protocol';

/** The part of a Worker the executor uses (a fake in tests). */
export interface WorkerLike {
  postMessage(message: WorkerRequest): void;
  terminate(): void;
  onmessage: ((e: { data: WorkerReply }) => void) | null;
  onerror: ((e: unknown) => void) | null;
}

interface Pending {
  resolve(r: RunResult): void;
  reject(e: Error): void;
  feedback: Feedback;
}

/** How often a running job checks whether the user pressed Durdur. */
const CANCEL_POLL_MS = 50;

/**
 * Runs tools in a Web Worker so long jobs leave the page responsive. The
 * worker is started on first use and kept; the document goes over as a
 * copy of its objects, the ChangeSet comes back and the runner applies it
 * as usual. Durdur terminates the worker (a tool in a tight loop cannot
 * be asked nicely) and the next job starts a fresh one.
 */
export function workerExecutor(spawn: () => WorkerLike, tools: ReadonlySet<string>): Executor {
  let worker: WorkerLike | null = null;
  /** Whether the worker still needs the geometry core (it comes with the first job). */
  let fresh = false;
  let seq = 0;
  const pending = new Map<number, Pending>();

  const failAll = (message: string) => {
    for (const p of pending.values()) p.reject(new Error(message));
    pending.clear();
  };
  const stop = () => {
    worker?.terminate();
    worker = null;
  };
  const ensure = (): WorkerLike => {
    if (worker) return worker;
    const w = spawn();
    w.onmessage = (e) => {
      const m = e.data;
      const p = pending.get(m.id);
      if (!p) return;
      if (m.type === 'progress') p.feedback.progress(m.fraction, m.label);
      else if (m.type === 'log') (m.level === 'warn' ? p.feedback.warn : p.feedback.info)(m.message);
      else {
        pending.delete(m.id);
        if (m.type === 'done') p.resolve(m.result);
        else p.reject(new Error(m.message));
      }
    };
    w.onerror = () => {
      stop();
      failAll('Arka plan çalıştırıcısı beklenmedik biçimde durdu.');
    };
    worker = w;
    fresh = true;
    return w;
  };

  return {
    target: 'worker',
    available: () => true,
    supports: (tool: ProcessingTool) => tools.has(tool.id),
    execute: (_tool: ProcessingTool, job: RunJob, doc: DocumentSnapshot, feedback: Feedback) =>
      new Promise<RunResult>((resolve, reject) => {
        const id = ++seq;
        const timer = setInterval(() => {
          if (!feedback.canceled || !pending.has(id)) return;
          pending.delete(id);
          clearInterval(timer);
          stop();
          failAll('Arka plan çalıştırıcısı durduruldu.');
          resolve({});
        }, CANCEL_POLL_MS);
        const done = <T>(fn: (v: T) => void) => (v: T) => {
          clearInterval(timer);
          fn(v);
        };
        pending.set(id, { resolve: done(resolve), reject: done(reject), feedback });
        try {
          const w = ensure();
          const core = fresh ? (coreModule() ?? undefined) : undefined;
          w.postMessage({ type: 'run', id, job, entities: [...doc.all()], core });
          fresh = false;
        } catch (err) {
          pending.delete(id);
          clearInterval(timer);
          reject(new Error(`Arka plana gönderilemedi: ${(err as Error).message}`));
        }
      }),
  };
}
