import type { Checkpoint } from '../../contracts/generated/Checkpoint';
import type { CheckpointChange } from '../../contracts/generated/CheckpointChange';
import type { CheckpointCreate } from '../../contracts/generated/CheckpointCreate';
import type { CheckpointDelete } from '../../contracts/generated/CheckpointDelete';
import type { CheckpointRestore } from '../../contracts/generated/CheckpointRestore';
import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { DocumentSnapshotV2 } from '../../contracts/generated/DocumentSnapshotV2';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { FileCommit } from '../../contracts/generated/FileCommit';
import type { FileCommitted } from '../../contracts/generated/FileCommitted';
import type { FileRevision } from '../../contracts/generated/FileRevision';
import type { FileRevisions } from '../../contracts/generated/FileRevisions';
import type { FileUpload } from '../../contracts/generated/FileUpload';
import type { FileUploadBegin } from '../../contracts/generated/FileUploadBegin';
import type { ProjectCheckpoints } from '../../contracts/generated/ProjectCheckpoints';
import type { ProjectDuplicated } from '../../contracts/generated/ProjectDuplicated';
import type { ProjectImport } from '../../contracts/generated/ProjectImport';
import type { ProjectImported } from '../../contracts/generated/ProjectImported';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { ApiFailure, type KcadDownload, type Transfer } from './api';
import { FILE_KEY, sha256Hex } from './transfer';

/**
 * The server's file side for the cloud tests (never used by the app):
 * uploads (declared size and SHA-256, bytes checked, a cut-off body sent
 * again), a file project's revisions and `project.file.commit` on the
 * revision a file was based on (409 `@file`), a database project's
 * snapshot, `project.import` into an empty project (all or nothing, the
 * refused object by its place), checkpoints (create, delete by their
 * maker or a manager, restore as a new project), and a new project in the
 * other storage mode (`project.convert`, docs/adr/0039). It follows
 * crates/server/application/src/files.rs, importing.rs, checkpoints.rs,
 * restore.rs and convert.rs; the real thing is tested against PostgreSQL in
 * Rust and end to end in the browser.
 */

/** What the files fake needs of the project fake it belongs to. */
export interface FileHost {
  readonly name: string;
  readonly userId: string;
  permissions(): ProjectPermission[];
  /** Throws as the server does for the project: offline, deleted, archived (when writing), hidden. */
  guard(writing: boolean): void;
  /** Records an event of the project; returns its cursor. */
  event(kind: string, requestId: string, meta?: boolean): EventRecord;
  /** Fills the project's objects and metadata from an imported drawing (at version 1, data revision 1). */
  imported(doc: DocumentSnapshotV2): void;
  /** Whether the database project has anything in it yet. */
  hasContent(): boolean;
  summary(): ProjectSummary;
}

const MAGIC = [0x89, 0x4b, 0x43, 0x41, 0x44, 0x0d, 0x0a, 0x1a, 0x0a];
const MAX = 256 * 1024 * 1024;

const invalid = (message: string, path?: string) => new ApiFailure(422, { error: 'invalid', message, ...(path ? { path } : {}) }, message);
const forbidden = (permission: string) =>
  new ApiFailure(403, { error: 'forbidden', message: `Bu projede bu işlem için yetkiniz yok (${permission}).` }, 'Yetki yok.');
const uploadGone = () =>
  new ApiFailure(404, { error: 'not_found', message: 'Yükleme bulunamadı: süresi dolmuş, kaydedilmiş ya da başkasına ait olabilir. Yeni bir yükleme başlatın.' }, 'Yükleme bulunamadı.');

interface Upload {
  id: string;
  size: number;
  sha256: string;
  bytes: Uint8Array | null;
  objects?: string;
}

interface Stored {
  revision: FileRevision;
  bytes: Uint8Array;
}

