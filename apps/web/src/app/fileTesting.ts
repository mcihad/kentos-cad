import { v1IdentitiesInProcess, kcadInProcess } from '../io/testFormats';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import type { OpeningView } from '../ui/io/OpeningDialog';
import type { AppContext } from './context';
import { DocumentFiles, type DiscardChoice, type DrawingFileHandle, type DrawingFilePicker } from './fileIO';

/**
 * What the file tests share (app/fileIO*.test.ts, app/recovery.test.ts): an
 * in-memory file, a cloud project as the file service sees it, and the file
 * service over a document with the formats module in process. Test code only.
 */

/**
 * An in-memory file holding bytes; `fail` makes its writer throw on close,
 * `during` runs while it is being written.
 */
export function memoryFile(name: string, opts: { data?: string | Uint8Array; fail?: boolean; during?: () => void } = {}) {
  const encode = (d: string | Uint8Array) => (typeof d === 'string' ? new TextEncoder().encode(d) : new Uint8Array(d));
  const file = {
    name,
    bytes: encode(opts.data ?? ''),
    getFile: async () => new Blob([file.bytes]),
    createWritable: async () => {
      const parts: Uint8Array[] = [];
      return {
        write: async (data: string | Uint8Array) => {
          parts.push(encode(data));
          opts.during?.();
        },
        close: async () => {
          if (opts.fail) throw new Error('disk dolu');
          const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
          let at = 0;
          for (const p of parts) (out.set(p, at), (at += p.length));
          file.bytes = out;
        },
      };
    },
  };
  return file satisfies DrawingFileHandle;
}

/** A cloud project as the file service sees it: open or not, autosaving or not, and how many changes stay unsent. */
export function fakeCloud(open: { name: string; canWrite: boolean; unsent: number } | null) {
  const cloud = {
    project: { value: open ? { name: open.name, canWrite: open.canWrite } : null },
    sync: { value: null as { state: { value: string } } | null },
    left: 0,
    autosaves: () => !!cloud.project.value?.canWrite,
    leave: async () => {
      cloud.left++;
      cloud.project.value = null;
      return open?.unsent ?? 0;
    },
    detach: () => {
      cloud.project.value = null;
    },
  };
  return cloud;
}

/** An open's window that records what it was told. */
export function recordingView(log: string[], onStep?: (text: string) => void): OpeningView {
  return {
    step: (text) => (log.push(`step: ${text}`), onStep?.(text)),
    project: (text) => log.push(`project: ${text}`),
    close: () => log.push('close'),
  };
}

export function setup(doc = new CadDocument({ name: 'Proje', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 } }), cloud = fakeCloud(null)) {
  const messages: string[] = [];
  const say = (kind: string) => (text: string) => messages.push(`${kind}: ${text}`);
  const fitted: unknown[] = [];
  const ctx = {
    doc,
    log: { success: say('ok'), warn: say('uyarı'), error: say('hata'), info: say('bilgi') },
    tools: { activate: () => {} },
    selection: { clear: () => {} },
    view: { camera: { fit: (b: unknown) => fitted.push(b) }, zoomExtents: () => {} },
    cloud,
  } as unknown as AppContext;
  const files = new DocumentFiles(ctx);
  // The formats module in this process instead of its worker.
  files.identities = v1IdentitiesInProcess;
  files.kcad = kcadInProcess;
  // Every question is recorded and answered with the next choice (none left: the test did not expect one).
  const asked: string[] = [];
  const answers: DiscardChoice[] = [];
  files.ask = async (_name, after) => {
    asked.push(after);
    const next = answers.shift();
    if (!next) throw new Error(`beklenmeyen soru: ${after}`);
    return next;
  };
  return { ctx, doc, files, messages, asked, answers, cloud, fitted };
}

/** A picker that hands out these files and records the names Save as suggested. */
export const pick = (save: DrawingFileHandle | null, open: DrawingFileHandle | null = null, suggested: string[] = []): DrawingFilePicker => ({
  save: async (name) => (suggested.push(name), save),
  open: async () => open,
});
