import type { AppContext } from '../../app/context';
import { changesHistory, loadHistory } from '../../app/cloud/history';
import type { ProjectDuplicated } from '../../contracts/generated/ProjectDuplicated';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { askRemove } from '../widgets/confirm';
import type { HistoryActions, HistoryState } from './catalogHistory';
import type { DownloadRequest } from './downloads';
import { openCheckpointDialog, openRestoreDialog, type HistoryTarget } from './HistoryForms';
import { reason } from './ProjectActions';

/**
 * The catalog's history tab behind the scenes (docs/adr/0034, 0038): it
 * asks the server for the selected project's revisions and checkpoints,
 * listens to the project's events while the tab shows it (a
 * `project.checkpoint` or `project.file` event asks again), and does what
 * the rows offer: download, name a checkpoint, remove one after a question,
 * restore a point as a new project (which the catalog then opens).
 */

export interface HistoryPanelOptions {
  /** Draws the tab again. */
  paint(): void;
  /** Downloads a file with its progress in the window. */
  download(request: Omit<DownloadRequest, 'say'>): void;
  /** A project made from the history (a restored point): the catalog shows and opens it. */
  opened(made: ProjectDuplicated): void;
}

/** How long to wait after a burst of events before asking again. */
const SETTLE_MS = 250;

export class HistoryPanel {
  /** What the tab shows. */
  state: HistoryState = 'none';
  readonly actions: HistoryActions;
  private readonly ctx: AppContext;
  private readonly o: HistoryPanelOptions;
  private project: ProjectSummary | null = null;
  private gen = 0;
  private timer = 0;
  private unwatch: (() => void) | null = null;

  constructor(ctx: AppContext, o: HistoryPanelOptions) {
    this.ctx = ctx;
    this.o = o;
    const target = (): HistoryTarget | null => {
      const p = this.project;
      return p && { tenantId: p.tenantId, projectId: p.id, name: p.name, storage: p.storage };
    };
    this.actions = {
      retry: () => this.ask(),
      downloadRevision: (r) => {
        const p = this.project;
        if (p) this.o.download({ name: `${p.name} (revizyon ${r.revision})`, fetch: (step, signal) => this.ctx.cloud.api.fileRevision(p.tenantId, p.id, r.revision, step, signal), listed: r.sha256 });
      },
      restoreRevision: (r) => {
        const t = target();
        if (t) openRestoreDialog(this.ctx, t, { revision: r }, this.o.opened);
      },
      createCheckpoint: () => {
        const t = target();
        const revisions = typeof this.state === 'object' && !(this.state instanceof Error) ? this.state.revisions : null;
        if (t) openCheckpointDialog(this.ctx, t, revisions, () => this.ask());
      },
      downloadCheckpoint: (c) => {
        const p = this.project;
        if (p) this.o.download({ name: `${p.name} - ${c.name}`, fetch: (step, signal) => this.ctx.cloud.api.checkpointFile(p.tenantId, p.id, c.id, step, signal), listed: c.sha256 });
      },
      restoreCheckpoint: (c) => {
        const t = target();
        if (t) openRestoreDialog(this.ctx, t, { checkpoint: c }, this.o.opened);
      },
      deleteCheckpoint: (c) => void this.remove(c.id, c.name, c.kind === 'revision'),
    };
  }

  /** Shows `p`'s history (null: none): asked now, and again whenever its events say it changed. */
  show(p: ProjectSummary | null): void {
    const same = !!p && p.id === this.project?.id && p.tenantId === this.project.tenantId;
    this.project = p;
    if (!same) {
      this.unwatch?.();
      this.unwatch = p && p.state !== 'trashed' ? this.ctx.cloud.watchProject(p.tenantId, p.id, (events) => changesHistory(events) && this.later()) : null;
    }
    this.ask();
  }

  /** The tab is hidden or the window closes: nothing more is asked or heard. */
  hide(): void {
    this.gen++;
    clearTimeout(this.timer);
    this.unwatch?.();
    this.unwatch = null;
    this.project = null;
    this.state = 'none';
  }

  private later(): void {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => this.ask(true), SETTLE_MS) as unknown as number;
  }

  /** Asks the server; `quiet`: the list stays as it is until the answer (an event's refresh). */
  private ask(quiet = false): void {
    const p = this.project;
    const gen = ++this.gen;
    if (!p || p.state === 'trashed') {
      this.state = 'none';
      return this.o.paint();
    }
    if (!quiet || typeof this.state !== 'object' || this.state instanceof Error) {
      this.state = 'loading';
      this.o.paint();
    }
    loadHistory(this.ctx.cloud.api, p.tenantId, p.id, p.storage, p.access.permissions).then(
      (data) => {
        if (gen !== this.gen) return;
        this.state = data;
        this.o.paint();
      },
      (e: unknown) => {
        if (gen !== this.gen) return;
        this.state = e instanceof Error ? e : new Error(String(e));
        this.o.paint();
      },
    );
  }

  /** Asks, then removes a checkpoint (its maker, or someone who manages the project). */
  private async remove(id: string, name: string, revision: boolean): Promise<void> {
    const p = this.project;
    if (!p) return;
    const ok = await askRemove({
      title: 'Kontrol noktasını sil',
      message: `“${name}” kontrol noktası silinsin mi?`,
      details: [
        revision ? 'Adlandırdığı revizyon kalır; yalnız ad ve not silinir.' : 'Saklanan anlık görüntünün dosyası da silinir; bu işlem geri alınamaz.',
        'Projenin kendisi ve öbür kontrol noktaları değişmez.',
      ],
      action: 'Sil',
    });
    if (!ok) return;
    try {
      await this.ctx.cloud.lifecycle.deleteCheckpoint({ tenantId: p.tenantId, projectId: p.id, name: p.name }, id);
      this.ctx.log.success(`“${name}” kontrol noktası silindi.`);
      this.ask();
    } catch (e) {
      this.ctx.log.error(`“${name}” kontrol noktası silinemedi: ${reason(e)}`);
    }
  }
}