export class FakeFiles {
  private readonly host: FileHost;
  /** How the project is kept. */
  storage: ProjectStorage = 'database';
  readonly uploads = new Map<string, Upload>();
  readonly revisions: Stored[] = [];
  readonly checkpoints: (Checkpoint & { bytes: Uint8Array })[] = [];
  /** A database project's snapshot bytes (the test sets them; the fake does not encode). */
  snapshotBytes: Uint8Array | null = null;
  /** Reads a `.kcad` into its drawing (the tests' in-process codec): object counts, imports. */
  decode: ((bytes: Uint8Array) => Promise<DocumentSnapshotV2>) | null = null;
  /** The next `PUT` takes half the bytes and then the connection is cut. */
  cutNextSend = 0;
  /** The next commit is written, then its answer is lost. */
  loseNextCommitAnswer = false;
  /** The next download has one byte changed on the way. */
  corruptNextDownload = false;
  /** The server has no import (an older one): `project.import` is an unknown command. */
  noImport = false;
  /** Which object of an imported file the server refuses (its index), as for a value beyond ±10⁹. */
  refuseImportAt: number | null = null;
  /** The next upload's bytes are received and kept, but the answer is lost. */
  loseNextSendAnswer = false;
  /** A server before `GET …/uploads/{id}` (docs/adr/0040). */
  noUploadState = false;
  /** Projects made by restoring or converting, for the tests. */
  readonly restored: ProjectDuplicated[] = [];
  sends = 0;
  commits = 0;
  private readonly log = new Map<string, { text: string; result: unknown }>();

  constructor(host: FileHost) {
    this.host = host;
  }

  private may(permission: ProjectPermission): void {
    if (!this.host.permissions().includes(permission)) throw forbidden(permission);
  }

  /** Someone else commits a revision straight into the store (returns its event). */
  async commitAs(bytes: Uint8Array, by = 'Mehmet Demir', requestId = 'baskasi'): Promise<EventRecord> {
    const revision = String(this.revisions.length + 1);
    const objects = this.decode ? String((await this.decode(bytes)).entities.length) : undefined;
    this.revisions.push({
      revision: { revision, size: bytes.byteLength, sha256: await sha256Hex(bytes), createdBy: 'u2', createdByName: by, createdAt: '2026-09-26T11:00:00Z', ...(objects ? { objects } : {}) },
      bytes,
    });
    return this.host.event('project.file', requestId);
  }

  async beginUpload(begin: FileUploadBegin): Promise<FileUpload> {
    this.host.guard(true);
    this.may('feature.write');
    if (begin.size < 1 || begin.size > MAX) throw invalid(`Dosya 1 bayt ile 256 MiB arasında olmalı (${begin.size} bayt bildirildi).`, 'size');
    if (!/^[0-9a-f]{64}$/.test(begin.sha256)) throw invalid('sha256, dosyanın SHA-256 özeti olmalı: 64 küçük harfli onaltılık rakam.', 'sha256');
    const id = crypto.randomUUID();
    this.uploads.set(id, { id, size: begin.size, sha256: begin.sha256, bytes: null });
    return this.view(this.uploads.get(id)!);
  }

  private view(u: Upload): FileUpload {
    return { id: u.id, size: u.size, sha256: u.sha256, createdAt: '2026-09-26T10:00:00Z', expiresAt: '2026-09-27T10:00:00Z', received: !!u.bytes, ...(u.objects ? { objects: u.objects } : {}) };
  }

