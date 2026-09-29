import { describe, expect, it } from 'vitest';
import { Camera } from './Camera';
import { HISTORY_LIMIT, ViewHistory, ViewNavigation, WHEEL_PAUSE_MS, type ViewState } from './viewHistory';

/** The views of docs/adr/0141's history rules: V(i) looks at (i, 0) from the same distance. */
const V = (i: number): ViewState => ({ center: { x: i, y: 0 }, scale: 1 });
const at = (h: ViewState | null) => h?.center.x ?? null;

describe('view history', () => {
  it('goes back to the view that was left, and forward again', () => {
    const h = new ViewHistory();
    expect(h.canBack.value).toBe(false);
    expect(h.canForward.value).toBe(false);
    h.record(V(0)); // left V0 for V1
    expect(h.canBack.value).toBe(true);
    expect(at(h.back(V(1)))).toBe(0);
    expect(h.canBack.value).toBe(false);
    expect(h.canForward.value).toBe(true);
    expect(at(h.forward(V(0)))).toBe(1);
    expect(h.canForward.value).toBe(false);
    expect(h.canBack.value).toBe(true);
  });

  it('has nothing to go to without a record', () => {
    const h = new ViewHistory();
    expect(h.back(V(0))).toBeNull();
    expect(h.forward(V(0))).toBeNull();
  });

  it('keeps 30 views: after 31 navigations 30 steps back, then 30 forward', () => {
    const h = new ViewHistory();
    // 31 navigations: V(i) is left for V(i + 1), so the view now is V(31).
    for (let i = 0; i < 31; i++) h.record(V(i));
    let now = V(31);
    const seen: (number | null)[] = [];
    for (let i = 0; i < HISTORY_LIMIT; i++) {
      const to = h.back(now);
      seen.push(at(to));
      now = to!;
    }
    // The oldest view (V0) was dropped: V30 … V1.
    expect(seen).toEqual(Array.from({ length: 30 }, (_, i) => 30 - i));
    expect(h.back(now)).toBeNull();
    expect(h.canBack.value).toBe(false);
    const forward: (number | null)[] = [];
    for (let i = 0; i < HISTORY_LIMIT; i++) {
      const to = h.forward(now);
      forward.push(at(to));
      now = to!;
    }
    expect(forward).toEqual(Array.from({ length: 30 }, (_, i) => i + 2));
    expect(at(now)).toBe(31);
    expect(h.forward(now)).toBeNull();
  });

  it('a new navigation empties what was to come', () => {
    const h = new ViewHistory();
    h.record(V(0));
    h.record(V(1));
    expect(at(h.back(V(2)))).toBe(1);
    expect(h.canForward.value).toBe(true);
    // From V1 the user zooms elsewhere: V1 is left, and Sonraki is gone.
    h.record(V(1.5));
    expect(h.canForward.value).toBe(false);
    expect(h.forward(V(9))).toBeNull();
    expect(at(h.back(V(9)))).toBe(1.5);
  });

  it('does not keep the view kept last again', () => {
    const h = new ViewHistory();
    h.record(V(0));
    h.record(V(0));
    h.record(V(1));
    h.record(V(1));
    expect(at(h.back(V(2)))).toBe(1);
    expect(at(h.back(V(1)))).toBe(0);
    expect(h.back(V(0))).toBeNull();
  });

  it('a repeated record leaves what is to come alone', () => {
    const h = new ViewHistory();
    h.record(V(0));
    h.record(V(1));
    h.back(V(2));
    // The view kept last is V0; recording it again is nothing new.
    h.record(V(0));
    expect(h.canForward.value).toBe(true);
  });

  it('passes over a kept view equal to the one now: a step never lands where the user is', () => {
    const h = new ViewHistory();
    h.record(V(0));
    h.record(V(1));
    // The view now is V1 again (nothing moved since it was kept): back goes on to V0.
    expect(at(h.back(V(1)))).toBe(0);
    // And V1 is what Sonraki brings back to.
    expect(at(h.forward(V(0)))).toBe(1);
  });

  it('clear empties both ways', () => {
    const h = new ViewHistory();
    h.record(V(0));
    h.record(V(1));
    h.back(V(2));
    h.clear();
    expect(h.canBack.value).toBe(false);
    expect(h.canForward.value).toBe(false);
    expect(h.back(V(0))).toBeNull();
    expect(h.forward(V(0))).toBeNull();
  });

  it('tells its signals', () => {
    const h = new ViewHistory();
    const seen: boolean[] = [];
    h.canBack.subscribe((v) => seen.push(v));
    h.record(V(0));
    h.record(V(1));
    h.back(V(2));
    h.back(V(1));
    expect(seen).toEqual([true, false]);
  });

  describe('the wheel', () => {
    it('keeps the view at the first step after a pause, and the run of steps is one pass', () => {
      const h = new ViewHistory();
      h.wheel(V(0), 0); // opens a pass
      h.wheel(V(1), 100);
      h.wheel(V(2), 300);
      h.wheel(V(3), 700); // 400 ms after the last step: the same pass
      expect(at(h.back(V(4)))).toBe(0);
      expect(h.back(V(0))).toBeNull();
    });

    it('a pause of 500 ms or more begins the next pass', () => {
      const h = new ViewHistory();
      h.wheel(V(0), 0);
      h.wheel(V(1), 100);
      h.wheel(V(2), 100 + WHEEL_PAUSE_MS - 1); // 499 ms: still the first pass
      h.wheel(V(3), 100 + WHEEL_PAUSE_MS - 1 + WHEEL_PAUSE_MS); // exactly 500 ms after the last step: a new one
      expect(at(h.back(V(4)))).toBe(3);
      expect(at(h.back(V(3)))).toBe(0);
      expect(h.back(V(0))).toBeNull();
    });

    it('another view kept in between ends the pass', () => {
      const h = new ViewHistory();
      h.wheel(V(0), 0);
      h.record(V(1)); // a pan begins
      h.wheel(V(2), 100); // 100 ms after the last step, but it is another pass
      expect(at(h.back(V(3)))).toBe(2);
      expect(at(h.back(V(2)))).toBe(1);
      expect(at(h.back(V(1)))).toBe(0);
    });

    it('a step back or forward ends the pass', () => {
      const h = new ViewHistory();
      h.record(V(0));
      h.wheel(V(1), 0);
      h.back(V(2));
      h.wheel(V(1), 50);
      expect(at(h.back(V(2)))).toBe(1);
    });

    it('a new pass empties what was to come like any new view', () => {
      const h = new ViewHistory();
      h.record(V(0));
      h.back(V(1));
      expect(h.canForward.value).toBe(true);
      h.wheel(V(0.5), 0);
      expect(h.canForward.value).toBe(false);
    });
  });
});

