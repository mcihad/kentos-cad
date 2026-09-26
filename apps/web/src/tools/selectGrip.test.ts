import { describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import { Formatter } from '../app/format';
import { MessageLog } from '../app/state';
import { Signal } from '../core/signal';
import { CadDocument } from '../model/document';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { LayerStore } from '../model/layers';
import { entityGrips } from '../model/ops/grips';
import { Selection } from '../model/selection';
import { SelectTool } from './SelectTool';
import { takesTypedInput, type ToolPointer } from './Tool';

/**
 * A grip clicked without dragging waits for its new place, and its prompt
 * asks for a coordinate: typed text then goes to the select tool (the command
 * line and the field beside the cursor ask `takesTypedInput`), not to the
 * command list.
 */
function harness() {
  const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'cizim', name: 'Çizim' }], 'cizim'), origin: { x: 0, y: 0 } });
  const line = doc.add({ kind: 'line', layerId: 'cizim', attrs: {}, a: { x: -20, y: -8 }, b: { x: -8, y: -8 } } as Entity);
  const end = entityGrips(line).findIndex((g) => g.x === -8 && g.y === -8);
  const ctx = {
    doc,
    log: new MessageLog(),
    selection: new Selection(),
    format: new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) }),
    settings: { color: new Signal<string | null>(null) },
    prefs: { snapAperture: new Signal(8) },
    view: { gripAt: () => ({ id: line.id, index: end }), requestOverlay: () => {}, trackAlong: () => null, camera: { worldToScreen: (p: Vec2) => p } },
    tools: { exit: () => {} },
  } as unknown as AppContext;
  return { ctx, doc, line };
}

const at = (p: Vec2): ToolPointer => ({ world: p, raw: p, screen: p, snap: null, track: null, button: 0, shift: false, ctrl: false, alt: false });

describe('grips take typed coordinates', () => {
  it('a clicked grip waits, takes “@0,4” and moves 4 m north of where it was, in one step', () => {
    const { ctx, doc, line } = harness();
    const tool = new SelectTool(ctx);
    expect(takesTypedInput('select', tool)).toBe(false);
    tool.pointerDown(at({ x: -8, y: -8 }));
    tool.pointerUp(at({ x: -8, y: -8 }));
    expect(tool.prompt.value).toBe('Tutamaç: yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)');
    expect(takesTypedInput('select', tool)).toBe(true);
    expect(tool.input('@0,4')).toBe(true);
    expect(doc.get(line.id)).toMatchObject({ a: { x: -20, y: -8 }, b: { x: -8, y: -4 } });
    expect(takesTypedInput('select', tool)).toBe(false);
    expect(tool.prompt.value).toBe('Komut');
    expect(doc.undo()).toBe('Tutamaçla düzenle');
  });

  it('any other tool takes typed text; selection without a waiting grip does not', () => {
    expect(takesTypedInput('line', {})).toBe(true);
    expect(takesTypedInput('select', {})).toBe(false);
    expect(takesTypedInput('select', { activeGrip: () => null })).toBe(false);
  });
});
