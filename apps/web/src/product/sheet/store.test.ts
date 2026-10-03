import { describe, expect, it } from 'vitest';
import { AssetStore, ASSETS, BOOK_FORMAT, BOOK_VERSION, BOOKS, BookStore, deviceKeyValue, fromBase64, keptKey, MemoryKeyValue, peekSheets, readStoredBook, sha256Hex, TEMPLATES, toBase64 } from './store';
import { DeviceTemplateStore, TEMPLATE_FORMAT } from './templateStore';

/**
 * The sheets kept on this device (docs/sheet/design.md §10, §12): a project's
 * book by its key and the user's templates, in a fake store standing for
 * IndexedDB `kentos.sheets.v1`. What is stored is checked, not trusted; what
 * cannot be read is kept aside before anything replaces it, never lost.
 */
describe('sheet book store', () => {
  const book = { schema: 'kentos.sheet/1', sheets: [{ id: 's1', name: 'Pafta 1' }], masters: [], assets: [], variables: [] };

  it('keeps a book under its project and reads it back as it was', async () => {
    const kv = new MemoryKeyValue();
    const store = new BookStore(kv, () => 1000);
    expect(await store.load('proje/a')).toEqual({ status: 'none' });
    await store.save('proje/a', book);
    const read = await store.load('proje/a');
    expect(read).toEqual({ status: 'ok', record: { format: BOOK_FORMAT, version: BOOK_VERSION, key: 'proje/a', saved: 1000, book } });
  });

  it('checks what it reads: format, version, key, time and content', () => {
    const ok = { format: BOOK_FORMAT, version: BOOK_VERSION, key: 'k', saved: 1, book: {} };
    expect(readStoredBook(null, 'k').status).toBe('none');
    expect(readStoredBook({ ...ok, format: 'x' }, 'k')).toMatchObject({ status: 'unreadable', reason: 'kayıt bir pafta kitabı değil' });
    expect(readStoredBook({ ...ok, version: 2 }, 'k')).toMatchObject({ status: 'unreadable', reason: 'kayıt bu sürümün bilmediği 2. biçimde' });
    expect(readStoredBook({ ...ok, key: 'other' }, 'k')).toMatchObject({ status: 'unreadable' });
    expect(readStoredBook({ ...ok, saved: 'dün' }, 'k')).toMatchObject({ status: 'unreadable' });
    expect(readStoredBook({ ...ok, book: [] }, 'k')).toMatchObject({ status: 'unreadable' });
    expect(readStoredBook(ok, 'k').status).toBe('ok');
  });

  it('keeps a record it cannot read aside before writing over its key', async () => {
    const kv = new MemoryKeyValue();
    const newer = { format: BOOK_FORMAT, version: 9, key: 'proje/a', saved: 5, book: { future: true } };
    await kv.put(BOOKS, 'proje/a', newer);
    const store = new BookStore(kv, () => 2000);
    expect((await store.load('proje/a')).status).toBe('unreadable');
    await store.save('proje/a', book);
    expect(await kv.get(BOOKS, keptKey('proje/a', 2000))).toEqual(newer);
    expect((await store.load('proje/a')).status).toBe('ok');
  });

  it('moves a session’s book to the drawing’s lasting key, and never over one that is there', async () => {
    const kv = new MemoryKeyValue();
    const store = new BookStore(kv, () => 1);
    await store.save('oturum/x', book);
    expect(await store.move('oturum/x', 'dosya/a.kcad')).toBe('moved');
    expect((await store.load('oturum/x')).status).toBe('none');
    expect((await store.load('dosya/a.kcad')).status).toBe('ok');
    await store.save('oturum/y', { ...book, sheets: [] });
    expect(await store.move('oturum/y', 'dosya/a.kcad')).toBe('kept');
    expect((await store.load('oturum/y')).status).toBe('ok');
    expect(await store.move('oturum/none', 'dosya/b.kcad')).toBe('none');
  });

  it('copies a book to another lasting key that has none, the old key keeping its own', async () => {
    const kv = new MemoryKeyValue();
    const store = new BookStore(kv, () => 1);
    await store.save('dosya/a.kcad', book);
    expect(await store.copy('dosya/a.kcad', 'bulut/t/p')).toBe('copied');
    expect((await store.load('dosya/a.kcad')).status).toBe('ok');
    expect((await store.load('bulut/t/p')).status).toBe('ok');
    expect(await store.copy('dosya/a.kcad', 'bulut/t/p')).toBe('kept');
  });

  it('copies values in and out, as IndexedDB does', async () => {
    const kv = new MemoryKeyValue();
    const v = { a: [1] };
    await kv.put(BOOKS, 'k', v);
    v.a.push(2);
    const back = (await kv.get(BOOKS, 'k')) as { a: number[] };
    expect(back.a).toEqual([1]);
    back.a.push(3);
    expect(((await kv.get(BOOKS, 'k')) as { a: number[] }).a).toEqual([1]);
  });

  it('uses memory, and says so, where the browser has no IndexedDB', () => {
    expect(deviceKeyValue(undefined).durable).toBe(false);
  });
});

