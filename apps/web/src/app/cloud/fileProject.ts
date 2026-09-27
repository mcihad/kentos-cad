import { Signal } from '../../core/signal';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FileCommitted } from '../../contracts/generated/FileCommitted';
import type { CadDocument } from '../../model/document';
import { encodeDrawing, type DrawingCodec } from '../drawingFile';
import { ApiFailure, type CloudApi } from './api';
import type { AccessOutcome } from './sync';
import {
  cellState,
  newestOf,
  opened,
  readEvents,
  resyncStep,
  saveStep,
  step,
  type FileSaveState,
  type NewerRevision,
  type ProjectAnswer,
  type RevisionConflict,
  type RevisionInput,
  type RevisionState,
} from './fileRevisionsPlan';
import { commitEnvelope, fileConflict, sha256Hex, uploadBytes, uploadGone } from './transfer';
import { again } from './upload';

export type { FileSaveState, NewerRevision } from './fileRevisionsPlan';

/**
 * Saving an open file project (docs/adr/0031, 0038; TODOS.md SYNC-02,
 * SYNC-04, SYNC-06): the drawing is written as a new `.kcad` revision only
 * when the user saves (Kaydet), never by itself. A save goes through stages
 * the status bar shows apart, and nothing is called saved before the server
 * committed the revision:
 *
 * `encoding` (the drawing of that moment to KCAD v2 bytes, in the formats
 * worker) → `uploading` (with how far) → `verifying` (the server checks the
 * bytes and commits them on the revision the drawing is based on) → `saved`
 * (revision N). Only the revision that was written is marked saved: an edit
 * made meanwhile stays unsaved (CLAUDE.md §4.8).
 *
 * - A connection cut while the bytes go: the same upload is sent again. A
 *   commit whose answer was lost goes again with the same idempotency key
 *   (the server answers it from its log), so it never meets itself as a
 *   conflict; one that still has no answer waits for the next Kaydet.
 * - Someone else saved first (`@file` conflict): nothing is written and the
 *   drawing stays as it is; the user chooses (a separate copy, a local file,
 *   or the newest revision). Two files are never merged byte by byte.
 * - Someone else saved while the project is open (a `project.file` event):
 *   the server is asked which revision, by whom and when; it is said, and
 *   the newest revision is offered; nothing is reloaded by itself
 *   (`newer`). A Kaydet over it uploads nothing: it would be refused, so
 *   the user chooses at once.
 * - The events missed cannot be replayed (a resync): the project and its
 *   newest revision are asked; the drawing is never replaced (`resync`).
 * - The project deleted, archived or out of reach, or the role lowered:
 *   nothing more is saved there; the drawing stays on screen, and the local
 *   recovery copy keeps its unsaved work (app/recovery.ts).
 *
 * What the state is and what each of these does to it is
 * fileRevisionsPlan.ts's (`step`, `cellState`, `saveStep`); this class does
 * the asking, the uploading and the committing.
 */

/** What one Kaydet did. */
export type SaveOutcome = 'saved' | 'unchanged' | 'conflict' | 'failed' | 'readonly' | 'ended';

export interface FileProjectOptions {
  doc: CadDocument;
  api: CloudApi;
  tenantId: string;
  projectId: string;
  /** The revision the drawing on screen came from ("0" for a project without one yet). */
  base: string;
  /** The event cursor at the moment the project was read. */
  cursor: string;
  canWrite: boolean;
  /** The KCAD v2 codec (the formats worker; tests run it in process). */
  codec: () => Promise<DrawingCodec>;
  warn: (text: string) => void;
  /** Someone else saved a newer revision while the project is open (said once per revision). */
  onNewer?: (newer: NewerRevision) => void;
  onDeleted?: () => void;
  onRevoked?: (reason: string) => void;
  onArchived?: () => void;
  onAccessChanged?: () => void;
  /** Waits before each next try of a request whose answer did not come (tests pass short ones). */
  waits?: readonly number[];
  /** The part size of a large file's upload (docs/adr/0045; the session's, tests set a small one). */
  part?: () => number | undefined;
}

/** A commit on its way, or one whose answer was lost: sent again unchanged. */
interface Inflight {
  envelope: CommandEnvelope;
  /** The drawing's revision the uploaded bytes hold. */
  revision: number;
}