  async sendUpload(id: string, bytes: Uint8Array, progress: Transfer = () => {}): Promise<FileUpload> {
    this.host.guard(true);
    this.may('feature.write');
    this.sends++;
    const u = this.uploads.get(id);
    if (!u) throw uploadGone();
    if (u.bytes) throw invalid('Bu yüklemenin baytları zaten alındı; project.file.commit ile kaydedin ya da yeni bir yükleme başlatın.');
    if (this.cutNextSend) {
      this.cutNextSend--;
      progress(Math.floor(bytes.byteLength / 2), bytes.byteLength);
      // Nothing is kept of a body cut short: the same upload can be sent again.
      throw new ApiFailure(0, { error: 'network' }, 'Sunucuya ulaşılamadı; dosyanın gönderimi yarıda kaldı.');
    }
    progress(bytes.byteLength, bytes.byteLength);
    const sha = await sha256Hex(bytes);
    if (bytes.byteLength !== u.size || sha !== u.sha256)
      throw invalid(`Gelen dosya bildirilenle aynı değil (${bytes.byteLength} bayt, SHA-256 ${sha}; beklenen ${u.size} bayt, ${u.sha256}). Hiçbir şey saklanmadı.`, bytes.byteLength !== u.size ? 'size' : 'sha256');
    if (!MAGIC.every((b, i) => bytes[i] === b)) throw invalid('Yüklenen dosya geçerli bir KCAD v2 dosyası değil.');
    u.bytes = bytes.slice();
    if (this.decode) u.objects = String((await this.decode(bytes)).entities.length);
    if (this.loseNextSendAnswer) {
      // Received and kept: only the answer never arrives.
      this.loseNextSendAnswer = false;
      throw new ApiFailure(0, { error: 'network' }, 'Sunucuya ulaşılamadı; yanıt gelmedi.');
    }
    return this.view(u);
  }

  /** `GET …/uploads/{id}` (docs/adr/0040): the caller's upload as it stands; a server before it answers 405. */
  async uploadState(id: string): Promise<FileUpload> {
    this.host.guard(true);
    this.may('feature.write');
    if (this.noUploadState) throw new ApiFailure(405, {}, 'Sunucu 405 yanıtı verdi.');
    const u = this.uploads.get(id);
    if (!u) throw uploadGone();
    return this.view(u);
  }

  async fileRevisions(): Promise<FileRevisions> {
    this.host.guard(false);
    this.may('project.read');
    if (this.storage !== 'file') throw invalid(`“${this.host.name}” projesi nesne nesne veritabanında saklanıyor; dosya yüklenmez.`);
    const revisions = [...this.revisions].reverse().map((r) => r.revision);
    return { ...(revisions.length ? { current: revisions[0].revision } : {}), revisions };
  }

  private download(bytes: Uint8Array, sha256: string, revision: string, fileName: string, progress: Transfer = () => {}): KcadDownload {
    const out = new Uint8Array(bytes);
    if (this.corruptNextDownload) {
      this.corruptNextDownload = false;
      out[out.length >> 1] ^= 0x40;
    }
    progress(out.byteLength, out.byteLength);
    return { bytes: out, sha256, revision, cursor: null, fileName };
  }

  async fileRevision(revision: string, progress?: Transfer): Promise<KcadDownload> {
    this.host.guard(false);
    this.may('project.download');
    const r = this.revisions.find((x) => x.revision.revision === revision);
    if (!r) throw new ApiFailure(404, { error: 'not_found', message: `“${this.host.name}” projesinde böyle bir revizyon yok.` }, 'Yok.');
    return this.download(r.bytes, r.revision.sha256, revision, `${this.host.name}-r${revision}.kcad`, progress);
  }

  async snapshot(progress?: Transfer): Promise<KcadDownload> {
    this.host.guard(false);
    this.may('project.download');
    if (this.storage === 'file') throw invalid(`“${this.host.name}” projesi dosya olarak saklanıyor; revizyonlarını indirin.`);
    if (!this.snapshotBytes) throw new Error('sınama: snapshotBytes verilmedi');
    return { ...this.download(this.snapshotBytes, await sha256Hex(this.snapshotBytes), '1', `${this.host.name}-r1.kcad`, progress), cursor: '1' };
  }

  async listCheckpoints(): Promise<ProjectCheckpoints> {
    this.host.guard(false);
    this.may('project.history');
    return { checkpoints: [...this.checkpoints].reverse().map(({ bytes: _bytes, ...c }) => c) };
  }

