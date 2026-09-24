import type { Command } from '../core/commands';
import type { Disposable } from '../core/disposable';
import { Signal, type ReadonlySignal } from '../core/signal';
import type { CadDocument } from '../model/document';
import type { Bounds } from '../model/geometry';
import type { Selection } from '../model/selection';
import { BUILTIN_TOOLS } from '../processing/builtin';
import { BUILTIN_MODELS } from '../processing/builtin/models';
import type { DocumentGeometry } from '../processing/geometry';
import type { ProcessingModel } from '../processing/model';
import { ProcessingRegistry } from '../processing/registry';
import { clientExecutor, type Executor } from '../processing/job';
import { ProcessingRunner, type TargetChoice } from '../processing/runner';
import { workerExecutor, type WorkerLike } from '../processing/worker/workerExecutor';
import type { AppContext } from './context';
import { persistedSignals } from './state';

/**
 * İşlem araçları in the app: the registry with the built-in tools, the
 * runner bound to this document, and each tool's last values (a user
 * preference kept in this browser, `kentos.processing.v1`).
 */
export interface ProcessingService {
  readonly registry: ProcessingRegistry;
  readonly runner: ProcessingRunner;
  /** Values of the tool's last run in this browser, if any. */
  lastValues(toolId: string): Record<string, unknown> | undefined;
  remember(toolId: string, values: Record<string, unknown>): void;
  /** Where the user wants the tool to run (Otomatik unless changed). */
  targetChoice(toolId: string): TargetChoice;
  setTargetChoice(toolId: string, choice: TargetChoice): void;
  /** Models: the built-in ones, then the user's (kept in this browser). */
  readonly models: ReadonlySignal<readonly ProcessingModel[]>;
  model(id: string): ProcessingModel | undefined;
  isBuiltinModel(id: string): boolean;
  saveModel(model: ProcessingModel): void;
  removeModel(id: string): void;
}

interface ProcessingMemory {
  lastValues: Record<string, Record<string, unknown>>;
  targets: Record<string, TargetChoice>;
  models: ProcessingModel[];
}

/** The page, and a Web Worker where the browser has one (the worker carries the built-in tools). */
function createExecutors(): Executor[] {
  if (typeof Worker === 'undefined') return [clientExecutor];
  const spawn = () => new Worker(new URL('../processing/worker/processingWorker.ts', import.meta.url), { type: 'module', name: 'KentOS işlemleri' }) as unknown as WorkerLike;
  return [clientExecutor, workerExecutor(spawn, new Set(BUILTIN_TOOLS.map((t) => t.id)))];
}

/**
 * `visibleBounds`: the world box on screen; `geometry`: the drawing's
 * geometry store (the viewport's), for the "visible" scope and the dialog's
 * previews.
 */
export function createProcessing(doc: CadDocument, selection: Selection, visibleBounds: () => Bounds | null, geometry: DocumentGeometry): ProcessingService {
  const registry = new ProcessingRegistry();
  for (const t of BUILTIN_TOOLS) registry.register(t);
  const runner = new ProcessingRunner({ doc, selectedIds: () => [...selection.ids.value], visibleBounds, geometry, select: (ids) => selection.set(ids) }, createExecutors());
  const memory = persistedSignals<ProcessingMemory>('kentos.processing.v1', { lastValues: {}, targets: {}, models: [] });
  const builtin = new Set(BUILTIN_MODELS.map((m) => m.id));
  const models = new Signal<readonly ProcessingModel[]>([]);
  memory.models.subscribe((mine) => models.set([...BUILTIN_MODELS, ...mine.filter((m) => !builtin.has(m.id))]), true);
  return {
    models,
    model: (id) => models.value.find((m) => m.id === id),
    isBuiltinModel: (id) => builtin.has(id),
    saveModel: (m) => {
      if (builtin.has(m.id)) throw new Error('Hazır modeller değiştirilemez; kopyasını kaydedin.');
      const copy = JSON.parse(JSON.stringify(m)) as ProcessingModel;
      const mine = memory.models.value;
      memory.models.set(mine.some((x) => x.id === m.id) ? mine.map((x) => (x.id === m.id ? copy : x)) : [...mine, copy]);
    },
    removeModel: (id) => memory.models.set(memory.models.value.filter((m) => m.id !== id)),
    registry,
    runner,
    lastValues: (id) => memory.lastValues.value[id],
    remember: (id, values) => memory.lastValues.set({ ...memory.lastValues.value, [id]: JSON.parse(JSON.stringify(values)) }),
    targetChoice: (id) => memory.targets.value[id] ?? 'auto',
    setTargetChoice: (id, choice) => memory.targets.set({ ...memory.targets.value, [id]: choice }),
  };
}

export const processingCommandId = (toolId: string) => `processing.run.${toolId}`;

/** One command per tool (menus, command line) plus the toolbox and history. */
export const modelCommandId = (modelId: string) => `processing.model.${modelId}`;

export interface ProcessingHooks {
  open(toolId: string, values?: Record<string, unknown>): void;
  openModel(modelId: string, values?: Record<string, unknown>): void;
  /** Opens the model designer on a model, or on a new one. */
  design(modelId?: string): void;
  show(tab: 'tools' | 'history'): void;
}

/** One command per tool and per model (menus, command line) plus the toolbox, history and designer. */
export function registerProcessingCommands(ctx: AppContext, hooks: ProcessingHooks): void {
  const cat = 'İşlemler';
  const list: Command[] = [
    { id: 'processing.toolbox', title: 'İşlem araç kutusu', category: cat, icon: 'processing', aliases: ['ISLEMLER', 'PROCESSING'], description: 'Toplu işlem araçlarını sağ panelde listeler.', run: () => hooks.show('tools') },
    // Harita menüsündeki eski komut, aynı işi yapan işlem aracını açar.
    { id: 'map.edgeLengths', title: 'Kenar ölçülerini yaz…', category: 'Harita', icon: 'dimension', aliases: ['KENAR', 'KENAROLCU'], description: 'Parsel ve çizgilerin kenar uzunluklarını yazar (işlem aracı).', run: () => hooks.open('annotation.edgeLengths') },
    { id: 'processing.history', title: 'İşlem geçmişi', category: cat, icon: 'history', description: 'Bu oturumda çalıştırılan işlemler; yeniden çalıştırılabilir.', run: () => hooks.show('history') },
    {
      id: 'processing.newModel',
      title: 'Yeni model…',
      category: cat,
      icon: 'modelNew',
      aliases: ['MODEL', 'YENIMODEL'],
      description: 'İşlem araçlarını birbirine bağlayan bir akış (model) tasarlar.',
      run: (id) => hooks.design(typeof id === 'string' ? id : undefined),
    },
    ...ctx.processing.registry.list().map(
      (t): Command => ({
        id: processingCommandId(t.id),
        title: `${t.label}…`,
        category: cat,
        icon: t.icon,
        description: t.description,
        aliases: t.aliases ? [...t.aliases] : undefined,
        run: (values) => hooks.open(t.id, isValues(values) ? values : undefined),
      }),
    ),
  ];
  for (const c of list) ctx.commands.register(c);

  // Model commands follow the library: saved, renamed and removed models.
  let registered: Disposable[] = [];
  ctx.processing.models.subscribe((models) => {
    registered.forEach((d) => d());
    registered = models.map((m) =>
      ctx.commands.register({
        id: modelCommandId(m.id),
        title: `${m.label}…`,
        category: cat,
        icon: 'processing',
        description: m.description,
        run: (values) => hooks.openModel(m.id, isValues(values) ? values : undefined),
      }),
    );
  }, true);
}

const isValues = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