/** How long a save waits for an open edit to end before it takes the drawing. */
const IDLE_MS = 100;

export class FileProjectSave {
  /** The save cell's state (fileRevisionsPlan.ts `cellState`). */
  readonly state = new Signal<FileSaveState>('saved');
  /** How far the upload is (0–1), while `uploading`. */
  readonly progress = new Signal(0);
  readonly error = new Signal('');
  /** The revision the drawing on screen is based on ("0": none yet). */
  readonly base: Signal<string>;
  /** The revision this window saved last: its number, when, and its bytes' hash and size. */
  readonly lastSaved = new Signal<{ revision: string; at: number; sha256: string; size: number } | null>(null);
  /** A newer revision than `base` that someone else saved, while the drawing is open. */
  readonly newer = new Signal<NewerRevision | null>(null);
  /** The revisions a refused save met: the drawing's base and the server's newest. */
  readonly conflict = new Signal<RevisionConflict | null>(null);
  /** Unsaved changes (0 or 1): what an access change says is not saved. */
  readonly pending = new Signal(0);
  private readonly o: FileProjectOptions;
  /** What the project knows; the signals above are published from it. */
  private s: RevisionState;
  private saving: Promise<SaveOutcome> | null = null;
  private inflight: Inflight | null = null;
  /** Request ids of this window's commits: their events are its own. */
  private readonly own = new Set<string>();
  private disposed = false;
  private readonly unsubscribe: (() => void)[] = [];
  /** The newest event cursor heard. */
  cursor: string;

  constructor(o: FileProjectOptions) {
    this.o = o;
    this.s = opened(o.base, { dirty: o.doc.dirty.value, writable: o.canWrite });
    this.base = new Signal(o.base);
    this.cursor = o.cursor;
    this.publish();
    this.unsubscribe.push(
      o.doc.dirty.subscribe((dirty) => {
        // Edited after a save, or clean again after undoing to the saved state; `pending` after, so what it wakes reads the new state.
        this.apply({ kind: 'dirty', dirty });
        this.pending.set(dirty ? 1 : 0);
      }, true),
    );
  }

  /** What the project knows now (fileRevisionsPlan.ts): what the questions read. */
  get revisions(): RevisionState {
    return this.s;
  }

  /** One change of what the project knows, published to the signals; true when a newer revision is to be said. */
  private apply(i: RevisionInput): boolean {
    const { state, say } = step(this.s, i);
    this.s = state;
    this.publish();
    return say;
  }

  private publish(): void {
    const s = this.s;
    this.base.set(s.base);
    this.newer.set(s.newer);
    this.conflict.set(s.conflict);
    this.state.set(cellState(s));
  }

  /** Stops for good (the project is left): an answer still on its way changes nothing any more. */
  dispose(): void {
    this.disposed = true;
    for (const u of this.unsubscribe) u();
  }

  /** Whether a save is under way (a second Kaydet waits for it). */
  get busy(): boolean {
    return !!this.saving;
  }

  /** Why nothing is saved there any more, if so. */
  get endedBy(): 'deleted' | 'revoked' | 'archived' | null {
    return this.s.ended;
  }

  /** Saves the drawing as a new revision; one save at a time (a second Kaydet gets the first one's outcome). */
  save(): Promise<SaveOutcome> {
    if (this.saving) return this.saving;
    this.saving = this.run().finally(() => {
      this.saving = null;
    });
    return this.saving;
  }

