import { Signal } from '../../core/signal';
import { unpackSnapshot } from '../../io/columns';
import { kcadInProcess } from '../../io/testFormats';
import type { CadDocument } from '../../model/document';
import { snapshotSampleDocument } from '../../model/snapshotSample';
import { encodeDrawing } from '../drawingFile';
import { setup as filesSetup } from '../fileTesting';
import { MemoryDraftStore } from './drafts';
import { FakeServer } from './fakeServer';
import type { SocketOptions } from './socket';
import { CloudSession, type SocketFactory } from './session';

/**
 * What the file project tests share (never used by the app): the fake
 * server reading `.kcad` with the formats module in process, a session over
 * it with a live channel the test drives, and the sample drawing as the
 * bytes of a revision.
 */

/** The fake server for a drawing, reading `.kcad` files as the real one does (object counts, imports). */
export function serverFor(doc: CadDocument): FakeServer {
  const server = new FakeServer({
    name: doc.name.value,
    settings: doc.settings.toJSON(),
    layers: [...doc.layers.tree],
    activeLayer: doc.layers.active.value,
    styles: structuredClone({ items: [...doc.styles.value.items], categories: [...doc.styles.value.categories] }),
    origin: doc.origin,
  });
  server.files.decode = async (bytes) => unpackSnapshot(await (await kcadInProcess()).decode(bytes));
  return server;
}

/** A drawing's `.kcad` v2 bytes, as Kaydet writes them. */
export async function bytesOf(doc: CadDocument): Promise<Uint8Array> {
  return (await encodeDrawing(doc, kcadInProcess)).bytes;
}

/** A live channel the test drives: `push` delivers events as the socket would. */
export function fakeSockets() {
  const made: { o: Omit<SocketOptions, 'url'>; started: boolean }[] = [];
  const factory: SocketFactory = (o) => {
    const socket = { o, started: false };
    made.push(socket);
    return {
      state: new Signal<'connecting' | 'online' | 'reconnecting' | 'offline' | 'auth_required'>('online'),
      start: () => void (socket.started = true),
      stop: () => void (socket.started = false),
      reconnect: () => {},
    };
  };
  return { factory, made, push: (events: Parameters<SocketOptions['onEvents']>[0]) => made.at(-1)?.o.onEvents(events) };
}

/**
 * The app's file service over the sample drawing with a cloud session on
 * the fake server (signed in as Ayşe, `u1`); waits are short so a lost
 * answer is tried again at once.
 */
export function cloudSetup(opts: { doc?: CadDocument; server?: FakeServer } = {}) {
  const doc = opts.doc ?? snapshotSampleDocument();
  const s = filesSetup(doc);
  const server = opts.server ?? serverFor(doc);
  const sockets = fakeSockets();
  const session = new CloudSession(s.ctx, server, new MemoryDraftStore(), sockets.factory);
  Object.assign(s.ctx, { cloud: session, files: s.files });
  session.me.set({ user: { id: 'u1', displayName: 'Ayşe Yılmaz', method: 'local' }, memberships: [] });
  session.auth.set('signedIn');
  return { ...s, server, session, sockets };
}
