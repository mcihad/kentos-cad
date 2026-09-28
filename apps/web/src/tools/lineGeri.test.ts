import { describe, expect, it } from 'vitest';
import type { AppContext } from '../app/context';
import { Formatter } from '../app/format';
import { MessageLog } from '../app/state';
import { Signal } from '../core/signal';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { Selection } from '../model/selection';
import { LineTool } from './drawTools';

/**
 * Çizgi's Geri (G) once the drawing changed after the chain's last line was
 * written: the line is deleted through cad.entities.delete, which keeps one
 * on a layer locked since; the chain's earlier lines are then deleted too,
 * never taken back as an undo (the deletion sits above them in the history).
 * Typed input only, over a document and a log, without a view.
 */
function harness() {
  const doc = new CadDocument({
    name: 'Deneme',
    layers: new LayerStore(
      [
        { id: 'cizim', name: 'Çizim' },
        { id: 'yol', name: 'Yol' },
      ],
      'cizim',
    ),
    origin: { x: 0, y: 0 },
  });
  const log = new MessageLog();
  const ctx = {
    doc,
    log,
    selection: new Selection(),
    format: new Formatter({ lengthDecimals: new Signal(3), areaDecimals: new Signal(2), areaUnit: new Signal('m2' as const), angleUnit: new Signal('grad' as const) }),
    settings: { color: new Signal<string | null>(null), lineWeight: new Signal<number | null>(null) },
    view: { requestOverlay: () => {} },
    tools: { exit: () => {} },
  } as unknown as AppContext;
  const said = () => log.entries.value.map((e) => e.text);
  return { ctx, doc, said };
}

/** A chain of two lines, (0,0)–(10,0)–(20,0). */
function chain(ctx: AppContext): LineTool {
  const tool = new LineTool(ctx);
  tool.activate();
  for (const p of ['0,0', '10,0', '20,0']) expect(tool.input(p)).toBe(true);
  return tool;
}

describe('Çizgi: Geri (G) after the drawing changed', () => {
  it('deletes the last line; later, G takes a new line back and deletes the earlier ones, never bringing the deleted line back', () => {
    const { ctx, doc } = harness();
    const tool = chain(ctx);
    expect(doc.size).toBe(2);
    // Another change to the drawing: the chain's lines can no longer be taken back as an undo.
    doc.layers.toggleVisible('yol');
    tool.input('G');
    expect([doc.size, tool.pointCount]).toEqual([1, 2]);
    tool.input('30,0');
    expect(doc.size).toBe(2);
    // The new line is the newest step: taken back as an undo.
    tool.input('G');
    expect([doc.size, tool.pointCount]).toEqual([1, 2]);
    // The first line sits under the deletion: deleted too. Before, the deletion was undone instead and
    // the second line came back.
    tool.input('G');
    expect([doc.size, tool.pointCount]).toEqual([0, 1]);
    // The drawing's own undo brings the deleted lines back, newest first, one step each (“Sil”).
    expect(doc.undo()).toBe('Sil');
    expect(doc.size).toBe(1);
  });

  it('keeps a line on a layer locked since, and the chain with it, saying why', () => {
    const { ctx, doc, said } = harness();
    const tool = chain(ctx);
    doc.layers.toggleLocked('cizim');
    tool.input('G');
    // Before, the line was deleted from the locked layer.
    expect([doc.size, tool.pointCount]).toEqual([2, 3]);
    expect(said().at(-1)).toBe('1 nesne kilitli katmanda olduğu için silinmedi. Silmek için katmanın kilidini Katmanlar panelinden açın.');
  });
});
