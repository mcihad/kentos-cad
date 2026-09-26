import { Signal } from '../../core/signal';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FileCommitted } from '../../contracts/generated/FileCommitted';
import type { CadDocument } from '../../model/document';
import { encodeDrawing, type DrawingCodec } from '../drawingFile';
import { ApiFailure, type CloudApi } from './api';
import type { AccessOutcome } from './sync';
import { PROJECT_ACCESS, PROJECT_ARCHIVED, PROJECT_DELETED } from './syncCore';
import { commitEnvelope, fileConflict, sha256Hex, uploadBytes, uploadGone } from './transfer';
import { again } from './upload';

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
 *   it is said, and the newest revision is offered; nothing is reloaded by
 *   itself (`newer`).
 * - The project deleted, archived or out of reach, or the role lowered:
 *   nothing more is saved there; the drawing stays on screen, and the local
 *   recovery copy keeps its unsaved work (app/recovery.ts).
 */

/**
 * `saved`: the drawing on screen is revision `base`. `pending`: it has
 * changes no revision holds. `outdated`: someone saved a newer revision.
 */
export type FileSaveState =
  | 'saved'
  | 'pending'
  | 'encoding'
  | 'uploading'
  | 'verifying'
  | 'conflict'
  | 'outdated'
  | 'error'
  | 'readonly'
  | 'deleted'
  | 'revoked'
  | 'archived';

/** What one Kaydet did. */
export type SaveOutcome = 'saved' | 'unchanged' | 'conflict' | 'failed' | 'readonly' | 'ended';

