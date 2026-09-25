import { describe, expect, it } from 'vitest';
import { chordFromEvent, isAltGrText } from './keymap';

/** The fields the keymap reads from a KeyboardEvent. */
const key = (k: string, code: string, mods: { ctrl?: boolean; alt?: boolean; shift?: boolean } = {}) =>
  ({ key: k, code, ctrlKey: !!mods.ctrl, altKey: !!mods.alt, shiftKey: !!mods.shift, metaKey: false, getModifierState: () => false }) as unknown as KeyboardEvent;

describe('chords follow the produced character on US and Turkish keyboards (docs/adr/0018)', () => {
  it('reads + the same whatever key produces it', () => {
    expect(chordFromEvent(key('+', 'NumpadAdd'))).toBe('+');
    expect(chordFromEvent(key('+', 'Equal', { shift: true }))).toBe('+'); // US
    expect(chordFromEvent(key('+', 'Digit4', { shift: true }))).toBe('+'); // Turkish Q
  });

  it('reads - on the Turkish Q key right of *', () => {
    expect(chordFromEvent(key('-', 'Equal'))).toBe('-');
    expect(chordFromEvent(key('-', 'Minus'))).toBe('-');
  });

  it('takes @ typed with AltGr (Ctrl+Alt on Windows) as text, not a chord', () => {
    const at = key('@', 'KeyQ', { ctrl: true, alt: true });
    expect(isAltGrText(at)).toBe(true);
    expect(chordFromEvent(at)).toBeNull();
  });

  it('keeps Ctrl+Alt with a letter a chord', () => {
    const n = key('n', 'KeyN', { ctrl: true, alt: true });
    expect(isAltGrText(n)).toBe(false);
    expect(chordFromEvent(n)).toBe('Ctrl+Alt+N');
  });

  it('keeps using the key position when a modifier turns a digit into a symbol', () => {
    expect(chordFromEvent(key('!', 'Digit1', { ctrl: true, shift: true }))).toBe('Ctrl+Shift+1');
  });

  it('reads the dotless ı as I', () => {
    expect(chordFromEvent(key('ı', 'KeyI'))).toBe('I');
  });
});
