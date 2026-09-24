export type Child = Node | string | number | null | undefined | false | Child[];

type Props = {
  class?: string | null;
  style?: string | Partial<CSSStyleDeclaration> | null;
  dataset?: Record<string, string>;
  [attr: string]: unknown;
};

/** Tiny hyperscript: h('button', { class: 'btn', onClick }, icon('line'), 'Çizgi'). */
export function h<K extends keyof HTMLElementTagNameMap>(tag: K, props?: Props | null, ...children: Child[]): HTMLElementTagNameMap[K] {
  const el = document.createElement(tag);
  if (props) {
    for (const [k, v] of Object.entries(props)) {
      if (v == null || v === false) continue;
      if (k === 'class') el.className = String(v);
      else if (k === 'style') typeof v === 'string' ? (el.style.cssText = v) : Object.assign(el.style, v);
      else if (k === 'dataset') Object.assign(el.dataset, v);
      else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2).toLowerCase(), v as EventListener);
      else if (v === true) el.setAttribute(k, '');
      else el.setAttribute(k, String(v));
    }
  }
  append(el, children);
  return el;
}

export function append(parent: Node, children: Child[]): void {
  for (const c of children) {
    if (c == null || c === false) continue;
    if (Array.isArray(c)) append(parent, c);
    else parent.appendChild(typeof c === 'object' ? c : document.createTextNode(String(c)));
  }
}

export function replaceChildren(el: Element, ...children: Child[]): void {
  el.textContent = '';
  append(el, children);
}

/** Root for popups, tooltips and dialogs so they escape panel overflow. */
export function overlayRoot(): HTMLElement {
  let root = document.getElementById('overlay-root');
  if (!root) {
    root = h('div', { id: 'overlay-root' });
    document.body.append(root);
  }
  return root;
}