  async checkpointFile(id: string, progress?: Transfer): Promise<KcadDownload> {
    this.host.guard(false);
    this.may('project.history');
    this.may('project.download');
    const c = this.checkpoints.find((x) => x.id === id);
    if (!c) throw new ApiFailure(404, { error: 'not_found', message: 'Kontrol noktası bulunamadı.' }, 'Yok.');
    return this.download(c.bytes, c.sha256, c.revision, `${this.host.name} - ${c.name}.kcad`, progress);
  }

  /** The file side's product commands; null when the command is not one of them. */
  async command(envelope: CommandEnvelope): Promise<unknown> {
    const text = JSON.stringify({ command: envelope.commandName, expected: envelope.expectedVersions, input: envelope.input });
    const earlier = this.log.get(envelope.idempotencyKey);
    if (earlier) {
      if (earlier.text !== text) throw invalid('Bu idempotency anahtarı başka bir istek için kullanılmış.');
      return { ...(earlier.result as object), replayed: true };
    }
    let result: unknown;
    switch (envelope.commandName) {
      case 'project.file.commit':
        result = await this.commit(envelope);
        break;
      case 'project.import':
        if (this.noImport) throw invalid('Bilinmeyen komut: project.import');
        result = await this.import(envelope);
        break;
      case 'project.checkpoint.create':
        result = await this.createCheckpoint(envelope);
        break;
      case 'project.checkpoint.delete':
        result = this.deleteCheckpoint(envelope);
        break;
      case 'project.checkpoint.restore':
        result = this.restore(envelope);
        break;
      case 'project.convert':
        result = await this.convert(envelope);
        break;
      default:
        return null;
    }
    this.log.set(envelope.idempotencyKey, { text, result });
    if (envelope.commandName === 'project.file.commit' && this.loseNextCommitAnswer) {
      this.loseNextCommitAnswer = false;
      throw new ApiFailure(0, { error: 'network' }, 'Yanıt kayboldu.');
    }
    return result;
  }

  private received(uploadId: string): Upload {
    const u = this.uploads.get(uploadId);
    if (!u) throw uploadGone();
    if (!u.bytes) throw invalid('Yüklemenin baytları henüz gelmedi; önce PUT …/uploads/{yükleme} ile gönderin.', 'uploadId');
    return u;
  }

  private async commit(envelope: CommandEnvelope): Promise<FileCommitted> {
    this.host.guard(true);
    this.may('feature.write');
    if (this.storage !== 'file') throw invalid(`“${this.host.name}” projesi nesne nesne veritabanında saklanıyor; dosya yüklenmez.`);
    const { uploadId } = envelope.input as FileCommit;
    const expected = envelope.expectedVersions[FILE_KEY];
    if (expected === undefined) throw invalid('project.file.commit, dosyanın dayandığı revizyonu ister.', 'expectedVersions[@file]');
    const current = String(this.revisions.length);
    if (expected !== current)
      throw new ApiFailure(
        409,
        {
          error: 'conflict',
          message: `Dosya siz kaydederken başka biri tarafından kaydedildi (şimdiki revizyon ${current}, sizinki ${expected}'e dayanıyor); hiçbir şey yazılmadı.`,
          conflicts: [{ id: FILE_KEY, reason: 'project', expected, actual: current }],
          revision: current,
        },
        'Çakışma',
      );
    const u = this.received(uploadId);
    const revision = String(this.revisions.length + 1);
    this.revisions.push({
      revision: { revision, size: u.size, sha256: u.sha256, createdBy: this.host.userId, createdByName: 'Ayşe Yılmaz', createdAt: '2026-09-26T10:30:00Z', ...(u.objects ? { objects: u.objects } : {}) },
      bytes: u.bytes!,
    });
    this.uploads.delete(uploadId);
    this.commits++;
    this.host.event('project.file', envelope.requestId);
    return { revision, size: u.size, sha256: u.sha256, ...(u.objects ? { objects: u.objects } : {}), replayed: false };
  }

