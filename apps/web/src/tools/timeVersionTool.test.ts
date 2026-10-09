import { describe, expect, it } from 'vitest';
import { TimeVersionTool } from './timeVersionTool';
import { toolHarness } from './toolHarness';

/**
 * Yeni sürüm oluştur and Sona erdir (docs/adr/0210 §7) with a date typed: the steps, the words and the values written.
 * The traces (fixtures/interaction/v1/time-version.json) play the same in both apps with the time slider; here a layer
 * whose end field is of the date kind (docs/adr/0199) takes the day alone (§3).
 */
describe('Yeni sürüm oluştur', () => {
  function drawing() {
    const h = toolHarness();
    h.doc.setLayerTime('cizim', { start: 'bas', end: 'bit', key: 'no' }, 'Zaman ayarları');
    const a = h.add({ kind: 'point', p: { x: 0, y: 0 }, attrs: { no: '1', bas: '2010-01-01', bit: '2030-01-01' } });
    const b = h.add({ kind: 'point', p: { x: 5, y: 0 }, attrs: { no: '2', bas: '2012-06-01T08:00:00' } });
    return { h, a, b };
  }

  it('ends the objects at the date typed and writes their new versions, selected, in one step', () => {
    const { h, a, b } = drawing();
    h.ctx.selection.set([a.id, b.id]);
    const tool = h.use(new TimeVersionTool(h.ctx, 'timeVersion'));
    tool.activate();
    expect(tool.input('15.06.2020 14:30')).toBe(true);
    expect(h.doc.get(a.id)!.attrs.bit).toBe('2020-06-15T14:30:00');
    expect(h.doc.get(b.id)!.attrs.bit).toBe('2020-06-15T14:30:00');
    const copies = [...h.ctx.selection.ids.value].map((id) => h.doc.get(id)!);
    expect(copies.map((e) => e.attrs)).toEqual([
      { no: '1', bas: '2020-06-15T14:30:00', bit: '2030-01-01' },
      { no: '2', bas: '2020-06-15T14:30:00' },
    ]);
    expect(h.said().at(-1)).toBe('Yeni sürüm: 2 nesne 15.06.2020 tarihinde sona erdi, 2 yeni sürümü yazıldı ve seçildi.');
    expect(h.doc.undo()).toBe('Yeni sürüm oluştur');
    expect([...h.doc.all()]).toHaveLength(2);
    expect(h.doc.get(a.id)!.attrs.bit).toBe('2030-01-01');
  });

  it('a date field takes the day alone: the date is that day’s midnight', () => {
    const { h, a } = drawing();
    h.doc.setLayerFields('cizim', [
      { name: 'no', kind: 'text' },
      { name: 'bas', kind: 'text' },
      { name: 'bit', kind: 'date' },
    ]);
    h.ctx.selection.set([a.id]);
    const tool = h.use(new TimeVersionTool(h.ctx, 'timeEnd'));
    tool.activate();
    tool.input('2020-06-15T14:30');
    expect(h.doc.get(a.id)!.attrs.bit).toBe('2020-06-15');
    expect(h.said().at(-1)).toBe('Sona erdi: 1 nesne, 15.06.2020.');
  });

  it('refuses a date the objects do not span, and writes nothing', () => {
    const { h, a, b } = drawing();
    h.ctx.selection.set([a.id, b.id]);
    const tool = h.use(new TimeVersionTool(h.ctx, 'timeEnd'));
    tool.activate();
    tool.input('2011-01-01');
    expect(h.said().at(-1)).toBe('1 nesnenin zamanı 01.01.2011 anını içermiyor: başlangıcı bu tarihten önce, bitişi sonra olmalı. Hiçbir nesne yazılmadı.');
    expect(h.doc.get(a.id)!.attrs.bit).toBe('2030-01-01');
    expect(h.doc.get(b.id)!.attrs.bit).toBeUndefined();
    expect(h.doc.undo()).toBe('Ekle');
  });
});
