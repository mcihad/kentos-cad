import { describe, expect, it } from 'vitest';
import { Routes } from './router';

describe('the portal’s addresses', () => {
  const routes = new Routes<string>().add('kurum/:slug', 'org').add('kurum/:slug/dosya/:id', 'dossier').add('mesleki/basvurular/yeni', 'apply').add('mesleki/basvurular', 'list');

  it('take their parameters and query', () => {
    const m = routes.match('#/kurum/ornek-belediyesi/dosya/01a1?sekme=yazisma');
    expect(m?.value).toBe('dossier');
    expect(m?.params).toEqual({ slug: 'ornek-belediyesi', id: '01a1' });
    expect(m?.query.get('sekme')).toBe('yazisma');
  });

  it('match whole paths only', () => {
    expect(routes.match('#/mesleki/basvurular/yeni')?.value).toBe('apply');
    expect(routes.match('#/mesleki/basvurular')?.value).toBe('list');
    expect(routes.match('#/kurum')).toBeNull();
    expect(routes.match('#/kurum/a/b')).toBeNull();
  });
});
