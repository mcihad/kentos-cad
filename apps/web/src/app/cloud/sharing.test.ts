import { describe, expect, it } from 'vitest';
import type { ProjectAccessHolder } from '../../contracts/generated/ProjectAccessHolder';
import type { ProjectAccessList } from '../../contracts/generated/ProjectAccessList';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { ApiFailure } from './api';
import { dateText, endOfDay, failureText, personRows, sharedWithMe, sourceText } from './sharing';

const person = (p: Partial<ProjectAccessHolder> & Pick<ProjectAccessHolder, 'userId' | 'displayName'>): ProjectAccessHolder => ({ expired: false, guest: false, ...p });

/** The owner, an admin through the policy (with a grant too), the caller (a manager), an editor with an end, an ended grant, a member without a seat. */
const list: ProjectAccessList = {
  tenantKind: 'organization',
  storage: 'database',
  adminsAccessAllProjects: true,
  ownerId: 'ayse',
  ownerName: 'Ayşe Yılmaz',
  people: [
    person({ userId: 'ayse', displayName: 'Ayşe Yılmaz', role: 'owner', via: 'owner' }),
    person({ userId: 'zeynep', displayName: 'Zeynep Kaya', role: 'manager', via: 'policy', grant: 'viewer' }),
    person({ userId: 'dilek', displayName: 'Dilek Er', role: 'manager', via: 'grant', grant: 'manager' }),
    person({ userId: 'mehmet', displayName: 'Mehmet Demir', email: 'mehmet@ornek.com.tr', role: 'editor', via: 'grant', grant: 'editor', expiresAt: '2026-12-31T20:59:59Z' }),
    person({ userId: 'bora', displayName: 'Bora Tan', blocked: 'expired', grant: 'viewer', expiresAt: '2026-01-01T00:00:00Z', expired: true }),
    person({ userId: 'can', displayName: 'Çağla Öz', blocked: 'noSeat', grant: 'editor' }),
    person({ userId: 'irem', displayName: 'İrem Işık', role: 'manager', via: 'policy' }),
  ],
};

describe('the share dialog’s list', () => {
  it('puts the owner first, then everyone by name in Turkish order', () => {
    const rows = personRows(list, 'dilek', true);
    expect(rows.map((r) => r.name)).toEqual(['Ayşe Yılmaz', 'Bora Tan', 'Çağla Öz', 'Dilek Er', 'İrem Işık', 'Mehmet Demir', 'Zeynep Kaya']);
  });

  it('offers a change only for someone else’s live grant, and never for the owner, the caller or the policy', () => {
    const rows = new Map(personRows(list, 'dilek', true).map((r) => [r.userId, r]));
    const can = (id: string) => [rows.get(id)!.canChange, rows.get(id)!.canRevoke];
    expect(can('ayse')).toEqual([false, false]);
    // Her own access does not change here.
    expect(can('dilek')).toEqual([false, false]);
    expect(rows.get('dilek')!.you).toBe(true);
    expect(can('mehmet')).toEqual([true, true]);
    // The policy is not a grant: its role stays; the grant beside it can still be taken away.
    expect(can('zeynep')).toEqual([false, true]);
    expect(can('irem')).toEqual([false, false]);
    // An ended grant is shared again rather than changed (a change would give it back without an end); it can be cleared.
    expect(can('bora')).toEqual([false, true]);
    expect(can('can')).toEqual([false, true]);
    // Whoever may not share gets no controls at all.
    expect(personRows(list, 'mehmet', false).some((r) => r.canChange || r.canRevoke)).toBe(false);
  });

  it('says the role now, where it comes from, or why not', () => {
    const rows = new Map(personRows(list, 'dilek', true).map((r) => [r.userId, r]));
    expect([rows.get('ayse')!.roleLabel, rows.get('ayse')!.source]).toEqual(['Sahip', 'Proje sahibi']);
    expect([rows.get('zeynep')!.roleLabel, rows.get('zeynep')!.source]).toEqual(['Yönetici', 'Kurum politikası: kurum yöneticisi · paylaşım: Görüntüleyici']);
    expect(rows.get('mehmet')!.source).toBe(`Paylaşım, ${dateText('2026-12-31T20:59:59Z')} tarihine kadar`);
    expect([rows.get('bora')!.roleLabel, rows.get('bora')!.blocked]).toEqual(['Erişemiyor', true]);
    expect(rows.get('bora')!.source).toMatch(/^Paylaşımın süresi doldu · paylaşım: Görüntüleyici, /);
    expect(rows.get('can')!.source).toBe('Kurumda koltuğu yok · paylaşım: Düzenleyici');
    expect(sourceText(person({ userId: 'x', displayName: 'X', role: 'viewer', via: 'grant', grant: 'viewer' }))).toBe('Paylaşım');
    expect(sourceText(person({ userId: 'x', displayName: 'X', blocked: 'inactive', grant: 'viewer' }))).toBe('Hesabı ya da kurum üyeliği etkin değil · paylaşım: Görüntüleyici');
    // A guest (docs/adr/0035): outside the organisation through an invitation; blocked while it takes no guests.
    expect(sourceText(person({ userId: 'x', displayName: 'X', role: 'editor', via: 'grant', grant: 'editor', guest: true }))).toBe('Misafir (davetle)');
    expect(sourceText(person({ userId: 'x', displayName: 'X', blocked: 'guestsOff', grant: 'editor', guest: true }))).toBe('Misafir; kurum dışarıdan misafir kabul etmiyor · paylaşım: Düzenleyici');
    // An owner who left the organisation comes without a name.
    expect(personRows({ ...list, people: [person({ userId: 'ayse', displayName: '', blocked: 'notMember' })] }, 'dilek', true)[0].name).toBe('Adı görünmeyen hesap');
  });
});

