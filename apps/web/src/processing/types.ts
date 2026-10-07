import type { AngleUnit, DrawingFont } from '../model/projectSettings';
import type { Entity, EntityKind, NewEntity } from '../model/entities';
import type { CompiledExpression } from '../model/expression/expression';
import type { Vec2 } from '../model/geometry';
import type { LayerField } from '../model/layerFields';
import type { LayerStyle } from '../model/layers';
import type { RunGeometry } from './geometry';

/**
 * İşlem araçları (QGIS Processing gibi): batch operations over many
 * objects, described declaratively so that the dialog, the command line,
 * history, models (flow diagrams) and — later — a worker or a server can
 * all run the same tool. See docs/PROCESSING.md.
 *
 * A ProcessingTool never touches the document. It receives resolved,
 * read-only inputs and returns a ChangeSet; the runner applies that as one
 * undo step. That keeps tools pure, testable, chainable and portable to
 * other execution targets.
 */

// ── Parameters ─────────────────────────────────────────────────────────

/** Which objects a features parameter reads. */
export type FeatureScope = 'selection' | 'visible' | 'all' | 'layer' | 'ids';

/**
 * Value of a features parameter. "ids" is what models pass between steps
 * (the objects a previous step created). `kinds` narrows the tool's kinds
 * for this run (only polygons, say); absent means every kind the tool takes.
 */
export type FeaturesValue = ({ scope: 'selection' | 'visible' | 'all' } | { scope: 'layer'; layerId: string } | { scope: 'ids'; ids: number[] }) & { kinds?: EntityKind[] };

/** Value of a layer parameter: an existing layer, or a new one made when the tool writes to it. */
export type LayerValue = { layerId: string } | { newName: string };

/** Units shown next to number fields (and used to read typed values). */
export type ParamUnit = 'm' | 'm²' | 'mm' | '°' | 'adet' | '';

/** Values the defaults may depend on (project settings at the time the dialog opens). */
export interface DefaultsContext {
  lengthDecimals: number;
  areaDecimals: number;
  angleUnit: AngleUnit;
  plotScale: number;
  activeLayer: string;
  /** The drawing's typeface (`ProjectSettings.drawingFont`): texts a tool places are measured in it. */
  drawingFont: DrawingFont;
}

/**
 * A fixed default, or one read from the project when the dialog opens.
 * Type the argument (`(c: DefaultsContext) => …`): an untyped arrow inside
 * a definition stops TypeScript from inferring the parameter types.
 */
type Default<T> = T | ((ctx: DefaultsContext) => T);

/** Values seen by `visibleWhen`. */
export type Shown = Readonly<Record<string, unknown>>;

interface ParamBase<N extends string> {
  /** Key in the values object (English identifier). */
  readonly name: N;
  /** Turkish label in the dialog. */
  readonly label: string;
  /** One or two sentences under the label: what it does, what it expects. */
  readonly description?: string;
  /** May be left empty (value null). Parameters are required by default. */
  readonly optional?: boolean;
  /** Folded under "Gelişmiş" in the dialog. */
  readonly advanced?: boolean;
  /**
   * Shown only when this returns true (e.g. a point for "nearest to a
   * point"). Type the argument as `Shown` in definitions: an untyped arrow
   * here would stop TypeScript from inferring the parameter types.
   */
  readonly visibleWhen?: (values: Shown) => boolean;
}

export interface FeaturesParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'features';
  /** Entity kinds the tool works on; others in scope are ignored. */
  readonly kinds?: readonly EntityKind[];
  /** Scopes offered in the dialog (default: all but "ids"). */
  readonly scopes?: readonly Exclude<FeatureScope, 'ids'>[];
  readonly default?: Default<FeaturesValue>;
  /**
   * The tool changes these objects (Öznitelik hesapla): objects on locked
   * layers are left out when the input is resolved, and said, so the
   * summary, the outputs and a model's next step count only what is written.
   */
  readonly writes?: boolean;
}

export interface NumberParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'number';
  readonly default?: Default<number>;
  readonly min?: number;
  readonly max?: number;
  readonly integer?: boolean;
  readonly unit?: ParamUnit;
}

export interface StringParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'string';
  readonly default?: Default<string>;
  readonly placeholder?: string;
  readonly maxLength?: number;
  /** Required strings may still be empty when this is true (e.g. an empty prefix). */
  readonly allowEmpty?: boolean;
}

export interface BooleanParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'boolean';
  readonly default?: Default<boolean>;
}

export interface EnumOption<V extends string = string> {
  readonly value: V;
  readonly label: string;
  readonly hint?: string;
}

