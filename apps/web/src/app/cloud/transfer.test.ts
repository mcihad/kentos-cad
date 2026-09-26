import { describe, expect, it } from 'vitest';
import { ApiFailure } from './api';
import { importUnavailable } from './importing';
import { FILE_KEY, commitEnvelope, fileConflict, sha256Hex, sizeText, uploadBytes, uploadGone, verifyDownload } from './transfer';
import type { CloudApi } from './api';
import { snapshotSampleDocument } from '../../model/snapshotSample';
import { bytesOf, serverFor } from './cloudTesting';
import { formatsBuilt } from '../../io/testFormats';

/**
 * Moving a `.kcad` between the browser and a cloud project
 * (docs/adr/0031, 0036, 0038): the hash, sizes as they are said, an upload
 * sent again after a cut connection, asked before its bytes go twice, or
 * opened again when it expired, a download checked against the server's
 * hash, the commit's envelope, and telling a server without the import
 * from a real refusal.
 */

const target = { tenantId: 't', projectId: 'p' };

/** The fake server keeping a file project, and the sample drawing's bytes (large enough for several parts). */
function parts() {
  const doc = snapshotSampleDocument();
  const server = serverFor(doc);
  server.files.storage = 'file';
  const seen = new Set<string>();
  /** The server, with `before` told each part's offset before it goes. */
  const around = (before: (offset: number | undefined) => unknown) =>
    Object.assign(Object.create(server) as CloudApi, {
      sendUpload: (...a: Parameters<CloudApi['sendUpload']>) => {
        before(a[6]);
        return server.sendUpload(...a);
      },
    });
  return { server, seen, around, bytes: () => bytesOf(doc) };
}

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

  it('asks an upload whose answer was lost before sending its bytes again (docs/adr/0040)', async () => {
    const calls: string[] = [];
    let received = false;
    const view = () => ({ id: 'u1', size: 3, sha256: 'x', createdAt: '', expiresAt: '', received, objects: '4' });
    const api = {
      beginUpload: async () => view(),
      sendUpload: async () => {
        calls.push('send');
        // The server refuses bytes it already has.
        if (received) throw new ApiFailure(422, { error: 'invalid', message: 'Bu yüklemenin baytları zaten alındı.' }, 'x');
        received = true;
        throw new ApiFailure(0, { error: 'network' }, 'yanıt gelmedi');
      },
      uploadState: async () => (calls.push('state'), view()),
    } as unknown as CloudApi;
    const up = await uploadBytes(api, { tenantId: 't', projectId: 'p' }, new Uint8Array(3), 'x', { waits: [1, 1] });
    expect([up.received, up.objects, calls]).toEqual([true, '4', ['send', 'state']]);
    // A server without the route (405): the bytes go again, and that send says what became of them.
    let sends = 0;
    const older = {
      beginUpload: async () => view(),
      sendUpload: async () => {
        if (++sends === 1) throw new ApiFailure(0, { error: 'network' }, 'kesildi');
        return { ...view(), received: true };
      },
      uploadState: async () => Promise.reject(new ApiFailure(405, {}, 'Sunucu 405 yanıtı verdi.')),
    } as unknown as CloudApi;
    expect([(await uploadBytes(older, { tenantId: 't', projectId: 'p' }, new Uint8Array(3), 'x', { waits: [1] })).received, sends]).toEqual([true, 2]);
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

describe.skipIf(!formatsBuilt)('an upload in parts (docs/adr/0045)', () => {
  it('a large file goes in parts, each from the bytes the server holds; a small one goes whole (docs/adr/0045)', async () => {
    const s = parts();
    const bytes = await s.bytes();
    const up = await uploadBytes(s.server, target, bytes, await sha256Hex(bytes), { part: 256, waits: [1, 1, 1] });
    const offsets = Array.from({ length: Math.ceil(bytes.byteLength / 256) }, (_, i) => i * 256);
    expect([up.received, s.server.files.offsets]).toEqual([true, offsets]);
    s.server.files.offsets.length = 0;
    const whole = await uploadBytes(s.server, target, bytes, await sha256Hex(bytes), { part: bytes.byteLength, waits: [1] });
    expect([whole.received, s.server.files.offsets]).toEqual([true, [null]]);
  });

  it('a cut part goes again from what arrived; a part whose answer was lost does not go twice', async () => {
    const s = parts();
    const bytes = await s.bytes();
    // The second part is cut on its way (nothing of it kept), the third arrives but its answer is lost.
    const api = s.around((offset) => {
      if (offset === 256 && !s.seen.has('cut')) (s.seen.add('cut'), (s.server.files.cutNextSend = 1));
      if (offset === 512 && !s.seen.has('lost')) (s.seen.add('lost'), (s.server.files.loseNextSendAnswer = true));
    });
    const up = await uploadBytes(api, target, bytes, await sha256Hex(bytes), { part: 256, waits: [1, 1, 1] });
    const rest = Array.from({ length: Math.ceil(bytes.byteLength / 256) - 3 }, (_, i) => 768 + i * 256);
    expect([up.received, s.server.files.offsets]).toEqual([true, [0, 256, 256, 512, ...rest]]);
  });

  it('the last part’s answer lost: the upload is asked and found verified; a part out of step goes on from the server’s count', async () => {
    const s = parts();
    const bytes = await s.bytes();
    const last = Math.floor((bytes.byteLength - 1) / 256) * 256;
    const lostLast = s.around((offset) => offset === last && (s.server.files.loseNextSendAnswer = true));
    const up = await uploadBytes(lostLast, target, bytes, await sha256Hex(bytes), { part: 256, waits: [1, 1] });
    expect([up.received, s.server.files.offsets.filter((o) => o === last).length]).toEqual([true, 1]);
    // No answer about it the first time (the status also unanswered): the same part goes again, is refused as out of step, and the count is asked.
    const t = parts();
    const tBytes = await t.bytes();
    let statusCalls = 0;
    const once = Object.assign(Object.create(t.around((offset) => offset === 256 && !t.seen.has('lost') && (t.seen.add('lost'), (t.server.files.loseNextSendAnswer = true)))) as CloudApi, {
      uploadState: async (tenant: string, project: string, upload: string) => {
        if (++statusCalls === 1) throw new ApiFailure(0, { error: 'network' }, 'yok');
        return t.server.uploadState(tenant, project, upload);
      },
    });
    const done = await uploadBytes(once, target, tBytes, await sha256Hex(tBytes), { part: 256, waits: [1, 1, 1] });
    expect([done.received, t.server.files.offsets.slice(0, 4)]).toEqual([true, [0, 256, 256, 512]]);
  });
});
