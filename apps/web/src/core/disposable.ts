/** A function that releases a resource (listener, subscription, GPU buffer…). */
export type Disposable = () => void;

/** Collects disposables and releases them in reverse order. */
export class DisposableStore {
  private items: Disposable[] = [];

  add(d: Disposable): Disposable {
    this.items.push(d);
    return d;
  }

  dispose(): void {
    const items = this.items;
    this.items = [];
    for (let i = items.length - 1; i >= 0; i--) items[i]();
  }
}

export function listen<T extends Event = Event>(
  target: EventTarget,
  type: string,
  handler: (e: T) => void,
  options?: AddEventListenerOptions | boolean,
): Disposable {
  const fn = handler as EventListener;
  target.addEventListener(type, fn, options);
  return () => target.removeEventListener(type, fn, options);
}
