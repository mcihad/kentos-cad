import { Signal } from '../core/signal';
import type { Bounds, Vec2 } from '../model/geometry';

/** Converts between absolute world metres and CSS pixels of the viewport. */
export interface ViewTransform {
  readonly scale: number;
  readonly width: number;
  readonly height: number;
  worldToScreen(p: Vec2): Vec2;
  screenToWorld(p: Vec2): Vec2;
}

const MIN_SCALE = 1e-4; // px per metre (≈ whole country)
const MAX_SCALE = 5e3; // ≈ 0.2 mm per pixel

/** 2D orthographic camera. Centre is absolute (float64), y axis points north. */
export class Camera implements ViewTransform {
  center: Vec2 = { x: 0, y: 0 };
  scale = 1;
  width = 1;
  height = 1;
  /** Bumped whenever the view changes. */
  readonly changed = new Signal(0);

  worldToScreen(p: Vec2): Vec2 {
    return { x: (p.x - this.center.x) * this.scale + this.width / 2, y: this.height / 2 - (p.y - this.center.y) * this.scale };
  }

  screenToWorld(p: Vec2): Vec2 {
    return { x: this.center.x + (p.x - this.width / 2) / this.scale, y: this.center.y - (p.y - this.height / 2) / this.scale };
  }

  setSize(w: number, h: number): void {
    this.width = Math.max(1, w);
    this.height = Math.max(1, h);
    this.bump();
  }

  zoomAt(factor: number, screen: Vec2): void {
    const before = this.screenToWorld(screen);
    this.scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, this.scale * factor));
    const after = this.screenToWorld(screen);
    this.center = { x: this.center.x + before.x - after.x, y: this.center.y + before.y - after.y };
    this.bump();
  }

  panBy(dxPx: number, dyPx: number): void {
    this.center = { x: this.center.x - dxPx / this.scale, y: this.center.y + dyPx / this.scale };
    this.bump();
  }

  fit(b: Bounds, paddingPx = 48): void {
    const w = Math.max(b.maxX - b.minX, 1e-6);
    const h = Math.max(b.maxY - b.minY, 1e-6);
    const sx = (this.width - paddingPx * 2) / w;
    const sy = (this.height - paddingPx * 2) / h;
    this.scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, Math.min(sx, sy)));
    this.center = { x: (b.minX + b.maxX) / 2, y: (b.minY + b.maxY) / 2 };
    this.bump();
  }

  visibleBounds(): Bounds {
    const a = this.screenToWorld({ x: 0, y: this.height });
    const b = this.screenToWorld({ x: this.width, y: 0 });
    return { minX: a.x, minY: a.y, maxX: b.x, maxY: b.y };
  }

  private bump(): void {
    this.changed.update((v) => v + 1);
  }
}
