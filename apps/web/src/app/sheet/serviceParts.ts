import type { Capabilities } from '../../contracts/generated/sheet/Capabilities';
import type { Workspace } from '../../contracts/generated/Workspace';
import type { ProjectTraits } from '../../product/sheet/templates';
import type { PaintSources } from '../../render/sheet/painter';
import type { AppContext } from '../context';
import { isLocal } from '../../geo/crs';
import { effectiveWorkspace } from '../workspaces';
import type { TemplateCloudApi } from './cloudApi';

/**
 * The sheet service's smaller parts (service.ts): what comes with the
 * engine, the paper's sources before it is there, the service's options,
 * and the open project as the engine's profile and the gallery read it.
 */

/** What comes with the engine (withEngine.ts): fetched with it, never at the app's start. */
export type EngineParts = typeof import('./withEngine');

/** The paper's sources before the painter is there (nothing is painted then: a sheet waits for the engine). */
export const NO_SOURCES: PaintSources = { image: () => null, map: () => null, family: (f) => f };

export interface ServiceOptions {
  /** Fetches what comes with the engine (tests may give their own). */
  readonly parts?: () => Promise<EngineParts>;
  /** The cloud's template library (tests give a fake; else the app's cloud client). */
  readonly templateApi?: TemplateCloudApi;
}

/** The open project as the sheets read it: its work mode, what it offers the tools (design §11a), its type. */
export class ProjectReading {
  private readonly ctx: AppContext;
  private attrs: { revision: number; any: boolean } | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  workspace(): Workspace {
    return effectiveWorkspace(this.ctx.doc.settings.workspace.value).id;
  }

  capabilities(): Capabilities {
    // A local project (no coordinate system, docs/adr/0165 §2) has no place on the earth to show.
    return { georeferenced: !isLocal(this.ctx.doc.crs.value), attributeLayers: this.hasAttributes(), plotScale: this.ctx.doc.settings.plotScale.value };
  }

  /** The project as the gallery sorts and checks templates for it (no project type yet: tasks-web.md Sapmalar 5). */
  traits(): ProjectTraits {
    return { workspace: this.workspace(), projectType: null, capabilities: this.capabilities() };
  }

  /** Whether some object carries attributes (read once per change of the drawing). */
  private hasAttributes(): boolean {
    const revision = this.ctx.doc.revision;
    if (this.attrs?.revision === revision) return this.attrs.any;
    let any = false;
    for (const e of this.ctx.doc.all())
      if (e.attrs && Object.values(e.attrs).some((v) => v !== '')) {
        any = true;
        break;
      }
    this.attrs = { revision, any };
    return any;
  }
}
