// The portal's icons (DESIGN.md §6): drawn by hand on a 20×20 grid, stroke = currentColor, 1.4 px, round caps and
// joins, no fills. The web app's set is the drawing tools'; these are the portal's own few, in the same hand.

const PATHS = {
  home: '<path d="M3.5 9 10 3.5 16.5 9"/><path d="M5 8v8.5h10V8"/><path d="M8.5 16.5v-4.5h3v4.5"/>',
  inbox: '<path d="M3 11.5 5 4.5h10l2 7"/><path d="M3 11.5v4h14v-4h-4l-1 2H8l-1-2z"/>',
  send: '<path d="M17 3 8.5 11.5"/><path d="M17 3 12 17l-3.5-5.5L3 8z"/>',
  building: '<path d="M4 17V4.5h8V17"/><path d="M12 8.5h4V17"/><path d="M2.5 17h15"/><path d="M6.5 7.5h3M6.5 10.5h3M6.5 13.5h3"/>',
  users: '<circle cx="7.5" cy="7" r="2.8"/><path d="M2.5 16.5c.4-3 2.4-4.6 5-4.6s4.6 1.6 5 4.6"/><path d="M12.5 4.5a2.6 2.6 0 0 1 0 5"/><path d="M14.5 12.2c1.7.5 2.8 1.9 3 4.3"/>',
  user: '<circle cx="10" cy="6.8" r="3.2"/><path d="M4 17c.5-3.6 2.8-5.4 6-5.4s5.5 1.8 6 5.4"/>',
  badge: '<rect x="3" y="4" width="14" height="12" rx="1.6"/><circle cx="7.5" cy="9" r="1.8"/><path d="M5 13.3c.4-1.4 1.3-2.1 2.5-2.1s2.1.7 2.5 2.1"/><path d="M11.5 8h3.5M11.5 11h3.5"/>',
  shield: '<path d="M10 2.8 16 5v4.6c0 3.6-2.5 6.3-6 7.6-3.5-1.3-6-4-6-7.6V5z"/><path d="m7.3 10 1.9 1.9 3.6-3.8"/>',
  file: '<path d="M5 2.5h6.5L15 6v11.5H5z"/><path d="M11.5 2.5V6H15"/>',
  pdf: '<path d="M5 2.5h6.5L15 6v11.5H5z"/><path d="M11.5 2.5V6H15"/><path d="M7.5 14.5v-4h1.4a1.2 1.2 0 0 1 0 2.4H7.5"/>',
  project: '<path d="M10 2.5 16.5 6v8L10 17.5 3.5 14V6z"/><path d="M3.5 6 10 9.5 16.5 6M10 9.5v8"/>',
  folder: '<path d="M2.5 5.5v10h15v-8H9.5L8 5.5z"/>',
  message: '<path d="M3 4.5h14v9H9l-4 3v-3H3z"/>',
  bell: '<path d="M5 13.5V9a5 5 0 0 1 10 0v4.5l1.5 1.5h-13z"/><path d="M8.3 17a1.8 1.8 0 0 0 3.4 0"/>',
  settings: '<circle cx="10" cy="10" r="2.6"/><path d="M10 2.5v2M10 15.5v2M2.5 10h2M15.5 10h2M4.7 4.7l1.4 1.4M13.9 13.9l1.4 1.4M4.7 15.3l1.4-1.4M13.9 6.1l1.4-1.4"/>',
  logout: '<path d="M8 3.5H4v13h4"/><path d="M12.5 6.5 16 10l-3.5 3.5M16 10H7.5"/>',
  plus: '<path d="M10 4v12M4 10h12"/>',
  search: '<circle cx="8.5" cy="8.5" r="5"/><path d="m12.3 12.3 4.2 4.2"/>',
  check: '<path d="m4 10.5 4 4 8-9"/>',
  close: '<path d="m5 5 10 10M15 5 5 15"/>',
  clock: '<circle cx="10" cy="10" r="7"/><path d="M10 6v4.3l2.8 1.7"/>',
  alert: '<path d="M10 3 17.5 16.5h-15z"/><path d="M10 8v4"/><path d="M10 14.3v.2"/>',
  download: '<path d="M10 3v10M6 9l4 4 4-4"/><path d="M3.5 14.5v2h13v-2"/>',
  upload: '<path d="M10 13V3M6 7l4-4 4 4"/><path d="M3.5 14.5v2h13v-2"/>',
  pin: '<path d="M10 17.5s5.5-5 5.5-9a5.5 5.5 0 0 0-11 0c0 4 5.5 9 5.5 9z"/><circle cx="10" cy="8.5" r="2"/>',
  list: '<path d="M7.5 5.5h9M7.5 10h9M7.5 14.5h9"/><path d="M3.5 5.5h.5M3.5 10h.5M3.5 14.5h.5"/>',
  chevronDown: '<path d="m5.5 8 4.5 4.5L14.5 8"/>',
  chevronRight: '<path d="m8 5.5 4.5 4.5L8 14.5"/>',
  arrowLeft: '<path d="M16 10H4.5M9 5.5 4.5 10 9 14.5"/>',
  open: '<path d="M11 3.5h5.5V9"/><path d="M16.5 3.5 9.5 10.5"/><path d="M14 12v4.5H3.5V6H8"/>',
  trash: '<path d="M3.5 5.5h13"/><path d="M8 5.5V3.5h4v2"/><path d="M5 5.5l.8 11h8.4l.8-11"/>',
  paperclip: '<path d="m15.5 9.5-5.8 5.8a3.5 3.5 0 0 1-5-5l6.3-6.3a2.3 2.3 0 0 1 3.3 3.3L8 13.6a1.2 1.2 0 0 1-1.7-1.7L12 6.2"/>',
  sun: '<circle cx="10" cy="10" r="3.2"/><path d="M10 2.5v1.8M10 15.7v1.8M2.5 10h1.8M15.7 10h1.8M4.7 4.7 6 6M14 14l1.3 1.3M4.7 15.3 6 14M14 6l1.3-1.3"/>',
  moon: '<path d="M15.8 12.6A6.5 6.5 0 0 1 7.4 4.2a6.5 6.5 0 1 0 8.4 8.4z"/>',
  eye: '<path d="M2.5 10c1.8-3.4 4.3-5 7.5-5s5.7 1.6 7.5 5c-1.8 3.4-4.3 5-7.5 5s-5.7-1.6-7.5-5z"/><circle cx="10" cy="10" r="2.3"/>',
  phone: '<rect x="6" y="2.5" width="8" height="15" rx="1.6"/><path d="M9 15h2"/>',
  monitor: '<rect x="2.5" y="3.5" width="15" height="10" rx="1.2"/><path d="M7 17h6M10 13.5V17"/>',
  globe: '<circle cx="10" cy="10" r="7"/><path d="M3 10h14"/><path d="M10 3c2 2.1 3 4.4 3 7s-1 4.9-3 7c-2-2.1-3-4.4-3-7s1-4.9 3-7z"/>',
  copy: '<rect x="6.5" y="6.5" width="10" height="10" rx="1.4"/><path d="M13.5 6.5V4.8a1.3 1.3 0 0 0-1.3-1.3H4.8a1.3 1.3 0 0 0-1.3 1.3v7.4a1.3 1.3 0 0 0 1.3 1.3h1.7"/>',
  image: '<rect x="2.5" y="3.5" width="15" height="13" rx="1.4"/><circle cx="7" cy="8" r="1.5"/><path d="m3 15 4.5-4.5 3 3 2-2 4.5 4.5"/>',
  robot: '<rect x="4" y="6.5" width="12" height="9" rx="2"/><path d="M10 6.5V3.8M7.5 10.5h.01M12.5 10.5h.01M8 13h4"/><path d="M2.5 10v2.5M17.5 10v2.5"/>',
  key: '<circle cx="7" cy="12.5" r="3.5"/><path d="m9.5 10 7-7M14 5.5l2 2M12 7.5l1.5 1.5"/>',
  lock: '<rect x="4.5" y="9" width="11" height="8" rx="1.4"/><path d="M7 9V6.5a3 3 0 0 1 6 0V9"/>',
  stamp: '<path d="M8 3.5h4l-.7 6h-2.6z"/><path d="M4 12.5h12v2.5H4z"/><path d="M4.5 17.5h11"/>',
  swap: '<path d="M4 7h11.5M12.5 4l3 3-3 3"/><path d="M16 13H4.5M7.5 10l-3 3 3 3"/>',
  hourglass: '<path d="M5.5 3h9M5.5 17h9"/><path d="M6.5 3c0 4 7 4 7 7s-7 3-7 7M13.5 3c0 4-7 4-7 7s7 3 7 7"/>',
  more: '<circle cx="5" cy="10" r=".9"/><circle cx="10" cy="10" r=".9"/><circle cx="15" cy="10" r=".9"/>',
  refresh: '<path d="M16 10a6 6 0 1 1-1.8-4.3"/><path d="M16 3.5v3.3h-3.3"/>',
  receipt: '<path d="M5 2.5h10v15l-1.7-1.2-1.6 1.2-1.7-1.2-1.7 1.2-1.6-1.2L5 17.5z"/><path d="M7.5 6.5h5M7.5 9.5h5M7.5 12.5h3"/>',
  signature: '<path d="M2.5 16.5h7"/><path d="m12 3.5 4.5 4.5L9.5 15H5v-4.5z"/><path d="m10.3 5.2 4.5 4.5"/>',
  grid: '<rect x="3" y="3" width="5.5" height="5.5" rx="1"/><rect x="11.5" y="3" width="5.5" height="5.5" rx="1"/><rect x="3" y="11.5" width="5.5" height="5.5" rx="1"/><rect x="11.5" y="11.5" width="5.5" height="5.5" rx="1"/>',
} as const;

