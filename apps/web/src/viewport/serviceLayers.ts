import type { ConnectionSecret } from '../contracts/generated/ConnectionSecret';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import type { CadDocument } from '../model/document';
import type { LayerNode } from '../model/layers';
import { datumChoices, ownSystem } from '../model/projectCrs';
import type { CanvasPalette } from '../render/color';
import { serviceKey, type Credit, type PlacedLabel, type ServiceProject, type ServiceShown } from '../render/serviceHub';
import type { RGBA, SceneLayer } from '../render/types';

/**
 * The map services in the drawing area (docs/adr/0208 §3, §9; the desktop's `services/overlay.rs` and `app.rs`): a
 * service layer's scene layer (one batch the drawing passes draw from the services' tiles), the services the scene
 * shows and the project they are drawn in, their labels over the drawing (under its own text) and the credits' strip
 * at the area's bottom right with the card it opens.
 */

/** A service layer's scene: one batch, drawn by the passes from its service's tiles. */
export function serviceSceneLayer(id: string, key: string, opacity: number, origin: { x: number; y: number }): SceneLayer {
  const huge = 1e15;
  return {
    id,
    lines: [],
    fills: [],
    points: [],
    styled: [
      {
        kind: 'fill',
        positions: new Float32Array(0),
        paint: { kind: 'service', service: key, opacity, anchor: [origin.x, origin.y] },
        bounds: [-huge, -huge, huge, huge],
        reach: 0,
        reachUnit: 'world',
      },
    ],
  };
}

/** A service layer's connection in the project. */
export function connectionOf(doc: CadDocument, node: LayerNode): ServiceConnection | null {
  const id = node.service?.connection;
  return id ? (doc.settings.connections.value.find((c) => c.id === id) ?? null) : null;
}

/** The services the drawing shows, bottom first (a hidden layer or group's are not). */
export function shownServices(doc: CadDocument): ServiceShown[] {
  return doc.layers
    .leaves()
    .filter((l) => l.service && doc.layers.isVisible(l.id))
    .reverse()
    .map((l) => {
      const connection = connectionOf(doc, l);
      return { key: serviceKey(l.service!, connection), service: l.service!, connection, name: l.name };
    });
}

/** The project the services are drawn in: its system and datum choices in the core's JSON, its anchor. */
export function serviceProject(doc: CadDocument): ServiceProject {
  const s = doc.settings.toJSON();
  const own = ownSystem(s);
  return {
    ours: own?.system ? JSON.stringify(own.system) : null,
    choices: JSON.stringify(datumChoices(s)),
    srid: s.srid,
    custom: s.srid === 0 && !!s.customCrs,
    origin: [doc.origin.x, doc.origin.y],
  };
}

/** A secret's look-up for the hub. */
export type SecretOf = (c: ServiceConnection) => ConnectionSecret | null;

const css = (c: readonly number[]) => `rgba(${Math.round(c[0] * 255)}, ${Math.round(c[1] * 255)}, ${Math.round(c[2] * 255)}, ${c[3]})`;

/** The labels the services' vector tiles show, as the worker placed them (CSS pixels of the area), in Görünüm kipleri's colours. */
export function drawServiceLabels(g: CanvasRenderingContext2D, labels: readonly PlacedLabel[], view: (c: RGBA) => RGBA): void {
  const mode = (c: readonly number[]) => css(view([c[0], c[1], c[2], c[3]]));
  if (!labels.length) return;
  g.save();
  g.lineJoin = 'round';
  g.textAlign = 'left';
  g.textBaseline = 'alphabetic';
  for (const l of labels) {
    g.font = `${l.italic ? 'italic ' : ''}${l.bold ? 700 : 400} ${l.size}px Arimo, Arial, "Liberation Sans", sans-serif`;
    const fill = mode(l.color);
    const halo = l.halo ? mode(l.halo) : null;
    const width = l.halo ? Math.min(l.halo[4], 3) : 0;
    for (const line of l.lines) {
      if (halo) {
        g.strokeStyle = halo;
        g.lineWidth = width * 2;
        g.strokeText(line.text, line.x, line.y);
      }
      g.fillStyle = fill;
      g.fillText(line.text, line.x, line.y);
    }
    if (l.glyphs.length) {
      g.textAlign = 'center';
      for (const ch of l.glyphs) {
        if (!ch.ch.trim()) continue;
        g.save();
        g.translate(ch.x, ch.y);
        g.rotate(ch.angle);
        if (halo) {
          g.strokeStyle = halo;
          g.lineWidth = width * 2;
          g.strokeText(ch.ch, 0, l.size * 0.35);
        }
        g.fillStyle = fill;
        g.fillText(ch.ch, 0, l.size * 0.35);
        g.restore();
      }
      g.textAlign = 'left';
    }
  }
  g.restore();
}

