import type { ReadonlySignal } from '../../core/signal';
import type { ProjectAccessView } from '../../contracts/generated/ProjectAccessView';
import { ApiFailure, type CloudApi } from './api';
import type { CloudProject } from './session';
import { ROLE_LABEL } from './sharing';
import type { AccessOutcome } from './sync';

/**
 * Keeps what this account may do in the open cloud project current while it
 * is open (docs/adr/0015, TODOS.md CLOUD-13). When the server says a grant
 * changed (a `project.access` event), refuses a command (403) or ends the
 * live subscription (`not_found`), the watch reads the project again and
 * applies the answer: the buttons follow the new permissions and the
 * autosave holds or resumes (`ProjectSync.setAccess`; a file project's
 * Kaydet is refused or allowed again). No access any more (a
 * 404, or a membership that may not be used now: 403) stops the project the
 * way a deletion does. Checks never overlap; one asked for meanwhile runs
 * after. An unreachable server leaves things as they are until the next
 * signal.
 */

/** What an access change acts on: a database project's autosave, or a file project's Kaydet. */
export interface AccessTarget {
  readonly pending: ReadonlySignal<number>;
  setAccess(canWrite: boolean, canEditMeta: boolean): AccessOutcome;
  markDeleted(): void;
  markRevoked(reason?: string): void;
}

export interface AccessWatchOptions {
  api: CloudApi;
  sync: AccessTarget;
  /** A file project: saved by Kaydet, not by itself (the messages say so). */
  explicit?: boolean;
  /** The open project as the session holds it now, or null once another took its place. */
  project: () => CloudProject | null;
  /** Records new permissions (the session owns the project signal). */
  update: (next: CloudProject) => void;
  /** Subscribes to the live events again (after a refused subscription, when the access turns out to be there). */
  resubscribe: () => void;
  info: (text: string) => void;
  warn: (text: string) => void;
}

export class AccessWatch {
  private readonly o: AccessWatchOptions;
  private running: Promise<void> | null = null;
  private again = false;
  private resubscribe = false;

  constructor(o: AccessWatchOptions) {
    this.o = o;
  }

  /** Asks the server what this account may do now; `resubscribe` after the live subscription was refused. */
  check(resubscribe = false): Promise<void> {
    this.resubscribe ||= resubscribe;
    if (this.running) {
      this.again = true;
      return this.running;
    }
    this.running = this.run().finally(() => {
      this.running = null;
      if (this.again) {
        this.again = false;
        void this.check();
      }
    });
    return this.running;
  }

  private async run(): Promise<void> {
    const { o } = this;
    const p = o.project();
    if (!p) return;
    const resubscribe = this.resubscribe;
    this.resubscribe = false;
    try {
      const info = await o.api.project(p.tenantId, p.projectId);
      if (o.project()?.projectId !== p.projectId) return;
      this.apply(info.access);
      if (resubscribe) o.resubscribe();
    } catch (e) {
      if (o.project()?.projectId !== p.projectId || !(e instanceof ApiFailure)) return;
      if (e.deleted) o.sync.markDeleted();
      else if (e.notFound) o.sync.markRevoked();
      // Its organisation may not be used now (membership off, no seat): the server says which.
      else if (e.code === 'forbidden') o.sync.markRevoked(e.message);
    }
  }

  private apply(access: ProjectAccessView): void {
    const { o } = this;
    const p = o.project();
    if (!p) return;
    const permissions = access.permissions;
    if (p.role === access.role && p.permissions.join() === permissions.join()) return;
    const canWrite = permissions.includes('feature.write');
    const canEditMeta = permissions.includes('project.edit');
    o.update({ ...p, role: access.role, permissions, canWrite, canEditMeta });
    const outcome = o.sync.setAccess(canWrite, canEditMeta);
    const now = `“${p.name}” projesinde rolünüz artık ${ROLE_LABEL[access.role]}`;
    const unsent = o.sync.pending.value;
    if (o.explicit)
      switch (outcome) {
        case 'held':
          return o.warn(`${now}: çizim buluta kaydedilemez.${unsent ? ' Kaydedilmemiş değişiklikleri saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin.' : ''}`);
        case 'resumed':
          return o.info(`${now}: Kaydet yeniden buluta yeni bir revizyon yazar.`);
        default:
          return o.info(`${now}.`);
      }
    switch (outcome) {
      case 'held':
        return o.warn(
          `${now}: değişiklikleriniz buluta kaydedilmiyor.${unsent ? ` Gönderilmemiş ${unsent} değişiklik bu cihazda saklanıyor; düzenleme yetkiniz dönünce gönderilir.` : ''}`,
        );
      case 'reopen':
        return o.warn(`${now}. Salt okunurken yaptığınız değişiklikler kaydedilmez; kaydetmeye başlamak için projeyi Dosya → Bulut projesi aç ile yeniden açın.`);
      case 'resumed':
        return o.info(`${now}: değişiklikleriniz yeniden buluta kaydediliyor.`);
      case 'same':
        return o.info(`${now}.`);
    }
  }
}
