import type { CommandRegistry } from '../core/commands';
import type { Keymap } from '../core/keymap';
import type { CadDocument } from '../model/document';
import type { Selection } from '../model/selection';
import type { ToolManager } from '../tools/ToolManager';
import type { ViewportController } from '../viewport/ViewportController';
import type { Clipboard } from './clipboard';
import type { CloudSession } from './cloud/session';
import type { DocumentFiles } from './fileIO';
import type { ServerStatus } from './server';
import type { Formatter } from './format';
import type { ProcessingService } from './processing';
import type { DraftingSettings, MessageLog, Preferences, UiState } from './state';
import type { SettingsStore } from './settings/store';
import type { StyleService } from './styles';

/**
 * The single dependency every feature module receives. Modules talk to each
 * other only through these services — never by importing each other's UI.
 */
export interface AppContext {
  readonly commands: CommandRegistry;
  readonly keymap: Keymap;
  readonly doc: CadDocument;
  readonly selection: Selection;
  readonly settings: DraftingSettings;
  readonly log: MessageLog;
  readonly ui: UiState;
  /** Persisted user preferences (Ayarlar): the values in use, one signal each. */
  readonly prefs: Preferences;
  /**
   * The typed settings behind `prefs` (docs/adr/0023): requested and effective
   * values with the reason, the device's constraints, export and import.
   */
  readonly settingsStore: SettingsStore;
  /** Units/precision-aware number formatting. */
  readonly format: Formatter;
  readonly tools: ToolManager;
  readonly view: ViewportController;
  /** Copied entities (session only). */
  readonly clipboard: Clipboard;
  /** İşlem araçları: registry, runner, last values (see docs/PROCESSING.md). */
  readonly processing: ProcessingService;
  /** Style library: system, user and project symbols (see docs/STYLE.md). */
  readonly styles: StyleService;
  /** Local drawing files (.kcad): save, save as, open (app/fileIO.ts). */
  readonly files: DocumentFiles;
  /** Whether the KentOS API answers (`/v1/health`); the drawing works without it (app/server.ts). */
  readonly server: ServerStatus;
  /** Signing in, the open cloud project, its autosave and live events (app/cloud/session.ts). */
  readonly cloud: CloudSession;
}