  private async import(envelope: CommandEnvelope): Promise<ProjectImported> {
    this.host.guard(true);
    this.may('feature.write');
    this.may('project.edit');
    if (this.storage === 'file') throw invalid(`“${this.host.name}” projesi dosya olarak saklanıyor; dosyası project.file.commit ile kaydedilir.`);
    if (this.host.hasContent()) throw invalid(`“${this.host.name}” projesinde kayıt var; içe aktarım yalnız yeni, boş bir projeye yapılır.`);
    const { uploadId } = envelope.input as ProjectImport;
    const u = this.received(uploadId);
    if (!this.decode) throw new Error('sınama: decode verilmedi');
    const doc = await this.decode(u.bytes!);
    if (this.refuseImportAt !== null && this.refuseImportAt < doc.entities.length) {
      const i = this.refuseImportAt;
      throw invalid(`Dosyanın ${i + 1}. nesnesi içe aktarılamadı: koordinat ±1 000 000 000 sınırının dışında.`, `entities[${i}]`);
    }
    this.host.imported(doc);
    this.uploads.delete(uploadId);
    this.host.event('project.import', envelope.requestId, true);
    return { objects: String(doc.entities.length), dataRevision: '1', metaVersion: '2', replayed: false };
  }

  private async createCheckpoint(envelope: CommandEnvelope): Promise<CheckpointChange> {
    this.host.guard(true);
    this.may('feature.write');
    const input = envelope.input as CheckpointCreate;
    const name = input.name.trim();
    if (!name || name.length > 120) throw invalid('Kontrol noktasının adı 1 ile 120 karakter arasında olmalı.', 'name');
    if ((input.note ?? '').length > 2000) throw invalid('Not en çok 2000 karakter olabilir.', 'note');
    let kind: Checkpoint['kind'] = 'snapshot';
    let revision = '1';
    let bytes: Uint8Array;
    if (this.storage === 'file') {
      kind = 'revision';
      const r = input.fileRevision ? this.revisions.find((x) => x.revision.revision === input.fileRevision) : this.revisions.at(-1);
      if (!r) throw invalid(input.fileRevision ? `Böyle bir revizyon yok: ${input.fileRevision}.` : 'Projenin henüz revizyonu yok.', 'fileRevision');
      revision = r.revision.revision;
      bytes = r.bytes;
    } else {
      if (input.fileRevision) throw invalid('Veritabanı projesinin kontrol noktası revizyon almaz.', 'fileRevision');
      if (!this.snapshotBytes) throw new Error('sınama: snapshotBytes verilmedi');
      bytes = this.snapshotBytes;
    }
    const checkpoint: Checkpoint & { bytes: Uint8Array } = {
      id: crypto.randomUUID(),
      name,
      ...(input.note?.trim() ? { note: input.note.trim() } : {}),
      kind,
      revision,
      size: String(bytes.byteLength),
      sha256: await sha256Hex(bytes),
      createdBy: this.host.userId,
      createdByName: 'Ayşe Yılmaz',
      createdAt: '2026-09-26T12:00:00Z',
      bytes,
    };
    this.checkpoints.push(checkpoint);
    this.host.event('project.checkpoint', envelope.requestId);
    const { bytes: _bytes, ...shown } = checkpoint;
    return { checkpoint: shown, removed: false, replayed: false };
  }

  /** Someone else's checkpoint, straight into the store (for the rights tests). */
  addCheckpointBy(createdBy: string, name: string): Checkpoint {
    const c = { id: crypto.randomUUID(), name, kind: 'snapshot' as const, revision: '1', size: '4', sha256: '0'.repeat(64), createdBy, createdByName: 'Mehmet Demir', createdAt: '2026-09-26T09:00:00Z', bytes: new Uint8Array(4) };
    this.checkpoints.push(c);
    const { bytes: _bytes, ...shown } = c;
    return shown;
  }