export type IconName = keyof typeof PATHS;

export function icon(name: IconName, size = 18): SVGSVGElement {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 20 20');
  svg.setAttribute('width', String(size));
  svg.setAttribute('height', String(size));
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', '1.4');
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.setAttribute('aria-hidden', 'true');
  svg.classList.add('icon');
  svg.innerHTML = PATHS[name];
  return svg;
}

/** The KentOS mark (DESIGN.md §2): a navy tile, the K with its survey point. */
export function brandMark(size = 24): SVGSVGElement {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 22 22');
  svg.setAttribute('width', String(size));
  svg.setAttribute('height', String(size));
  svg.setAttribute('aria-hidden', 'true');
  svg.classList.add('brand-mark');
  svg.innerHTML =
    '<defs><linearGradient id="kentos-tile" x1="0" y1="0" x2="1" y2="1">' +
    '<stop offset="0" stop-color="var(--c-brand-hi)"/><stop offset="1" stop-color="var(--c-brand)"/></linearGradient></defs>' +
    '<rect x="0.5" y="0.5" width="21" height="21" rx="6" fill="url(#kentos-tile)"/>' +
    '<rect x="1.3" y="1.3" width="19.4" height="19.4" rx="5.2" fill="none" stroke="rgba(255,255,255,.22)" stroke-width=".8"/>' +
    '<g fill="none" stroke="var(--c-brand-ink)" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">' +
    '<path d="M7.6 5.6v10.8M7.6 11.2l6.4-5.6M10.2 9.3l4.2 7.1"/></g>' +
    '<circle cx="10.2" cy="9.3" r="1.6" fill="var(--c-brand-ink)"/>';
  return svg;
}