describe('view navigation', () => {
  function camera(): Camera {
    const c = new Camera();
    c.setSize(800, 600);
    c.setView({ x: 100, y: 200 }, 2);
    return c;
  }

  it('a navigation command keeps the view it left, and going back restores it exactly', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    nav.navigate(() => c.fit({ minX: 0, minY: 0, maxX: 1000, maxY: 500 }));
    expect(nav.history.canBack.value).toBe(true);
    expect(c.center).not.toEqual({ x: 100, y: 200 });
    expect(nav.back()).toBe(true);
    expect(c.center).toEqual({ x: 100, y: 200 });
    expect(c.scale).toBe(2);
    expect(nav.history.canForward.value).toBe(true);
    expect(nav.forward()).toBe(true);
    expect(c.center).toEqual({ x: 500, y: 250 });
  });

  it('a command that changes nothing keeps nothing and leaves Sonraki alone', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    nav.navigate(() => c.fit({ minX: 0, minY: 0, maxX: 1000, maxY: 500 }));
    nav.back();
    expect(nav.history.canForward.value).toBe(true);
    // Tümünü göster when the view is already there.
    nav.navigate(() => {});
    expect(nav.history.canForward.value).toBe(true);
    nav.forward();
    nav.navigate(() => c.fit({ minX: 0, minY: 0, maxX: 1000, maxY: 500 }));
    // The same fit again: no second, identical record.
    nav.back();
    expect(nav.back()).toBe(false);
  });

  it('zooming in and out are navigations of their own', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    nav.navigate(() => c.zoomAt(1.5, { x: 400, y: 300 }));
    nav.navigate(() => c.zoomAt(1 / 1.5, { x: 400, y: 300 }));
    expect(nav.back()).toBe(true);
    expect(c.scale).toBeCloseTo(3, 12);
    expect(nav.back()).toBe(true);
    expect(c.scale).toBe(2);
    expect(nav.back()).toBe(false);
  });

  it('remember keeps the view a pan is about to leave', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    nav.remember();
    c.panBy(50, 30);
    expect(c.center).not.toEqual({ x: 100, y: 200 });
    nav.back();
    expect(c.center).toEqual({ x: 100, y: 200 });
  });

  it('a wheel pass is one view however many steps it has', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    const step = (now: number) => {
      nav.wheel(now);
      c.zoomAt(1.1, { x: 400, y: 300 });
    };
    step(0);
    step(80);
    step(160);
    step(240);
    step(2000); // after a pause: the second pass
    step(2050);
    nav.back();
    expect(c.scale).toBeCloseTo(2 * 1.1 ** 4, 10);
    nav.back();
    expect(c.scale).toBe(2);
    expect(nav.back()).toBe(false);
  });

  it('the size of the window is not part of a view', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    nav.navigate(() => c.zoomAt(2, { x: 400, y: 300 }));
    c.setSize(1200, 700);
    c.setSize(500, 500);
    // Resizing kept nothing: one record is there, the zoom's.
    expect(nav.back()).toBe(true);
    expect(nav.back()).toBe(false);
    expect(c.width).toBe(500);
    expect(c.scale).toBe(2);
  });

  it('keeps the scale within the camera’s limits when it returns', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    nav.history.record({ center: { x: 0, y: 0 }, scale: 1e12 });
    nav.back();
    expect(c.scale).toBe(5e3);
  });

  it('goes nowhere when there is nowhere to go', () => {
    const c = camera();
    const nav = new ViewNavigation(c);
    expect(nav.back()).toBe(false);
    expect(nav.forward()).toBe(false);
    expect(c.center).toEqual({ x: 100, y: 200 });
  });
});