export interface EnumParam<N extends string = string, V extends string = string> extends ParamBase<N> {
  readonly type: 'enum';
  readonly options: readonly EnumOption<V>[];
  readonly default: Default<V>;
  /**
   * One of the options is a point picked on the drawing (the numbering's
   * start vertex, docs/adr/0088): that option, and the point parameter the
   * pick fills. The dialog puts “Sahneden seç” beside the choice; a picked
   * point sets both.
   */
  readonly picks?: { readonly option: V; readonly point: string };
}

export interface LayerParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'layer';
  readonly default?: Default<LayerValue>;
  /** Style of a layer created for the output. */
  readonly newLayerStyle?: Partial<LayerStyle>;
}

export interface PointParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'point';
  readonly default?: Default<Vec2 | null>;
}

/**
 * An expression over each object's attributes and geometry (expression.ts):
 * a condition (select, filter) or a value (field calculator). Written as
 * text; the tool receives it compiled.
 */
export interface ExpressionParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'expression';
  readonly default?: Default<string>;
  /** "condition": true or false per object; "value": something to write. */
  readonly returns: 'condition' | 'value';
  /** Name of the features parameter whose objects it reads (fields offered, live preview). */
  readonly of?: string;
  readonly placeholder?: string;
}

/**
 * An attribute name of the objects of a features parameter, or a column of a file parameter's table; `allowNew`
 * lets the user type a new one. `of` names the parameter the names come from; a list names several, of which the
 * first shown is read (a source that is a layer or a file, docs/adr/0200 §6). `multiple`: several names, written
 * with commas between them ("Malik, Hisse").
 */
export interface FieldParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'field';
  readonly default?: Default<string>;
  readonly of: string | readonly string[];
  readonly allowNew?: boolean;
  readonly multiple?: boolean;
}

/**
 * A file the user chooses (docs/adr/0200 §7): a table read when it is chosen (CSV, TXT, XLSX: Tablo ekle's
 * reader, the first sheet), its first row the column names. `accept`: the extensions offered (".csv").
 */
export interface FileParam<N extends string = string> extends ParamBase<N> {
  readonly type: 'file';
  readonly accept: readonly string[];
  readonly default?: null;
}

/**
 * A file parameter's value: the file's name and its rows. The last values keep only the name (the desktop also its
 * path), so a file is chosen again when the dialog opens again; a value without rows asks for that.
 */
export interface FileValue {
  readonly name: string;
  readonly rows?: readonly (readonly string[])[];
  /** Where the desktop read it, to read it again. */
  readonly path?: string;
}

export type ParamDef = FeaturesParam | NumberParam | StringParam | BooleanParam | EnumParam | LayerParam | PointParam | ExpressionParam | FieldParam | FileParam;
export type ParamType = ParamDef['type'];

/** A parameter's value as the dialog and history hold it. */
export type ValueOf<D> = D extends { type: 'features' }
  ? FeaturesValue
  : D extends { type: 'number' }
    ? number
    : D extends { type: 'string' }
      ? string
      : D extends { type: 'boolean' }
        ? boolean
        : D extends { type: 'enum'; options: readonly { value: infer V }[] }
          ? V
          : D extends { type: 'layer' }
            ? LayerValue
            : D extends { type: 'point' }
              ? Vec2 | null
              : D extends { type: 'expression' | 'field' }
                ? string
                : D extends { type: 'file' }
                  ? FileValue | null
                  : never;

type Maybe<D, T> = D extends { optional: true } ? T | null : T;

/** Values keyed by parameter name, typed from the definitions. */
export type ParamValues<Ds extends readonly ParamDef[]> = { [D in Ds[number] as D['name']]: Maybe<D, ValueOf<D>> };

/** Objects a features parameter resolved to, with a short description ("12 seçili kapalı alan"). */
export interface FeatureSet {
  readonly entities: readonly Entity[];
  readonly description: string;
}

/** Where a layer parameter writes: an existing layer, or one the runner creates on apply. */
export interface TargetLayer {
  readonly id: string;
  readonly name: string;
  readonly isNew: boolean;
}

type ResolvedOf<D> = D extends { type: 'features' } ? FeatureSet : D extends { type: 'layer' } ? TargetLayer : D extends { type: 'expression' } ? CompiledExpression : ValueOf<D>;

/** What `run` receives: features and layers resolved, the rest as entered. */
export type ResolvedValues<Ds extends readonly ParamDef[]> = { [D in Ds[number] as D['name']]: Maybe<D, ResolvedOf<D>> };

// ── Outputs ────────────────────────────────────────────────────────────

