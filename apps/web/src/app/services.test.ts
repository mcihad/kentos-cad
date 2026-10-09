import { describe, expect, it, vi } from 'vitest';
import type { AppContext } from './context';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { PRESETS, type Preset } from '../model/servicePresets';
import { addBasemap, bottomBasemap, removeBasemap } from './services';

/**
 * Hazır altlıklar (docs/adr/0208 §1, §14): a ready basemap goes below everything, the next takes its place, Altlığı
 * kaldır takes it away, each one undo step; one that needs a key this device does not have opens Bağlantılar on its
 * connection. The desktop's `services::basemaps::tests` are the same.
 */
const opened = vi.hoisted(() => [] as unknown[]);
vi.mock('../ui/services/ConnectionsDialog', () => ({
  openConnections: (_ctx: unknown, options: unknown) => void opened.push(options),
}));

function setup(secret: string | null = null) {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(
      [
        { id: 'parsel', name: 'Parsel' },
        { id: 'cizim', name: 'Çizim' },
      ],
      'parsel',
    ),
    origin: { x: 0, y: 0 },
  });
  const said: string[] = [];
  const say = (text: string) => void said.push(text);
  const ctx = { doc, log: { info: say, warn: say, success: say }, secrets: { get: () => secret } } as unknown as AppContext;
  return { ctx, doc, said };
}

function preset(id: string): Preset {
  const p = PRESETS.find((q) => q.id === id);
  if (!p) throw new Error(`no preset ${id}`);
  return p;
}

/** The bottom basemap's name and preset, and the number of top-level nodes. */
function bottom(ctx: AppContext): [[string, string | undefined] | null, number] {
  const b = bottomBasemap(ctx);
  return [b && [b.name, b.service?.preset], ctx.doc.layers.tree.length];
}

describe('Hazır altlıklar', () => {
  it('go below everything, the next takes the place of the one there, each in one step', () => {
    const { ctx, doc, said } = setup();
    expect(bottom(ctx)).toEqual([null, 2]);
    addBasemap(ctx, preset('osm-standard'));
    expect(bottom(ctx)).toEqual([['OSM Standart', 'osm-standard'], 3]);
    expect(said.at(-1)).toBe('“OSM Standart” altlık olarak eklendi.');
    addBasemap(ctx, preset('osm-topo'));
    expect(bottom(ctx), 'in place of the one there was').toEqual([['OSM Topo', 'osm-topo'], 3]);
    expect(said.at(-1)).toBe('Altlık “OSM Topo” oldu.');
    doc.undo();
    expect(bottom(ctx)).toEqual([['OSM Standart', 'osm-standard'], 3]);
    removeBasemap(ctx);
    expect(bottom(ctx)).toEqual([null, 2]);
    expect(said.at(-1)).toBe('“OSM Standart” altlığı kaldırıldı.');
    removeBasemap(ctx);
    expect(said.at(-1)).toBe('Kaldırılacak hazır altlık yok.');
    doc.undo();
    expect(bottom(ctx)).toEqual([['OSM Standart', 'osm-standard'], 3]);
  });

  it('open Bağlantılar on the connection whose key this device does not have', async () => {
    opened.length = 0;
    const { ctx, doc, said } = setup();
    addBasemap(ctx, preset('google-satellite'));
    expect(bottom(ctx)[0]).toEqual(['Google Uydu', 'google-satellite']);
    const connection = doc.settings.connections.value[0];
    expect(connection).toBeDefined();
    expect(said.at(-1)).toContain(`“${connection?.name}” bağlantısının anahtarını girin`);
    await vi.waitFor(() => expect(opened).toEqual([{ focus: connection?.id }]));

    opened.length = 0;
    const keyed = setup('anahtar');
    addBasemap(keyed.ctx, preset('google-satellite'));
    expect(keyed.said).toEqual(['“Google Uydu” altlık olarak eklendi.']);
    await Promise.resolve();
    expect(opened).toEqual([]);
  });
});
