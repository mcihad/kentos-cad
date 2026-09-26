import { describe, expect, it } from 'vitest';
import { MessageLog } from '../../app/state';
import { unseenWarnings } from './warnings';

const newest = (log: MessageLog) => log.entries.value.at(-1)?.id ?? 0;

describe('the Uyarılar badge', () => {
  it('counts the warnings and errors logged since the tab was last on screen', () => {
    const log = new MessageLog();
    log.command('Çizgi');
    log.warn('bir');
    log.info('bilgi');
    log.error('iki');
    expect(unseenWarnings(log.entries.value, 0)).toBe(2);
    const seen = newest(log);
    expect(unseenWarnings(log.entries.value, seen)).toBe(0);
    log.success('tamam');
    log.warn('üç');
    expect(unseenWarnings(log.entries.value, seen)).toBe(1);
  });

  it('counts new warnings from zero after the log is cleared from another tab', () => {
    const log = new MessageLog();
    for (let i = 0; i < 5; i++) log.warn(`uyarı ${i}`);
    const seen = newest(log);
    log.clear();
    expect(unseenWarnings(log.entries.value, seen)).toBe(0);
    log.warn('yeni');
    expect(unseenWarnings(log.entries.value, seen)).toBe(1);
  });

  it('still counts a new warning after the log has dropped the old ones it saw', () => {
    const log = new MessageLog();
    for (let i = 0; i < 10; i++) log.warn(`uyarı ${i}`);
    const seen = newest(log);
    // The log keeps 500 entries: these push every warning seen out of it.
    for (let i = 0; i < 500; i++) log.info(`bilgi ${i}`);
    log.warn('yeni');
    expect(log.entries.value).toHaveLength(500);
    expect(unseenWarnings(log.entries.value, seen)).toBe(1);
  });
});