describe('“Benimle paylaşılanlar”', () => {
  const summary = (id: string, via: 'owner' | 'grant' | 'policy', updatedAt: string): ProjectSummary => ({
    id,
    name: id,
    srid: 5256,
    dataRevision: '1',
    updatedAt,
    tenantId: 't',
    tenantName: 'Büro',
    tenantKind: 'organization',
    ownerName: 'Ayşe Yılmaz',
    access: { role: via === 'owner' ? 'owner' : via === 'policy' ? 'manager' : 'viewer', via, permissions: ['project.read'] },
    projectType: 'cad',
    description: '',
    tags: [],
    state: 'active',
    catalogVersion: '1',
    createdAt: updatedAt,
    creatorName: 'Ayşe Yılmaz',
    areaUnit: 'm2',
    storage: 'database',
    favorite: false,
  });

  it('holds the projects shared with the account, newest first; its own and the policy’s stay in their lists', () => {
    const shared = sharedWithMe({
      projects: [
        summary('kendi', 'owner', '2026-09-25T10:00:00Z'),
        summary('eski', 'grant', '2026-09-20T10:00:00Z'),
        summary('politika', 'policy', '2026-09-25T11:00:00Z'),
        summary('yeni', 'grant', '2026-09-25T09:00:00Z'),
      ],
    });
    expect(shared.map((p) => p.id)).toEqual(['yeni', 'eski']);
  });
});

describe('the share dialog’s words', () => {
  it('turns a picked day into the end of that day, and refuses nonsense', () => {
    const end = endOfDay('2026-12-31')!;
    const local = new Date(end);
    expect([local.getFullYear(), local.getMonth(), local.getDate(), local.getHours(), local.getMinutes()]).toEqual([2026, 11, 31, 23, 59]);
    for (const bad of ['', '31.12.2026', '2026-13-45', 'yarın']) expect(endOfDay(bad)).toBeNull();
  });

  it('says what failed, why, and what to do', () => {
    const failure = (status: number, error: string, message: string) => new ApiFailure(status, { error, message }, message);
    expect(failureText(new ApiFailure(0, { error: 'network' }, 'x'), 'Paylaşılamadı')).toBe('Paylaşılamadı: sunucuya ulaşılamadı. Bağlantınızı denetleyip yeniden deneyin.');
    expect(failureText(failure(401, 'unauthenticated', 'Oturum yok.'), 'Paylaşılamadı')).toMatch(/oturumunuz sona erdi\. Yeniden giriş/);
    expect(failureText(failure(404, 'not_found', 'Proje bulunamadı.'), 'Erişim listesi okunamadı')).toMatch(/^Erişim listesi okunamadı: proje bulunamadı; .*listesini yenileyin\.$/);
    // The server's own reason, with its fix, as it said it.
    const refused = 'Bu kişi “Büro” kurumunun üyesi değil. Kurum projeleri yalnız kurum üyeleriyle paylaşılır; kurum dışından paylaşım henüz yok.';
    expect(failureText(failure(422, 'invalid', refused), 'Paylaşılamadı')).toBe(`Paylaşılamadı: ${refused}`);
    expect(failureText(failure(503, 'unavailable', 'x'), 'Rol değiştirilemedi')).toMatch(/Birazdan yeniden deneyin/);
    expect(failureText(new Error('bozuk'), 'Paylaşılamadı')).toBe('Paylaşılamadı: bozuk');
  });
});
