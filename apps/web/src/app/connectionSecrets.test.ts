import { describe, expect, it } from 'vitest';
import { ConnectionSecrets, SECRETS_KEY, type SecretsStorage } from './connectionSecrets';

/** The device's store of connection secrets (docs/adr/0208 §12): by origin and id, kept apart from any drawing. */

function memory(start: Record<string, string> = {}): SecretsStorage & { data: Record<string, string> } {
  const data = { ...start };
  return { data, getItem: (k) => data[k] ?? null, setItem: (k, v) => void (data[k] = v) };
}

describe('connection secrets', () => {
  it('keeps a secret under its origin and id, replaces it, and forgets it', () => {
    const storage = memory();
    const s = new ConnectionSecrets(storage);
    expect(s.get('https://atlas.harita.gov.tr', 'hgm')).toBeNull();
    expect(s.put('https://atlas.harita.gov.tr', { id: 'hgm', values: ['a'] })).toBeNull();
    expect(s.put('https://atlas.harita.gov.tr', { id: 'hgm', values: ['b'] })).toBeNull();
    // The same id at another origin is another connection's.
    expect(s.put('https://ornek.org', { id: 'hgm', token: 't' })).toBeNull();
    expect(s.get('https://atlas.harita.gov.tr', 'hgm')).toEqual({ id: 'hgm', values: ['b'] });
    expect(s.get('https://ornek.org', 'hgm')).toEqual({ id: 'hgm', token: 't' });
    // A new store reads what the first wrote.
    expect(new ConnectionSecrets(storage).get('https://atlas.harita.gov.tr', 'hgm')).toEqual({ id: 'hgm', values: ['b'] });
    const before = s.version.value;
    expect(s.remove('https://ornek.org', 'hgm')).toBeNull();
    expect(s.version.value).toBe(before + 1);
    expect(s.get('https://ornek.org', 'hgm')).toBeNull();
    // Forgetting what is not there changes nothing.
    expect(s.remove('https://ornek.org', 'hgm')).toBeNull();
    expect(s.version.value).toBe(before + 1);
  });

  it('sets a store it cannot read aside instead of writing over it', () => {
    const storage = memory({ [SECRETS_KEY]: '{"format":"başka"' });
    const s = new ConnectionSecrets(storage);
    expect(s.get('https://ornek.org', 'x')).toBeNull();
    const aside = Object.keys(storage.data).filter((k) => k.startsWith(`${SECRETS_KEY}#unreadable-`));
    expect(aside).toHaveLength(1);
    expect(storage.data[aside[0]]).toBe('{"format":"başka"');
  });

  it('works without storage, for this session only', () => {
    const s = new ConnectionSecrets(null);
    expect(s.put('https://ornek.org', { id: 'x', token: 't' })).toBeNull();
    expect(s.get('https://ornek.org', 'x')).toEqual({ id: 'x', token: 't' });
  });

  it('says why when the browser will not keep it', () => {
    const s = new ConnectionSecrets({
      getItem: () => null,
      setItem: () => {
        throw new Error('QuotaExceededError');
      },
    });
    expect(s.put('https://ornek.org', { id: 'x', token: 't' })).toContain('yalnız bu oturumda');
    expect(s.get('https://ornek.org', 'x')).toEqual({ id: 'x', token: 't' });
  });
});
