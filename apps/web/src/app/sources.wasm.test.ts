import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/exchange/v1/cases.json?raw';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import type { AppContext } from './context';
import { takeLayerInto } from './drawingExchange';
import { folderListing, type FileHandle, type FolderHandle } from './sources';

/**
 * Kaynaklar on the page (docs/adr/0199 §7): a folder's listing through the browser's handle, and a KentOS project's
 * layer taken with its objects as one undo step (the exchange cases' source drawing, on the desktop's sample tree).
 * The desktop's `sources::tests` are the same.
 */

/** A folder handle over names (a name ending in “/” a folder). */
function folder(name: string, names: readonly string[]): FolderHandle {
  const entries = names.map((n): FolderHandle | FileHandle =>
    n.endsWith('/') ? folder(n.slice(0, -1), []) : { kind: 'file', name: n, getFile: async () => new File(['x'], n) },
  );
  return {
    kind: 'directory',
    name,
    async *values() {
      yield* entries;
    },
    getDirectoryHandle: async (n) => entries.find((e): e is FolderHandle => e.kind === 'directory' && e.name === n)!,
    getFileHandle: async (n) => entries.find((e): e is FileHandle => e.kind === 'file' && e.name === n)!,
  };
}

describe('Kaynaklar: a folder', () => {
  it('lists its folders and the files the panel adds, a Shapefile with its parts', async () => {
    const got = await folderListing(folder('Veri', ['Pafta 2/', 'parsel.shp', 'parsel.dbf', 'parsel.prj', 'yol.dxf', 'noktalar.ncn', 'rapor.pdf', '.gizli.dxf']));
    expect(got).toEqual({
      folders: ['Pafta 2'],
      files: [
        { name: 'noktalar.ncn', kind: 'coords', parts: ['noktalar.ncn'] },
        { name: 'parsel.shp', kind: 'shapefile', parts: ['parsel.shp', 'parsel.dbf', 'parsel.prj'] },
        { name: 'yol.dxf', kind: 'dxf', parts: ['yol.dxf'] },
      ],
    });
  });
});

function setup() {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(
      [
        {
          id: 'layer-g',
          name: 'Kadastro',
          children: [
            { id: 'parsel', name: 'Parsel' },
            { id: 'bina', name: 'Bina', locked: true },
          ],
        },
        { id: 'cizim', name: 'Çizim', visible: false },
      ],
      'parsel',
    ),
    origin: { x: 0, y: 0 },
  });
  doc.add({ kind: 'point', layerId: 'parsel', attrs: {}, p: { x: 1, y: 0 } });
  const said: string[] = [];
  const say = (t: string) => void said.push(t);
  const ctx = { doc, log: { info: say, warn: say, success: say }, cloud: { project: { value: null } } } as unknown as AppContext;
  return { ctx, doc, said };
}

describe('Kaynaklar: a KentOS project’s layer', () => {
  const theirs = JSON.parse(text).drawings.theirs;

  it('comes with its objects, its blocks and the layers they draw on, as one undo step', () => {
    const { ctx, doc, said } = setup();
    const before = doc.size;
    expect(takeLayerInto(ctx, theirs, 'Yol', 'Kaynak')).toBe(true);
    expect(said.at(-1)).toBe('“Kaynak” içinden “Yol” alındı: 2 nesne, 2 yeni katman ya da grup, 2 blok (tek adımda geri alınır).');
    expect(doc.size).toBe(before + 2);
    // Yol, and 0 where the lamp's post (the block Direk inside Lamba) is drawn.
    expect(doc.layers.leaves().map((l) => l.name)).toEqual(['Parsel', 'Bina', 'Çizim', '0', 'Yol']);
    doc.undo();
    expect(doc.size).toBe(before);
    expect(doc.layers.leaves().map((l) => l.name)).toEqual(['Parsel', 'Bina', 'Çizim']);
  });

  it('goes into our layer of the same path, and not into a locked one', () => {
    const { ctx, doc, said } = setup();
    const before = doc.size;
    expect(takeLayerInto(ctx, theirs, 'Kadastro / Parsel', 'Kaynak')).toBe(true);
    expect(doc.size).toBe(before + 5);
    expect(doc.byLayer('parsel').length).toBe(6);
    expect(takeLayerInto(ctx, theirs, 'Kadastro / Bina', 'Kaynak')).toBe(false);
    expect(said.at(-1)).toBe('Katman olarak ekle: “Kadastro / Bina” katmanı kilitli; kilidini açıp yeniden deneyin.');
    expect(takeLayerInto(ctx, theirs, 'Kadastro', 'Kaynak')).toBe(false);
  });
});
