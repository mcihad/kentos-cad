/// <reference lib="webworker" />
import type { Entity } from '../../contracts/generated/Entity';
import type { ImportResult } from '../../contracts/generated/ImportResult';
import type { ReportItem } from '../../contracts/generated/ReportItem';
import { serviceText, type Proof, type Wire } from './fetch';
import { loadServices } from './module';
import type { TakeAnswer, TakeAsk } from './protocol';

/**
 * Servisten veri al's worker (docs/adr/0208 §10; the desktop's `feed_window::take_all`): a feed's objects taken page
 * by page (the services core's `Taking`), each request with the connection's proof, every page moved into the
 * project's system (`featuresToProject`, the project's datum choices taken; an object with a vertex that has no place
 * there left out and counted). The count goes to the page after each page; the page stops a take by ending the worker,
 * which aborts the request on its way. Nothing here touches the drawing: the page writes what comes back as one undo
 * step.
 */

declare const self: DedicatedWorkerGlobalScope;

/** Pages at most: a service that never says it is done stops here. */
const MOST_PAGES = 1000;

const post = (m: TakeAnswer) => self.postMessage(m);

async function take(ask: TakeAsk): Promise<void> {
  const m = await loadServices();
  const taking = new m.Taking(JSON.stringify(ask.feed), ask.area ? Float64Array.from(ask.area) : new Float64Array(0), ask.most, ask.geojson);
  try {
    const proof: Proof | null = ask.connection ? { connection: ask.connection, secret: ask.secret } : null;
    const opts = { proxy: ask.proxy, referer: ask.feed.url };
    const entities: Entity[] = [];
    let dropped = 0;
    let next: Wire | null = JSON.parse(taking.first()) as Wire;
    for (let pages = 1; next && pages <= MOST_PAGES; pages++) {
      const body = await serviceText(m, next, proof, opts);
      const page = JSON.parse(taking.answer(body)) as { result: ImportResult; next: Wire | null };
      let result = page.result;
      if (ask.move) {
        const moved = JSON.parse(m.featuresToProject(JSON.stringify(result), ask.move.from, ask.move.to, ask.move.choices)) as { result: ImportResult; dropped: number };
        result = moved.result;
        dropped += moved.dropped;
      }
      for (const e of result.entities) entities.push(e);
      post({ type: 'progress', taken: entities.length });
      next = page.next;
    }
    const matched = taking.matched();
    const skipped = JSON.parse(taking.skipped()) as ReportItem[];
    post({ type: 'taken', entities, dropped, matched: matched ?? null, capped: taking.taken() >= ask.most, skipped });
  } finally {
    taking.free();
  }
}

self.onmessage = (e: MessageEvent<TakeAsk>) => {
  if (e.data?.type !== 'take') return;
  take(e.data).catch((err: unknown) => post({ type: 'failed', message: err instanceof Error ? err.message : String(err) }));
};
