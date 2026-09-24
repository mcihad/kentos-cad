import { Signal } from '../../core/signal';
import type { ClientMessage } from '../../contracts/generated/ClientMessage';
import type { EventRecord } from '../../contracts/generated/EventRecord';
import type { ServerMessage } from '../../contracts/generated/ServerMessage';

/**
 * The live channel of an open cloud project (`/v1/ws`, CLAUDE.md §21.1).
 * It subscribes with the cursor the app has applied, so missed events are
 * replayed first. A heartbeat goes out every 20 s; no word from the server
 * for 60 s counts as a dead link. Reconnects back off from 1 s to 30 s with
 * jitter; a signed-out session stops retrying until the user signs in. The
 * socket carries notifications only: commits go over HTTP and a server job
 * never depends on it.
 */

export type LinkState = 'connecting' | 'online' | 'reconnecting' | 'offline' | 'auth_required';

export interface SocketLike {
  readyState: number;
  send(data: string): void;
  close(): void;
  onopen: ((ev: Event) => void) | null;
  onclose: ((ev: CloseEvent) => void) | null;
  onmessage: ((ev: MessageEvent) => void) | null;
  onerror: ((ev: Event) => void) | null;
}

export interface SocketOptions {
  url: string;
  tenantId: string;
  projectId: string;
  /** The cursor the app has applied (read at every (re)subscribe). */
  cursor: () => string;
  onEvents: (events: EventRecord[]) => void;
  /** The server cannot continue from our cursor: reopen the project. */
  onResync: () => void;
  onError: (message: string) => void;
  open?: (url: string) => SocketLike;
  heartbeatMs?: number;
  deadMs?: number;
}

const OPEN = 1;

export class ProjectSocket {
  readonly state = new Signal<LinkState>('connecting');
  /** When the server last answered (ms since epoch). */
  readonly lastHeard = new Signal(0);
  private readonly o: SocketOptions;
  private socket: SocketLike | null = null;
  private attempts = 0;
  private stopped = false;
  private heartbeat = 0;
  private retry = 0;
  private readonly onOnline = () => this.state.value !== 'online' && this.connect();

  constructor(o: SocketOptions) {
    this.o = o;
  }

  start(): void {
    this.stopped = false;
    if (typeof window !== 'undefined') window.addEventListener('online', this.onOnline);
    this.connect();
  }

  stop(): void {
    this.stopped = true;
    if (typeof window !== 'undefined') window.removeEventListener('online', this.onOnline);
    clearInterval(this.heartbeat);
    clearTimeout(this.retry);
    const s = this.socket;
    this.socket = null;
    s?.close();
    this.state.set('offline');
  }

  /** Try again now (after signing in again, or when the user asks). */
  reconnect(): void {
    this.attempts = 0;
    this.stopped = false;
    this.connect();
  }

  private send(msg: ClientMessage): void {
    if (this.socket?.readyState === OPEN) this.socket.send(JSON.stringify(msg));
  }

  private connect(): void {
    if (this.stopped) return;
    clearTimeout(this.retry);
    const old = this.socket;
    this.socket = null;
    old?.close();
    this.state.set(this.attempts === 0 ? 'connecting' : 'reconnecting');
    let s: SocketLike;
    try {
      s = (this.o.open ?? ((u) => new WebSocket(u) as SocketLike))(this.o.url);
    } catch {
      return this.later();
    }
    this.socket = s;
    s.onopen = () => {
      if (this.socket !== s) return;
      this.lastHeard.set(Date.now());
      this.send({ type: 'subscribe', tenantId: this.o.tenantId, projectId: this.o.projectId, after: this.o.cursor() });
      clearInterval(this.heartbeat);
      this.heartbeat = setInterval(() => this.beat(), this.o.heartbeatMs ?? 20_000) as unknown as number;
    };
    s.onmessage = (ev) => {
      if (this.socket !== s) return;
      this.lastHeard.set(Date.now());
      let msg: ServerMessage;
      try {
        msg = JSON.parse(String(ev.data)) as ServerMessage;
      } catch {
        return;
      }
      this.handle(msg);
    };
    s.onclose = () => {
      if (this.socket !== s) return;
      this.socket = null;
      clearInterval(this.heartbeat);
      if (this.state.value !== 'auth_required') this.later();
    };
    s.onerror = () => {};
  }

  private handle(msg: ServerMessage): void {
    switch (msg.type) {
      case 'subscribed':
        this.attempts = 0;
        this.state.set('online');
        return;
      case 'events':
        if (msg.events.length) this.o.onEvents(msg.events);
        return;
      case 'resyncRequired':
        this.o.onResync();
        return;
      case 'error':
        if (msg.error === 'unauthenticated') {
          // Retrying cannot help until the user signs in again.
          this.state.set('auth_required');
          this.stopped = true;
          this.socket?.close();
        }
        this.o.onError(msg.message);
        return;
      case 'pong':
        return;
    }
  }

  private beat(): void {
    if (Date.now() - this.lastHeard.value > (this.o.deadMs ?? 60_000)) {
      // No answer for too long: the link is dead even if the socket says open.
      this.connect();
      return;
    }
    this.send({ type: 'ping', t: Date.now() });
  }

  private later(): void {
    if (this.stopped) return;
    this.attempts++;
    this.state.set(typeof navigator !== 'undefined' && navigator.onLine === false ? 'offline' : 'reconnecting');
    const base = Math.min(30_000, 1000 * 2 ** Math.min(this.attempts - 1, 5));
    this.retry = setTimeout(() => this.connect(), base / 2 + Math.random() * (base / 2)) as unknown as number;
  }
}

/** `ws(s)://<this host>/v1/ws`. */
export const socketUrl = () => `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/v1/ws`;