describe('device template store', () => {
  it('lists the templates on this device, one it cannot read as such (never dropped)', async () => {
    const kv = new MemoryKeyValue();
    const store = new DeviceTemplateStore(kv, () => 7);
    await store.save('dev:a', { schema: 'kentos.sheet.template/1' });
    await kv.put(TEMPLATES, 'dev:b', { format: 'eski' });
    const list = await store.list();
    expect(list.map((r) => r.status)).toEqual(['ok', 'unreadable']);
    expect(list[0]).toMatchObject({ status: 'ok', record: { format: TEMPLATE_FORMAT, id: 'dev:a', saved: 7 } });
    // Written over, the unreadable one is kept aside, and the aside copy is not listed.
    await store.save('dev:b', { schema: 'kentos.sheet.template/1' }, { account: 'u1', revision: 3, changed: false, role: 'owner' });
    expect(await kv.get(TEMPLATES, keptKey('dev:b', 7))).toEqual({ format: 'eski' });
    const again = await store.list();
    expect(again.map((r) => r.status)).toEqual(['ok', 'ok']);
    expect(again[1]).toMatchObject({ record: { cloud: { account: 'u1', revision: 3, changed: false, role: 'owner' } } });
    expect(await store.get('dev:b')).toMatchObject({ status: 'ok', record: { id: 'dev:b' } });
    expect(await store.get('yok')).toBeNull();
    await store.remove('dev:a');
    expect((await store.list()).length).toBe(1);
  });

  it('reads a record’s place in a cloud library, and leaves one it cannot read on this device only', async () => {
    const kv = new MemoryKeyValue();
    const store = new DeviceTemplateStore(kv, () => 9);
    const t = { schema: 'kentos.sheet.template/1' };
    await kv.put(TEMPLATES, 'c1', { format: TEMPLATE_FORMAT, version: 1, id: 'c1', saved: 1, template: t, cloud: { account: 'u1', revision: 4, changed: true, role: 'editor', ownerName: 'Ayşe', shared: false, deleted: false, formerIds: ['eski', 3], conflictOf: 'c0' } });
    // No account or an unknown role: the cloud link is dropped, the template stays.
    await kv.put(TEMPLATES, 'c2', { format: TEMPLATE_FORMAT, version: 1, id: 'c2', saved: 1, template: t, cloud: { revision: 2, changed: false } });
    await kv.put(TEMPLATES, 'c3', { format: TEMPLATE_FORMAT, version: 1, id: 'c3', saved: 1, template: t, cloud: { account: 'u1', revision: 2, changed: false, role: 'yazar' } });
    // An organisation's administrator (KURUM) is a role this store knows.
    await kv.put(TEMPLATES, 'c4', { format: TEMPLATE_FORMAT, version: 1, id: 'c4', saved: 1, template: t, cloud: { account: 'u1', revision: 2, changed: false, role: 'admin', organization: { tenantId: 'buro', name: 'Büro' } } });
    const read = await store.list();
    expect(read[0]).toMatchObject({ status: 'ok', record: { cloud: { account: 'u1', revision: 4, changed: true, role: 'editor', ownerName: 'Ayşe', formerIds: ['eski'], conflictOf: 'c0' } } });
    expect(read[0].status === 'ok' && read[0].record.cloud?.deleted).toBeUndefined();
    expect(read.slice(1, 3).map((r) => r.status === 'ok' && r.record.cloud)).toEqual([undefined, undefined]);
    expect(read[3]).toMatchObject({ status: 'ok', record: { cloud: { role: 'admin', organization: { tenantId: 'buro', name: 'Büro' } } } });
  });
});

describe('picture store', () => {
  const bytes = new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"/>');

  it('keeps bytes under their SHA-256 and gives them back; bytes under another digest are refused', async () => {
    const kv = new MemoryKeyValue();
    const store = new AssetStore(kv);
    const sha = await sha256Hex(bytes);
    expect(sha).toMatch(/^[0-9a-f]{64}$/);
    await store.put({ sha256: sha, kind: 'svg', name: 'logo.svg', width: 10, height: 10, bytes: bytes.length }, bytes);
    expect((await store.get(sha))?.bytes).toEqual(bytes);
    expect(await store.digests()).toEqual([sha]);
    await expect(store.put({ sha256: '0'.repeat(64), kind: 'svg', name: 'yanlış.svg', width: 1, height: 1, bytes: 1 }, bytes)).rejects.toThrow('SHA-256');
    expect(await new AssetStore(kv).get('f'.repeat(64))).toBeNull();
    // What is not a picture record under a digest is not taken for one.
    await kv.put(ASSETS, 'a'.repeat(64), { format: 'başka' });
    expect(await new AssetStore(kv).get('a'.repeat(64))).toBeNull();
  });

  it('writes bytes as base64 and reads them back', () => {
    const all = Uint8Array.from({ length: 70_000 }, (_, i) => (i * 31) & 255);
    expect(fromBase64(toBase64(all))).toEqual(all);
    expect(toBase64(new TextEncoder().encode('Pafta'))).toBe('UGFmdGE=');
  });
});

describe('stored book names before the engine', () => {
  it('takes only the sheets’ string ids and names', () => {
    expect(peekSheets({ sheets: [{ id: 's1', name: 'Pafta 1' }, { id: 2, name: 'x' }, { id: 's3' }, null, { id: 's4', name: 'Kroki' }] })).toEqual([
      { id: 's1', name: 'Pafta 1' },
      { id: 's4', name: 'Kroki' },
    ]);
    expect(peekSheets(null)).toEqual([]);
    expect(peekSheets({ sheets: 'hayır' })).toEqual([]);
  });
});