/** The strip's longest text, characters: the rest is in the card. */
const STRIP_MOST = 96;

/**
 * The credits' strip at the drawing area's bottom right, under the scale bar; a click opens the card that lists each
 * credit with its links (opened in a new tab; only `http` and `https` addresses).
 */
export class CreditsStrip {
  readonly el: HTMLButtonElement;
  private readonly card: HTMLDivElement;
  private credits: Credit[] = [];
  private shownText = '';

  constructor(host: HTMLElement) {
    this.el = document.createElement('button');
    this.el.type = 'button';
    this.el.className = 'service-credits';
    this.el.hidden = true;
    this.el.title = 'Görünen servislerin atıfları';
    this.card = document.createElement('div');
    this.card.className = 'service-credits-card';
    this.card.hidden = true;
    this.el.addEventListener('click', (e) => {
      e.stopPropagation();
      this.card.hidden = !this.card.hidden;
      if (!this.card.hidden) this.fillCard();
    });
    this.el.addEventListener('pointerdown', (e) => e.stopPropagation());
    this.card.addEventListener('pointerdown', (e) => e.stopPropagation());
    host.append(this.el, this.card);
  }

  /** Its height and gap: how far the scale bar is lifted over it. */
  lift(): number {
    return this.el.hidden ? 0 : Math.max(0, this.el.offsetHeight + 2 + 3 - 18);
  }

  update(credits: readonly Credit[], joined: (texts: string[]) => string[]): void {
    this.credits = [...credits];
    const text = joined(credits.map((c) => c.text)).join(' | ');
    if (text === this.shownText) return;
    this.shownText = text;
    this.el.hidden = !text;
    this.el.textContent = text.length > STRIP_MOST ? `${[...text].slice(0, STRIP_MOST - 1).join('')}…` : text;
    if (!text) this.card.hidden = true;
    else if (!this.card.hidden) this.fillCard();
  }

  private fillCard(): void {
    this.card.replaceChildren();
    const head = document.createElement('div');
    head.className = 'service-credits-head';
    const title = document.createElement('strong');
    title.textContent = 'Atıflar';
    const close = document.createElement('button');
    close.type = 'button';
    close.className = 'service-credits-close';
    close.textContent = '×';
    close.title = 'Kapat';
    close.addEventListener('click', () => (this.card.hidden = true));
    head.append(title, close);
    const note = document.createElement('p');
    note.className = 'service-credits-note';
    note.textContent = 'Görünen servislerin atıfları; bağlantılar yeni sekmede açılır.';
    this.card.append(head, note);
    for (const c of this.credits) {
      const item = document.createElement('div');
      item.className = 'service-credits-item';
      const text = document.createElement('div');
      text.textContent = c.text;
      item.append(text);
      for (const [words, url] of c.links) {
        if (!/^https?:\/\//i.test(url)) continue;
        const a = document.createElement('a');
        a.href = url;
        a.target = '_blank';
        a.rel = 'noopener noreferrer';
        a.textContent = words;
        item.append(a);
      }
      this.card.append(item);
    }
  }

  /** The palette's look (the strip's ground follows the drawing area's). */
  theme(pal: CanvasPalette): void {
    const [r, g, b] = pal.background;
    this.el.style.setProperty('--credits-ground', `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`);
  }

  dispose(): void {
    this.el.remove();
    this.card.remove();
  }
}