export interface OutputDef {
  readonly name: string;
  readonly label: string;
  /**
   * features: ids of objects the tool created (a model feeds them to the
   * next step); number/string: a value in RunResult.outputs; table: a
   * `TableOutput` the dialog shows after the run (docs/adr/0200 §7), not
   * passed to a model's next step.
   */
  readonly type: 'features' | 'number' | 'string' | 'table';
}

/** A table a run gives (Özet istatistik): its columns and rows, texts. */
export interface TableOutput {
  readonly columns: readonly string[];
  readonly rows: readonly (readonly string[])[];
}

// ── Running ────────────────────────────────────────────────────────────

/**
 * Where a tool can run, in order of preference. Only "client" (in the
 * page) exists today; "worker" (a Web Worker with a document snapshot),
 * "server" (the KentOS service) and "postgis" (SQL on the database) are
 * reserved so tools can declare them now. See docs/PROCESSING.md.
 */
export type ExecutionTarget = 'client' | 'worker' | 'server' | 'postgis';

/** Read-only view of the document a tool may consult beyond its inputs. */
export interface DocumentSnapshot {
  get(id: number): Entity | undefined;
  all(): Iterable<Entity>;
  byLayer(layerId: string): readonly Entity[];
}

export interface RunContext {
  readonly doc: DocumentSnapshot;
  readonly units: DefaultsContext;
  /** Layer name for an id (expressions, summaries). */
  layerName(id: string): string;
  /** Ids selected when the run started (selection tools combine with it). */
  readonly selection: readonly number[];
  /**
   * Geometry of the run's features inputs, by id, from the Rust geometry
   * store (docs/adr/0008, S4): the same code wherever the run is.
   */
  readonly geometry: RunGeometry;
  /** A layer's field of that name (docs/adr/0199 §1): what a value written to the attribute becomes. */
  field(layerId: string, name: string): LayerField | undefined;
}

/** Progress, messages and cancellation, shared with the dialog. */
export interface Feedback {
  /** Share done in 0..1, with an optional step label. */
  progress(fraction: number, label?: string): void;
  info(message: string): void;
  warn(message: string): void;
  readonly canceled: boolean;
  /** Lets the page breathe during long loops; resolves at once when there is nothing to do. */
  yield(): Promise<void>;
}

/** Document edits a run asks for, applied by the runner in one transaction. */
export interface ChangeSet {
  add?: NewEntity[];
  update?: { id: number; patch: Partial<Entity> }[];
  remove?: number[];
}

export interface RunResult {
  changes?: ChangeSet;
  /** The selection after the run, for tools that select rather than edit. */
  select?: readonly number[];
  /** Values for number/string outputs. */
  outputs?: Record<string, unknown>;
  /** One line for the log and history ("24 köşe numaralandı: P00001 – P00024"). */
  summary?: string;
  /**
   * The run refuses, and says why (a file without the key column): nothing
   * changes and the run ends as an error with this message as it is.
   */
  refused?: string;
}

// ── The tool ───────────────────────────────────────────────────────────

export interface ProcessingTool<Ds extends readonly ParamDef[] = readonly ParamDef[]> {
  /** Stable identifier "alan.eylem" (English), e.g. "numbering.vertices". */
  readonly id: string;
  /** Turkish name shown in the toolbox, menus and dialog title. */
  readonly label: string;
  /** Category id (see categories.ts); decides where the tool sits in the tree. */
  readonly category: string;
  /** One sentence: what it does. */
  readonly description: string;
  /** Longer help for the dialog's side panel (paragraphs separated by blank lines). */
  readonly help?: string;
  /** Extra search words (Turkish and English). */
  readonly keywords?: readonly string[];
  /** Short names for the command line ("KOSENUMARA"); Turkish letters are folded. */
  readonly aliases?: readonly string[];
  readonly icon?: string;
  readonly parameters: Ds;
  readonly outputs?: readonly OutputDef[];
  /** Supported execution targets, preferred first. */
  readonly targets: readonly ExecutionTarget[];
  /** Cross-parameter checks; returns a message for the user, or null. */
  validate?(values: ParamValues<Ds>): string | null;
  /** Live one-line preview in the dialog (e.g. "P00001, P00002, …"). */
  preview?(values: ParamValues<Ds>): string | null;
  run(values: ResolvedValues<Ds>, ctx: RunContext, feedback: Feedback): RunResult | Promise<RunResult>;
}

/** Declares a tool with its parameter values typed from the definitions. */
export function defineTool<const Ds extends readonly ParamDef[]>(tool: ProcessingTool<Ds>): ProcessingTool<Ds> {
  return tool;
}