  private async run(): Promise<SaveOutcome> {
    const { o } = this;
    if (this.disposed) return 'failed';
    // Where nothing is saved, and while a conflict stands (the user chooses first: a save now would meet the same).
    const first = saveStep(this.s);
    if (first === 'ended' || first === 'readonly' || first === 'conflict') return first;
    await this.whenIdle();
    if (this.disposed) return 'failed';
    try {
      // A commit whose answer was lost goes first, unchanged: the server answers its key from its log.
      if (this.inflight) {
        const r = await this.commit(this.inflight);
        if (r !== 'saved' || !o.doc.dirty.value) return r;
      }
      switch (saveStep(this.s)) {
        case 'ended':
          return 'ended';
        case 'readonly':
          return 'readonly';
        case 'conflict':
          return 'conflict';
        case 'unchanged':
          this.apply({ kind: 'unchanged' });
          return 'unchanged';
        case 'behind':
          // A newer revision is known: the server would refuse this one, so nothing is uploaded; the user chooses.
          this.apply({ kind: 'refused', actual: this.s.newer!.revision });
          return 'conflict';
        case 'save':
          break;
      }
      this.error.set('');
      this.apply({ kind: 'stage', stage: 'encoding' });
      const encoded = await encodeDrawing(o.doc, o.codec);
      if (this.disposed) return 'failed';
      if (encoded.dropped) o.warn(`KCAD v2'nin tanımadığı alanlar dosyaya yazılmadı: ${encoded.dropped}.`);
      const sha256 = await sha256Hex(encoded.bytes);
      this.progress.set(0);
      this.apply({ kind: 'stage', stage: 'uploading' });
      const target = { tenantId: o.tenantId, projectId: o.projectId };
      const upload = await uploadBytes(o.api, target, encoded.bytes, sha256, {
        progress: (done, total) => this.progress.set(total ? Math.min(1, done / total) : 0),
        verifying: () => void (!this.disposed && this.apply({ kind: 'stage', stage: 'verifying' })),
        waits: o.waits,
        part: o.part?.(),
      });
      if (this.disposed) return 'failed';
      this.inflight = { envelope: commitEnvelope(target, upload.id, this.base.value), revision: encoded.revision };
      return await this.commit(this.inflight);
    } catch (e) {
      return this.failed(e);
    }
  }

  /** Commits the uploaded revision on the drawing's base. */
  private async commit(f: Inflight): Promise<SaveOutcome> {
    this.apply({ kind: 'stage', stage: 'verifying' });
    this.own.add(f.envelope.requestId);
    let done: FileCommitted;
    try {
      done = await again(() => this.o.api.lifecycle<FileCommitted>(f.envelope), this.o.waits);
    } catch (e) {
      const c = fileConflict(e);
      if (c) {
        this.inflight = null;
        this.apply({ kind: 'refused', actual: c.actual });
        // The refusal names only the number: who saved it and when are asked (the question may already be up).
        void this.askNewest();
        return 'conflict';
      }
      // Refused for good: it is not sent again. Unanswered: it is, with its key, at the next Kaydet.
      if (!(e instanceof ApiFailure) || !e.transient) this.inflight = null;
      throw e;
    }
    if (this.disposed) return 'failed';
    this.inflight = null;
    this.lastSaved.set({ revision: done.revision, at: Date.now(), sha256: done.sha256, size: done.size });
    // Only the revision written is saved: an edit made meanwhile stays unsaved (CLAUDE.md §4.8).
    this.o.doc.markSaved(f.revision);
    this.error.set('');
    this.apply({ kind: 'committed', revision: done.revision, dirty: this.o.doc.dirty.value });
    return 'saved';
  }

  /** A save that failed: the drawing stays as it is; the state and the message say why and what to do. */
  private failed(e: unknown): SaveOutcome {
    if (this.disposed) return 'failed';
    const f = e instanceof ApiFailure ? e : null;
    if (f?.deleted) {
      this.markDeleted();
      return 'ended';
    }
    if (f?.archived) {
      this.markArchived();
      return 'ended';
    }
    if (f?.notFound && !uploadGone(f)) {
      this.markRevoked();
      return 'ended';
    }
    if (f?.code === 'forbidden') this.o.onAccessChanged?.();
    const why = e instanceof Error ? e.message : String(e);
    const text = f?.transient
      ? `Dosya kaydedilemedi: sunucuya ulaşılamadı (${why}). Çizim olduğu gibi duruyor; bağlantı dönünce yeniden Kaydet'e basın.`
      : `Dosya kaydedilemedi: ${why} Çizim olduğu gibi duruyor.`;
    this.error.set(text);
    this.apply({ kind: 'failed' });
    this.o.warn(text);
    return 'failed';
  }

  /** Resolves once no edit or group is open in the drawing (a save takes a whole drawing). */
  private whenIdle(): Promise<void> {
    return new Promise((resolve) => {
      const check = () => (this.o.doc.busy && !this.disposed ? setTimeout(check, IDLE_MS) : resolve());
      check();
    });
  }