/** A newer revision on the server than the drawing is based on. */
export interface NewerRevision {
  revision: string;
  /** Who saved it, when the server says. */
  by: string;
}

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
  /** Someone else saved a newer revision while the project is open. */
  onNewer?: (newer: NewerRevision) => void;
  onDeleted?: () => void;
  onRevoked?: (reason: string) => void;
  onArchived?: () => void;
  onAccessChanged?: () => void;
  /** Waits before each next try of a request whose answer did not come (tests pass short ones). */
  waits?: readonly number[];
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
  readonly conflict = new Signal<{ expected: string; actual: string } | null>(null);
  /** Unsaved changes (0 or 1): what an access change says is not saved. */
  readonly pending = new Signal(0);
  private readonly o: FileProjectOptions;
  private saving: Promise<SaveOutcome> | null = null;
  private inflight: Inflight | null = null;
  /** Request ids of this window's commits: their events are its own. */
  private readonly own = new Set<string>();
  private ended: 'deleted' | 'revoked' | 'archived' | null = null;
  private writable: boolean;
  private disposed = false;
  private readonly unsubscribe: (() => void)[] = [];
  /** The newest event cursor heard. */
  cursor: string;

  constructor(o: FileProjectOptions) {
    this.o = o;
    this.base = new Signal(o.base);
    this.cursor = o.cursor;
    this.writable = o.canWrite;
    if (!this.writable) this.state.set('readonly');
    this.unsubscribe.push(
      o.doc.dirty.subscribe((dirty) => {
        this.pending.set(dirty ? 1 : 0);
        // Edited after a save, or clean again after undoing to the saved state.
        if (this.state.value === 'saved' && dirty) this.state.set('pending');
        else if (this.state.value === 'pending' && !dirty) this.state.set('saved');
      }, true),
    );
    if (o.doc.dirty.value && this.state.value === 'saved') this.state.set('pending');
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
    return this.ended;
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
    if (this.ended) return 'ended';
    if (!this.writable) {
      this.state.set('readonly');
      return 'readonly';
    }
    // The user chooses first (a copy, a local file or the newest revision): a save now would meet the same.
    if (this.conflict.value) return 'conflict';
    await this.whenIdle();
    if (this.disposed) return 'failed';
    try {
      // A commit whose answer was lost goes first, unchanged: the server answers its key from its log.
      if (this.inflight) {
        const r = await this.commit(this.inflight);
        if (r !== 'saved' || !o.doc.dirty.value) return r;
      }
      if (!o.doc.dirty.value && this.base.value !== '0') {
        this.state.set(this.newer.value ? 'outdated' : 'saved');
        return 'unchanged';
      }
      this.error.set('');
      this.state.set('encoding');
      const encoded = await encodeDrawing(o.doc, o.codec);
      if (this.disposed) return 'failed';
      if (encoded.dropped) o.warn(`KCAD v2'nin tanımadığı alanlar dosyaya yazılmadı: ${encoded.dropped}.`);
      const sha256 = await sha256Hex(encoded.bytes);
      this.progress.set(0);
      this.state.set('uploading');
      const target = { tenantId: o.tenantId, projectId: o.projectId };
      const upload = await uploadBytes(o.api, target, encoded.bytes, sha256, {
        progress: (done, total) => this.progress.set(total ? Math.min(1, done / total) : 0),
        verifying: () => !this.disposed && this.state.set('verifying'),
        waits: o.waits,
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
    this.state.set('verifying');
    this.own.add(f.envelope.requestId);
    let done: FileCommitted;
    try {
      done = await again(() => this.o.api.lifecycle<FileCommitted>(f.envelope), this.o.waits);
    } catch (e) {
      const c = fileConflict(e);
      if (c) {
        this.inflight = null;
        this.conflict.set(c);
        this.newer.set({ revision: c.actual, by: '' });
        this.state.set('conflict');
        return 'conflict';
      }
      // Refused for good: it is not sent again. Unanswered: it is, with its key, at the next Kaydet.
      if (!(e instanceof ApiFailure) || !e.transient) this.inflight = null;
      throw e;
    }
    if (this.disposed) return 'failed';
    this.inflight = null;
    this.base.set(done.revision);
    if (this.newer.value && Number(this.newer.value.revision) <= Number(done.revision)) this.newer.set(null);
    this.lastSaved.set({ revision: done.revision, at: Date.now(), sha256: done.sha256, size: done.size });
    // Only the revision written is saved: an edit made meanwhile stays unsaved (CLAUDE.md §4.8).
    this.o.doc.markSaved(f.revision);
    this.error.set('');
    this.state.set(this.o.doc.dirty.value ? 'pending' : this.newer.value ? 'outdated' : 'saved');
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
    this.state.set('error');
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

  /**
   * The conflict is settled (a copy was saved elsewhere, a local file was
   * written, or the newest revision replaced the drawing): the next Kaydet
   * goes on from `base`.
   */
  settle(base = this.base.value): void {
    this.conflict.set(null);
    this.base.set(base);
    if (this.newer.value && Number(this.newer.value.revision) <= Number(base)) this.newer.set(null);
    if (!this.ended && this.writable) this.state.set(this.o.doc.dirty.value ? 'pending' : this.newer.value ? 'outdated' : 'saved');
  }

  // ── Events ─────────────────────────────────────────────────────────────

  /** Events of the project (from the socket, in order): another's revision, a deletion, an archive, an access change. */
  receive(events: readonly EventRecord[]): void {
    if (this.disposed || this.ended) return;
    for (const e of events) {
      this.cursor = e.seq;
      const mine = !!e.requestId && this.own.has(e.requestId);
      if (e.kind === PROJECT_DELETED) return this.markDeleted();
      if (e.kind === PROJECT_ARCHIVED) return this.markArchived(mine);
      if (e.kind === PROJECT_ACCESS) this.o.onAccessChanged?.();
      if (e.kind === 'project.file' && !mine) void this.heardNewer();
    }
  }

  /** Someone else committed a revision: which one and who, from the server; said, never loaded by itself. */
  private async heardNewer(): Promise<void> {
    let revs;
    try {
      revs = await this.o.api.fileRevisions(this.o.tenantId, this.o.projectId);
    } catch {
      return;
    }
    if (this.disposed || this.ended || !revs.current || Number(revs.current) <= Number(this.base.value)) return;
    const newest = revs.revisions.find((r) => r.revision === revs.current);
    const newer = { revision: revs.current, by: newest?.createdByName ?? '' };
    if (this.newer.value?.revision === newer.revision) return;
    this.newer.set(newer);
    if (this.state.value === 'saved') this.state.set('outdated');
    this.o.onNewer?.(newer);
  }

  // ── Deletion and access ────────────────────────────────────────────────

  private end(why: 'deleted' | 'revoked' | 'archived'): boolean {
    if (this.ended || this.disposed) return false;
    this.ended = why;
    this.state.set(why);
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
    if (this.ended || this.disposed || canWrite === this.writable) return 'same';
    this.writable = canWrite;
    if (!canWrite) {
      this.state.set('readonly');
      return 'held';
    }
    this.state.set(this.conflict.value ? 'conflict' : this.o.doc.dirty.value ? 'pending' : 'saved');
    return 'resumed';
  }
}
