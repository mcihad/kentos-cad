import { describe, expect, it } from 'vitest';
import { jpegSize, MOST_BYTES, pictureItem, pngSize } from './pictureFile';

/** Resim ekle's file (docs/adr/0192 §2): a PNG's and a JPEG's size from their headers, the library item, the refusals. */

const be32 = (n: number) => [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255];
const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, ...'IHDR'.split('').map((c) => c.charCodeAt(0)), ...be32(160), ...be32(120), 8, 2, 0, 0, 0]);
// SOI, an APP0 of 4 bytes, then SOF0: length 17, precision 8, height 300, width 400.
const JPEG = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0, 4, 0, 0, 0xff, 0xc0, 0, 17, 8, 0x01, 0x2c, 0x01, 0x90, 3]);

describe('pictureFile', () => {
  it('reads the size from the header', () => {
    expect(pngSize(PNG)).toEqual([160, 120]);
    expect(jpegSize(JPEG)).toEqual([400, 300]);
    expect(pngSize(PNG.subarray(0, 20))).toBeNull();
    expect(jpegSize(new Uint8Array([0xff, 0xd8, 0xff, 0xd9]))).toBeNull();
  });

  it('makes the library item, its id from its content', async () => {
    const got = await pictureItem('logo.PNG', PNG);
    expect(got.ok).toBe(true);
    if (!got.ok) return;
    expect(got.id).toMatch(/^resim-[0-9a-f]{16}$/);
    expect(got.item).toMatchObject({ kind: 'asset', id: got.id, name: 'logo', path: ['Resimler'], format: 'png', width: 160, height: 120 });
    expect(got.item.data.startsWith('data:image/png;base64,iVBORw0KGgo')).toBe(true);
    expect((await pictureItem('b.png', PNG)).ok && (await pictureItem('b.png', PNG))).toMatchObject({ id: got.id });
    expect(await pictureItem('foto.jpg', JPEG)).toMatchObject({ ok: true, item: { format: 'jpeg', width: 400, height: 300 } });
  });

  it('says why a file is not taken', async () => {
    expect(await pictureItem('not.gif', new Uint8Array([0x47, 0x49, 0x46]))).toEqual({ ok: false, error: '“not.gif” PNG ya da JPEG değil; resim olarak eklenemez.' });
    expect(await pictureItem('bad.png', PNG.subarray(0, 12))).toEqual({ ok: false, error: '“bad.png” okunamadı: PNG dosyası bozuk görünüyor.' });
    expect(await pictureItem('big.png', new Uint8Array(MOST_BYTES + 1))).toEqual({ ok: false, error: '“big.png” 32 MB; resim en çok 32 MB olabilir.' });
  });
});
