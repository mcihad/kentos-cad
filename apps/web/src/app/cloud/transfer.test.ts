import { describe, expect, it } from 'vitest';
import { ApiFailure } from './api';
import { importUnavailable } from './importing';
import { FILE_KEY, commitEnvelope, fileConflict, sha256Hex, sizeText, uploadBytes, uploadGone, verifyDownload } from './transfer';
import type { CloudApi } from './api';

/**
 * Moving a `.kcad` between the browser and a cloud project
 * (docs/adr/0031, 0036, 0038): the hash, sizes as they are said, an upload
 * sent again after a cut connection or opened again when it expired, a
 * download checked against the server's hash, the commit's envelope, and
 * telling a server without the import from a real refusal.
 */

describe('file transfers of cloud projects', () => {
  it('hashes as the server checks (SHA-256, lowercase hex) and says sizes in Turkish', async () => {
    expect(await sha256Hex(new TextEncoder().encode('abc'))).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
    expect([sizeText(812), sizeText(12_700), sizeText(50_663_220)]).toEqual(['812 bayt', '12,4 KB', '48,3 MB']);
  });

  it('sends a cut-off upload again, and opens a new one when the server lost it', async () => {
    const calls: string[] = [];
    let sends = 0;
    const api = {
      beginUpload: async () => (calls.push('begin'), { id: `u${calls.filter((c) => c === 'begin').length}`, size: 3, sha256: 'x', createdAt: '', expiresAt: '', received: false }),
      sendUpload: async (_t: string, _p: string, id: string) => {
        calls.push(`send ${id}`);
        sends++;
        if (sends === 1) throw new ApiFailure(0, { error: 'network' }, 'kesildi');
        if (sends === 2) throw new ApiFailure(404, { error: 'not_found', message: 'Yükleme bulunamadı: süresi dolmuş.' }, 'yok');
        return { id, size: 3, sha256: 'x', createdAt: '', expiresAt: '', received: true };
      },
    } as unknown as CloudApi;
    const up = await uploadBytes(api, { tenantId: 't', projectId: 'p' }, new Uint8Array(3), 'x', { waits: [1, 1] });
    expect([up.id, calls]).toEqual(['u2', ['begin', 'send u1', 'send u1', 'begin', 'send u2']]);
    // The project's own 404 (access taken away) is not an expired upload: it is thrown.
    const gone = { ...api, sendUpload: async () => Promise.reject(new ApiFailure(404, { error: 'not_found', message: 'Proje bulunamadı.' }, 'yok')) } as unknown as CloudApi;
    const e = (await uploadBytes(gone, { tenantId: 't', projectId: 'p' }, new Uint8Array(3), 'x', { waits: [1] }).catch((x: unknown) => x)) as ApiFailure;
    expect([e.notFound, uploadGone(e)]).toEqual([true, false]);
  });

  it('never takes a download whose bytes are not the server’s', async () => {
    const bytes = new TextEncoder().encode('abc');
    const sha = await sha256Hex(bytes);
    const d = { bytes, sha256: sha, revision: '1', cursor: null, fileName: null };
    await expect(verifyDownload(d)).resolves.toBeUndefined();
    await expect(verifyDownload({ ...d, bytes: new TextEncoder().encode('abd') })).rejects.toThrow(/SHA-256 tutmuyor/);
    // Listed otherwise than the server's own tag: refused too.
    await expect(verifyDownload(d, '0'.repeat(64))).rejects.toThrow(/SHA-256 tutmuyor/);
  });

  it('commits on the revision the file was based on, and reads the @file conflict', () => {
    const e = commitEnvelope({ tenantId: 't', projectId: 'p' }, 'u1', '4');
    expect([e.commandName, e.version, e.expectedVersions, e.input]).toEqual(['project.file.commit', 1, { [FILE_KEY]: '4' }, { uploadId: 'u1' }]);
    const refused = new ApiFailure(409, { error: 'conflict', message: 'x', conflicts: [{ id: '@file', reason: 'project', expected: '4', actual: '5' }] }, 'x');
    expect(fileConflict(refused)).toEqual({ expected: '4', actual: '5' });
    expect(fileConflict(new ApiFailure(409, { error: 'conflict', message: 'x', conflicts: [{ id: 'abc', reason: 'changed' }] }, 'x'))).toBeNull();
  });

  it('tells a server without the import from a refusal', () => {
    const invalid = (message: string, path?: string) => new ApiFailure(422, { error: 'invalid', message, ...(path ? { path } : {}) }, message);
    expect(importUnavailable(invalid('Bilinmeyen komut: project.import'))).toBe(true);
    expect(importUnavailable(invalid('“Ada” projesi nesne nesne veritabanında saklanıyor; dosya yüklenmez. Değişiklikler project.changes ile kaydedilir.'))).toBe(true);
    expect(importUnavailable(new ApiFailure(404, {}, 'Sunucu 404 yanıtı verdi.'))).toBe(true);
    expect(importUnavailable(invalid('Dosyanın 14. nesnesi içe aktarılamadı: sınır dışında.', 'entities[13]'))).toBe(false);
    expect(importUnavailable(new ApiFailure(404, { error: 'not_found', message: 'Proje bulunamadı.' }, 'yok'))).toBe(false);
    expect(importUnavailable(new ApiFailure(0, { error: 'network' }, 'yok'))).toBe(false);
  });
});