  // ── Events ─────────────────────────────────────────────────────────────

  /** Events of the project (from the socket, in order): another's revision, a deletion, an archive, an access change. */
  receive(events: readonly EventRecord[]): void {
    if (this.disposed || this.s.ended) return;
    const read = readEvents(events, (id) => this.own.has(id));
    if (read.cursor !== null) this.cursor = read.cursor;
    if (read.end?.why === 'deleted') return this.markDeleted();
    if (read.end?.why === 'archived') return this.markArchived(read.end.quiet);
    if (read.access) this.o.onAccessChanged?.();
    if (read.newest) void this.askNewest();
  }

  /** Asks the server which revision is its newest, who saved it and when: said once when newer, never loaded by itself. */
  private async askNewest(): Promise<void> {
    let revs;
    try {
      revs = await this.o.api.fileRevisions(this.o.tenantId, this.o.projectId);
    } catch {
      return;
    }
    if (this.disposed) return;
    if (this.apply({ kind: 'newest', newest: newestOf(revs) }) && this.s.newer) this.o.onNewer?.(this.s.newer);
  }

  /**
   * The server keeps no events from this window's cursor any more (or the
   * cursor is beyond its newest): the project is asked what became of it,
   * then its newest revision; events are followed from its cursor now. The
   * drawing is never replaced. Answers what to do next: `follow` (subscribe
   * again), `retry` (later: no answer came), or `ended`.
   */
  async resync(): Promise<'follow' | 'retry' | 'ended'> {
    if (this.disposed || this.s.ended) return 'ended';
    const next = resyncStep(await this.askProject());
    if (this.disposed || this.s.ended) return 'ended';
    switch (next.kind) {
      case 'end':
        if (next.why === 'deleted') this.markDeleted();
        else if (next.why === 'archived') this.markArchived();
        else this.markRevoked(next.reason);
        return 'ended';
      case 'retry':
        return 'retry';
      case 'follow':
        this.cursor = next.cursor;
        // The events missed may have changed the access, or brought a revision: asked as those events would.
        this.o.onAccessChanged?.();
        await this.askNewest();
        return this.disposed || this.s.ended ? 'ended' : 'follow';
    }
  }

  /** The project as the server has it now, for a resync; passing failures are tried again first. */
  private async askProject(): Promise<ProjectAnswer> {
    try {
      const info = await again(() => this.o.api.project(this.o.tenantId, this.o.projectId), this.o.waits);
      return { kind: 'project', state: info.state, eventCursor: info.eventCursor };
    } catch (e) {
      if (!(e instanceof ApiFailure)) return { kind: 'failed', message: e instanceof Error ? e.message : String(e) };
      if (e.deleted) return { kind: 'deleted' };
      if (e.notFound) return { kind: 'notFound' };
      if (e.code === 'forbidden') return { kind: 'forbidden', message: e.message };
      return e.transient ? { kind: 'unreachable' } : { kind: 'failed', message: e.message };
    }
  }

  // ── Deletion and access ────────────────────────────────────────────────

  private end(why: 'deleted' | 'revoked' | 'archived'): boolean {
    if (this.s.ended || this.disposed) return false;
    this.apply({ kind: 'ended', why });
    return true;
  }

  markDeleted(): void {
    if (this.end('deleted')) this.o.onDeleted?.();
  }

  markRevoked(reason = ''): void {
    if (this.end('revoked')) this.o.onRevoked?.(reason);
  }

  /** Archived (docs/adr/0028): read-only until unarchived and opened again; `quiet` when this window did it. */
  markArchived(quiet = false): void {
    if (this.end('archived') && !quiet) this.o.onArchived?.();
  }

  /** A request this window sends outside Kaydet (a lifecycle command): its event is its own. */
  expect(requestId: string): void {
    this.own.add(requestId);
  }

  /** What this account may do changed while the project is open: without the right to write, Kaydet is refused. */
  setAccess(canWrite: boolean): AccessOutcome {
    if (this.s.ended || this.disposed || canWrite === this.s.writable) return 'same';
    this.apply({ kind: 'access', writable: canWrite });
    return canWrite ? 'resumed' : 'held';
  }
}
