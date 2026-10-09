import { initCoreFrom } from '../../wasm/core';
import { NetworkHost } from './handle';
import type { NetworkReply, NetworkRequest } from './protocol';

/**
 * Entry of the network Web Worker (started by app/networks.ts, docs/adr/0209 §12): builds the project's networks and
 * answers their questions off the page's thread; everything else is in handle.ts.
 */
const host = new NetworkHost();
const scope = self as unknown as { onmessage: ((e: MessageEvent<NetworkRequest>) => void) | null; postMessage(m: NetworkReply, transfer?: Transferable[]): void };

scope.onmessage = (e) => {
  const msg = e.data;
  // The page's compiled geometry core comes with the first message; this worker starts its own copy of it.
  if (msg.type !== 'drop' && msg.core) initCoreFrom(msg.core);
  host.handle(msg, (reply, transfer) => scope.postMessage(reply, transfer ?? []));
};
