import { DisposableStore } from '../core/disposable';

/**
 * Base for UI parts. A component owns one root element and every
 * subscription it makes; dispose() detaches both.
 */
export abstract class Component<E extends HTMLElement = HTMLElement> {
  abstract readonly el: E;
  protected readonly d = new DisposableStore();

  dispose(): void {
    this.d.dispose();
    this.el.remove();
  }
}
