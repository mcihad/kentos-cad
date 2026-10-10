import { CommandRegistry } from '../../core/commands';
import { Emitter } from '../../core/emitter';
import { Keymap } from '../../core/keymap';
import { Signal } from '../../core/signal';
import { MessageLog } from '../state';
import type { AppContext } from '../context';

/**
 * For tests only: the parts of the app the sheet service and its commands
 * use, in memory. `account` signs someone in (with a personal space,
 * `kisisel-<id>`, as the cloud's fake names it) and makes the server answer.
 */
export function fakeApp(o: { account?: { id: string; name: string } } = {}) {
  const events = new Emitter<{ reset: undefined; touched: unknown }>();
  const layerEvents = new Emitter<{ state: unknown; structure: unknown }>();
  const crs = new Signal({ srid: 5256, name: 'TUREF / TM36', datum: 'TUREF', projection: 'Transverse Mercator', centralMeridian: 36, scaleFactor: 1, falseEasting: 500_000, falseNorthing: 0, ellipsoid: 'GRS80' });
  const doc = {
    events,
    projectId: null as string | null,
    name: new Signal('Yeni çizim'),
    crs,
    // As the project's settings: the same system, no definition of its own (docs/adr/0168 §1).
    settings: { crs, customCrs: new Signal(null), hasSystem: true, workspace: new Signal('cad'), plotScale: new Signal(1000), drawingFont: new Signal('barlow'), variables: new Signal([]) },
    revision: 1,
    layers: { events: layerEvents, leaves: () => [], get: () => undefined, isVisible: () => true, parentOf: () => null },
    byLayer: () => [],
    get: () => undefined,
    all: () => [] as { attrs: Record<string, string> }[],
    origin: { x: 0, y: 0 },
  };
  const me = o.account
    ? {
        user: { id: o.account.id, displayName: o.account.name, method: 'local' },
        memberships: [{ tenantId: `kisisel-${o.account.id}`, tenantSlug: o.account.id, tenantName: o.account.name, tenantKind: 'personal', role: 'owner', seat: true, active: true, capabilities: [] }],
      }
    : null;
  const commands = new CommandRegistry();
  const ctx = {
    commands,
    keymap: new Keymap(commands),
    log: new MessageLog(),
    doc,
    selection: { ids: new Signal(new Set<number>()) },
    cloud: { project: new Signal<null | { tenantId: string; projectId: string; name: string }>(null), me: new Signal<typeof me>(me), auth: new Signal(me ? 'signedIn' : 'signedOut'), api: {} },
    server: { state: new Signal(me ? 'online' : 'offline'), check: async () => (me ? 'online' : 'offline') },
    files: { handle: null as null | { name: string }, picker: { save: async () => undefined, open: async () => undefined } },
    styles: { library: { events: new Emitter<{ changed: unknown }>(), symbol: () => undefined, get: () => undefined } },
    view: {
      camera: { center: { x: 486_780, y: 4_420_080 } },
      inBox: () => [],
      measure: () => ({ length: 0, area: 0 }),
      palette: {},
      geometry: { labels: () => ({ records: new Float64Array(0), texts: [] }), blockPieces: () => null },
      dimensionText: () => '',
    },
  };
  return { ctx, app: ctx as unknown as AppContext };
}
