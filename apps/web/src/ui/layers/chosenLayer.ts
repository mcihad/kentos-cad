import { Signal } from '../../core/signal';

/**
 * The layer chosen in the layer tree (a click or the keys; the desktop's `selected_layer`): with nothing selected on
 * the drawing, Öznitelikler shows a map service layer's or a data layer's source for it (docs/adr/0208 §14), else for
 * the active layer. Not the active layer: choosing a row does not change where new objects go.
 */
export const chosenLayer = new Signal<string | null>(null);
