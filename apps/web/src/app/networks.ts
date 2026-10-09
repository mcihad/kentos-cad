import type { NetworkDef } from '../contracts/generated/NetworkDef';
import { Signal } from '../core/signal';
import type { Disposable } from '../core/disposable';
import type { CadDocument } from '../model/document';
import { NetworkHost } from '../io/network/handle';
import type { NetworkQuestion, NetworkReply, NetworkRequest, NetworkSummary, XY } from '../io/network/protocol';
import { coreModule } from '../wasm/core';

/**
 * The project's networks as the page asks them (docs/adr/0209 §10, §12). A network is built in the network worker
 * from its layers' objects the first time it is asked, and again only when one of its layers' objects, the layer tree
 * or its definition changed; questions go to the worker in order and come back with their paths ready to draw. The way
 * to the cursor, asked on every pointer move, is asked one at a time: a newer one waiting takes the place of an older
 * one (which answers null). Without Web Workers (the tests) the same work runs in the page. The desktop's is
 * `apps/desktop/src/networks.rs`.
 */

/** The part of a Worker the service uses (a fake in tests). */
export interface NetworkWorkerLike {
  postMessage(message: NetworkRequest, transfer?: Transferable[]): void;
  terminate(): void;
  onmessage: ((e: { data: NetworkReply }) => void) | null;
  onerror: ((e: unknown) => void) | null;
}

/** An answer: the core's (parsed) and the paths and fills it draws as. */
export interface NetworkAnswer<T> {
  readonly value: T;
  readonly paths: Float64Array;
  readonly fills: Float64Array;
}

interface Waiting {
  resolve(reply: NetworkReply): void;
  reject(e: Error): void;
}

/** A network as last built: its definition, the change count it was built at, and its summary when it comes. */
interface Built {
  readonly key: string;
  readonly at: number;
  readonly layers: ReadonlySet<string>;
  readonly summary: Promise<NetworkSummary>;
}

interface PathAsk {
  readonly network: string;
  readonly at: XY;
  readonly reach: number;
  resolve(a: NetworkAnswer<{ cost: number } | null> | null): void;
  reject(e: Error): void;
}

export class NetworkService {
  /** Whether anything is being built or asked (the trace players wait until it is not). */
  readonly busy = new Signal(false);
  private readonly doc: CadDocument;
  private readonly spawn: (() => NetworkWorkerLike) | null;
  private worker: NetworkWorkerLike | null = null;
  /** The page's own host when there is no worker. */
  private host: NetworkHost | null = null;
  private fresh = false;
  private seq = 0;
  private readonly waiting = new Map<number, Waiting>();
  private readonly built = new Map<string, Built>();
  /** How many times each layer's objects changed (a network is built again when one of its layers' counts moved). */
  private readonly changes = new Map<string, number>();
  /** Changes of the layer tree (names: `$katman`) and of the whole drawing. */
  private everything = 0;
  private pathBusy = false;
  private pathNext: PathAsk | null = null;
  private readonly subs: Disposable[] = [];

  /** Says what a build said (the app's log; none in the tests that do not listen). */
  private readonly say: (text: string) => void;

  constructor(doc: CadDocument, spawn: (() => NetworkWorkerLike) | null, say: (text: string) => void = () => {}) {
    this.doc = doc;
    this.spawn = spawn;
    this.say = say;
    const touch = (layer: string) => this.changes.set(layer, (this.changes.get(layer) ?? 0) + 1);
    this.subs.push(
      doc.events.on('changed', ({ layerIds }) => layerIds.forEach(touch)),
      doc.events.on('attrs', ({ ids }) => {
        for (const id of ids) {
          const e = doc.get(id);
          if (e) touch(e.layerId);
        }
      }),
      doc.events.on('reset', () => this.everything++),
      doc.layers.events.on('structure', () => this.everything++),
      // A network taken away is forgotten by the worker too.
      doc.settings.networks.subscribe((list) => {
        for (const id of [...this.built.keys()]) if (!list.some((n) => n.id === id)) this.forget(id);
      }),
    );
  }

  /** The project's networks. */
  list(): readonly NetworkDef[] {
    return this.doc.settings.networks.value;
  }

  find(id: string): NetworkDef | undefined {
    return this.list().find((n) => n.id === id);
  }

  /** The change count of a network's layers now: it is built again when this moved since. */
  private stamp(layers: ReadonlySet<string>): number {
    let n = this.everything;
    for (const l of layers) n += this.changes.get(l) ?? 0;
    return n;
  }