  private deleteCheckpoint(envelope: CommandEnvelope): CheckpointChange {
    this.host.guard(true);
    // As the server: one who may write, then only its maker or someone with project.edit.
    this.may('feature.write');
    const { checkpointId } = envelope.input as CheckpointDelete;
    const at = this.checkpoints.findIndex((c) => c.id === checkpointId);
    if (at < 0) throw new ApiFailure(404, { error: 'not_found', message: 'Kontrol noktası bulunamadı.' }, 'Yok.');
    const c = this.checkpoints[at];
    if (c.createdBy !== this.host.userId && !this.host.permissions().includes('project.edit')) throw forbidden('project.edit');
    this.checkpoints.splice(at, 1);
    this.host.event('project.checkpoint', envelope.requestId);
    const { bytes: _bytes, ...shown } = c;
    return { checkpoint: shown, removed: true, replayed: false };
  }

  /** `project.convert` v1: a new project in the other mode; the source does not change (docs/adr/0039). */
  private async convert(envelope: CommandEnvelope): Promise<ProjectDuplicated> {
    this.host.guard(false);
    this.may('project.download');
    const input = envelope.input as { to: ProjectStorage; name?: string; tenantId?: string };
    if (input.to === this.storage) throw invalid(`“${this.host.name}” zaten ${input.to === 'file' ? 'dosya' : 'veritabanı'} projesi; öbür biçimi seçin.`, 'to');
    let objects: string;
    if (this.storage === 'file') {
      const newest = this.revisions.at(-1);
      if (!newest) throw invalid(`“${this.host.name}” projesinin kaydedilmiş revizyonu yok; dönüştürülecek bir şey yok.`);
      const doc = this.decode ? await this.decode(newest.bytes) : null;
      if (doc && this.refuseImportAt !== null && this.refuseImportAt < doc.entities.length)
        throw invalid(`Dosyanın ${this.refuseImportAt + 1}. nesnesi içe aktarılamadı: koordinat ±1 000 000 000 sınırının dışında.`, `entities[${this.refuseImportAt}]`);
      objects = doc ? String(doc.entities.length) : (newest.revision.objects ?? '0');
    } else objects = this.snapshotBytes && this.decode ? String((await this.decode(this.snapshotBytes)).entities.length) : '0';
    const source = this.host.summary();
    const made: ProjectDuplicated = {
      project: { ...source, id: crypto.randomUUID(), name: input.name?.trim() || `${source.name} (${input.to === 'database' ? 'PostGIS' : 'dosya'})`, tenantId: input.tenantId ?? source.tenantId, storage: input.to, favorite: false, state: 'active' },
      sourceId: source.id,
      objects,
      replayed: false,
    };
    this.restored.push(made);
    return made;
  }

  private restore(envelope: CommandEnvelope): ProjectDuplicated {
    this.host.guard(false);
    this.may('project.history');
    this.may('project.download');
    const input = envelope.input as CheckpointRestore;
    if (!!input.checkpointId === !!input.fileRevision) throw invalid('checkpointId ile fileRevision\'dan tam olarak biri verilmeli.');
    const point = input.checkpointId ? this.checkpoints.find((c) => c.id === input.checkpointId) : null;
    const revision = input.fileRevision ? this.revisions.find((r) => r.revision.revision === input.fileRevision) : null;
    if (input.checkpointId && !point) throw new ApiFailure(404, { error: 'not_found', message: 'Kontrol noktası bulunamadı.' }, 'Yok.');
    if (input.fileRevision && !revision) throw invalid(`Böyle bir revizyon yok: ${input.fileRevision}.`, 'fileRevision');
    const label = point ? point.name : `r${input.fileRevision}`;
    const source = this.host.summary();
    const copy: ProjectDuplicated = {
      project: { ...source, id: crypto.randomUUID(), name: input.name?.trim() || `${source.name} (${label})`, tenantId: input.tenantId ?? source.tenantId, favorite: false, state: 'active' },
      sourceId: source.id,
      objects: point?.objects ?? revision?.revision.objects ?? '0',
      replayed: false,
    };
    this.restored.push(copy);
    return copy;
  }
}
