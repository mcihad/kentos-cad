/**
 * What comes with the engine (docs/sheet/integration.md §7, Web, C bölümü):
 * the painter's sources and caches (the map frames drawn through the app's
 * own pipeline, the engine's inputs), the template actions, the pictures,
 * and the cloud's template library. The service imports this module
 * together with the engine's package, the first time a sheet comes forward
 * or the gallery opens, so none of it is in the app's first download.
 */

export { SheetPaint } from './paint';
export { newSheet, saveAsTemplate, sheetFromTemplate, templateAction, templateNeeds, templatePreview } from './templateActions';
export { addPicture } from './pictures';
export { coordinateInputOf, coordinateObjects } from './inputs';
export { CloudLibrary } from './cloudLibrary';