  /**
   * Network `id` built for the drawing as it is now: its summary (built now or before). `def`: a definition not saved
   * yet (Ağlar's Denetle) in place of the project's. Refused when the project has no such network or the worker could
   * not build it.
   */
  ready(id: string, def: NetworkDef | undefined = this.find(id)): Promise<NetworkSummary> {
    if (!def) return Promise.reject(new Error(`“${id}” kimlikli ağ projede yok; Ağlar penceresinden tanımlayın.`));
    const key = JSON.stringify(def);
    const layers = new Set([...def.edges.map((l) => l.layer), ...(def.junctions ?? []).map((l) => l.layer)]);
    const old = this.built.get(id);
    if (old && old.key === key && old.at === this.stamp(layers)) return old.summary;
    const entities = [...layers].flatMap((l) => this.doc.byLayer(l));
    const names = this.doc.layers.leaves().map((l) => [l.id, l.name] as const);
    const summary = this.send({ type: 'build', id: 0, network: id, def, entities, layers: names }).then((r) => {
      if (r.type !== 'built') throw new Error('Ağ kurulamadı.');
      // What building said, once a build (not every question) says it: expressions that did not compile, objects
      // of the network's layers not taken.
      for (const p of r.summary.problems) this.say(`${def.name}: ${p}`);
      const skipped = r.summary.skipped.length;
      if (skipped) this.say(`${def.name}: ağın katmanlarında ${skipped} nesne çizgi, çoklu çizgi, yay ya da nokta değil; ağa alınmadı.`);
      return r.summary;
    });
    const record: Built = { key, at: this.stamp(layers), layers, summary };
    this.built.set(id, record);
    // A build that failed is tried again next time.
    summary.catch(() => {
      if (this.built.get(id) === record) this.built.delete(id);
    });
    return summary;
  }

  /** Asks network `id` (built first when it needs to be; `def` as `ready` takes it). */
  async ask<T>(id: string, question: NetworkQuestion, def?: NetworkDef): Promise<NetworkAnswer<T>> {
    await this.ready(id, def ?? this.find(id));
    const r = await this.send({ type: 'ask', id: 0, network: id, question });
    if (r.type !== 'answer') throw new Error('Ağ yanıt vermedi.');
    return { value: JSON.parse(r.json) as T, paths: r.paths, fills: r.fills };
  }

  /**
   * The way from the kept tree (`treeFrom`) to `at`: null when a newer call took its place before it was asked; the
   * answer's value is null when there is no way.
   */
  pathTo(id: string, at: XY, reach: number): Promise<NetworkAnswer<{ cost: number } | null> | null> {
    return new Promise((resolve, reject) => {
      const ask: PathAsk = { network: id, at, reach, resolve, reject };
      if (!this.pathBusy) return void this.askPath(ask);
      this.pathNext?.resolve(null);
      this.pathNext = ask;
    });
  }

  private askPath(a: PathAsk): void {
    this.pathBusy = true;
    this.ask<{ cost: number } | null>(a.network, { kind: 'pathTo', at: a.at, reach: a.reach })
      .then(a.resolve, a.reject)
      .finally(() => {
        this.pathBusy = false;
        const next = this.pathNext;
        this.pathNext = null;
        if (next) this.askPath(next);
      });
  }

  /** Forgets a network (taken away or changed beyond use): the worker frees it. */
  private forget(id: string): void {
    this.built.delete(id);
    if (this.worker) this.worker.postMessage({ type: 'drop', network: id });
    else this.host?.handle({ type: 'drop', network: id }, () => {});
  }

  private send(msg: NetworkRequest): Promise<NetworkReply> {
    const id = ++this.seq;
    const out = { ...msg, id } as NetworkRequest;
    return new Promise<NetworkReply>((resolve, reject) => {
      this.waiting.set(id, {
        resolve: (r) => {
          this.waiting.delete(id);
          this.busy.set(this.waiting.size > 0);
          if (r.type === 'error') reject(new Error(r.message));
          else resolve(r);
        },
        reject: (e) => {
          this.waiting.delete(id);
          this.busy.set(this.waiting.size > 0);
          reject(e);
        },
      });
      this.busy.set(true);
      try {
        const w = this.ensure();
        if (w) {
          if (this.fresh && out.type !== 'drop') (out as { core?: WebAssembly.Module }).core = coreModule() ?? undefined;
          this.fresh = false;
          w.postMessage(out);
        } else {
          // No worker: the page's own host, answering after this call returns as a worker would.
          queueMicrotask(() => this.host!.handle(out, (reply) => this.heard(reply)));
        }
      } catch (err) {
        this.waiting.get(id)?.reject(new Error(`Ağ işçisine gönderilemedi: ${(err as Error).message}`));
      }
    });
  }

  private heard(reply: NetworkReply): void {
    this.waiting.get(reply.id)?.resolve(reply);
  }

  /** The worker, started on first use; null when the page does the work. */
  private ensure(): NetworkWorkerLike | null {
    if (this.worker) return this.worker;
    if (!this.spawn) {
      this.host ??= new NetworkHost();
      return null;
    }
    const w = this.spawn();
    w.onmessage = (e) => this.heard(e.data);
    w.onerror = () => {
      // The worker is gone with every network it built: those are built again on the next question.
      w.terminate();
      this.worker = null;
      this.built.clear();
      for (const p of [...this.waiting.values()]) p.reject(new Error('Ağ işçisi beklenmedik biçimde durdu; yeniden deneyin.'));
    };
    this.worker = w;
    this.fresh = true;
    return w;
  }

  dispose(): void {
    for (const d of this.subs) d();
    this.subs.length = 0;
    this.worker?.terminate();
    this.worker = null;
    for (const p of [...this.waiting.values()]) p.reject(new Error('Ağlar kapandı.'));
  }
}
