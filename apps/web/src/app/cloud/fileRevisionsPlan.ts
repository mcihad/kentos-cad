import { when } from './catalog';

/**
 * An open file project's revisions (docs/specs/file-revisions.md,
 * docs/adr/0038): what the drawing knows of the server's newest revision
 * and how it learns it (an event says someone committed a revision; the
 * server is then asked which, by whom and when), the save cell's state,
 * what Kaydet does first, what a resync asks when the missed events cannot
 * be replayed, the question a click brings and what each answer does to the
 * drawing and its unsaved work, and the words. A newer revision is said and
 * offered, never loaded by itself.
 *
 * Apart from the DOM and the network: fileProject.ts keeps its state through
 * `step`, ui/statusbar/cellsPlan.ts says the cell, ui/cloud/FileConflict.ts
 * asks. fixtures/cloud/v1/file-revisions.json holds all of it for the
 * desktop (format in fixtures/cloud/README.md).
 */

/** A newer revision than the drawing's: its number, who saved it and when, as far as the server said. */
export interface NewerRevision {
  revision: string;
  /** Who saved it; '' when not known (a refused Kaydet names only the number). */
  by: string;
  /** When it was saved (RFC 3339); null when not known. */
  at: string | null;
}

/** The revisions a refused Kaydet met: the drawing's base and the server's newest. */
export interface RevisionConflict {
  expected: string;
  actual: string;
}

/** Where a Kaydet is. */
export type SaveStage = 'idle' | 'encoding' | 'uploading' | 'verifying';

/** Why nothing more is saved to the project. */
export type Ended = 'deleted' | 'revoked' | 'archived';

/** The save cell's state (`cellState`). */
export type FileSaveState = 'saved' | 'pending' | 'encoding' | 'uploading' | 'verifying' | 'conflict' | 'outdated' | 'error' | 'readonly' | 'deleted' | 'revoked' | 'archived';

/** What the open file project knows: the facts its save cell and Kaydet read. */
export interface RevisionState {
  /** The revision the drawing on screen is based on ('0': none yet). */
  base: string;
  /** A newer revision than `base` on the server, as far as known. */
  newer: NewerRevision | null;
  /** Set when a Kaydet was refused (or known to be): until the user chooses, Kaydet asks again. */
  conflict: RevisionConflict | null;
  /** The drawing has changes no revision holds. */
  dirty: boolean;
  stage: SaveStage;
  /** The last Kaydet failed for another reason than a newer revision; forgotten when the next one begins. */
  failed: boolean;
  /** This account may write revisions (feature.write). */
  writable: boolean;
  ended: Ended | null;
}

/** The project opened at revision `base` ('0': none yet). */
export function opened(base: string, t: { dirty: boolean; writable: boolean }): RevisionState {
  return { base, newer: null, conflict: null, dirty: t.dirty, stage: 'idle', failed: false, writable: t.writable, ended: null };
}

/** What changes an open file project's state. */
export type RevisionInput =
  /** The drawing's unsaved changes came, or went (undone back to the saved state). */
  | { kind: 'dirty'; dirty: boolean }
  /** The server named its newest revision (asked after someone else's event, after a resync, or on a refused Kaydet); null: none yet. */
  | { kind: 'newest'; newest: NewerRevision | null }
  /** A Kaydet reached a stage; `encoding` begins one, and its last failure is forgotten. */
  | { kind: 'stage'; stage: Exclude<SaveStage, 'idle'> }
  /** The server committed this window's Kaydet as `revision`; `dirty`: edits made while it went stay unsaved. */
  | { kind: 'committed'; revision: string; dirty: boolean }
  /** Someone saved `actual` first: the server refused the Kaydet (`@file`), or Kaydet knew it would. */
  | { kind: 'refused'; actual: string }
  /** The Kaydet failed otherwise: no answer, or a refusal the drawing can do nothing about. */
  | { kind: 'failed' }
  /** A Kaydet found the drawing as its base: nothing to write, and the last failure is forgotten. */
  | { kind: 'unchanged' }
  /** What this account may do changed. */
  | { kind: 'access'; writable: boolean }
  /** The project was deleted, archived, or is out of reach: nothing more is saved there. */
  | { kind: 'ended'; why: Ended };

const num = (revision: string): number => Number(revision);

/**
 * One input: the state after it, and `say`: a newer revision became known
 * and is to be said (once; the log line). Once the project ended only the
 * drawing's own changes count.
 */
export function step(s: RevisionState, i: RevisionInput): { state: RevisionState; say: boolean } {
  const same = { state: s, say: false };
  if (s.ended && i.kind !== 'dirty') return same;
  const to = (state: RevisionState, say = false) => ({ state, say });
  switch (i.kind) {
    case 'dirty':
      return i.dirty === s.dirty ? same : to({ ...s, dirty: i.dirty });
    case 'newest': {
      const n = i.newest;
      if (!n || num(n.revision) <= num(s.base)) return same;
      const known = s.newer;
      if (known && num(n.revision) < num(known.revision)) return same;
      if (known && n.revision === known.revision) {
        // Said once; what was not known of it (a refused Kaydet names only the number) is filled in.
        const by = known.by || n.by;
        const at = known.at ?? n.at;
        return by === known.by && at === known.at ? same : to({ ...s, newer: { revision: known.revision, by, at } });
      }
      // A standing conflict is with the newest revision known.
      return to({ ...s, newer: n, conflict: s.conflict && { expected: s.conflict.expected, actual: n.revision } }, true);
    }
    case 'stage':
      return to({ ...s, stage: i.stage, failed: i.stage === 'encoding' ? false : s.failed });
    case 'committed': {
      const newer = s.newer && num(s.newer.revision) > num(i.revision) ? s.newer : null;
      return to({ ...s, base: i.revision, newer, dirty: i.dirty, stage: 'idle', failed: false });
    }
    case 'refused': {
      // What is known of the newer revision stays when it is the same one, or a later one.
      const newer = s.newer && num(s.newer.revision) >= num(i.actual) ? s.newer : { revision: i.actual, by: '', at: null };
      return to({ ...s, conflict: { expected: s.base, actual: newer.revision }, newer, stage: 'idle' });
    }
    case 'failed':
      return to({ ...s, stage: 'idle', failed: true });
    case 'unchanged':
      return s.failed ? to({ ...s, failed: false }) : same;
    case 'access':
      return i.writable === s.writable ? same : to({ ...s, writable: i.writable });
    case 'ended':
      return to({ ...s, ended: i.why, stage: 'idle' });
  }
}

/**
 * The save cell's state: why nothing is saved any more; a Kaydet's stage;
 * a conflict; a newer revision (also over unsaved work, and while read-only:
 * it can be opened); read-only; the last Kaydet's failure; unsaved changes;
 * saved.
 */
export function cellState(s: RevisionState): FileSaveState {
  if (s.ended) return s.ended;
  if (s.stage !== 'idle') return s.stage;
  if (s.conflict) return 'conflict';
  if (s.newer) return 'outdated';
  if (!s.writable) return 'readonly';
  if (s.failed) return 'error';
  return s.dirty ? 'pending' : 'saved';
}

/** What a Kaydet does first. */
export type SaveStep = 'ended' | 'readonly' | 'conflict' | 'unchanged' | 'behind' | 'save';

/**
 * What a Kaydet does first: nothing where nothing is saved any more or the
 * account may not write; the question again while a conflict stands;
 * nothing when the drawing is its base (a project without a revision
 * writes its first even so); the question when a newer revision is known
 * (`behind`: the server would refuse, so nothing is uploaded; the state
 * takes it as refused); else it writes the next revision on `base`.
 */
export function saveStep(s: RevisionState): SaveStep {
  if (s.ended) return 'ended';
  if (!s.writable) return 'readonly';
  if (s.conflict) return 'conflict';
  if (!s.dirty && s.base !== '0') return 'unchanged';
  if (s.newer) return 'behind';
  return 'save';
}

// ── Events ─────────────────────────────────────────────────────────────────

/** An event of the open project, as this plan reads it (`EventRecord`). */
export interface RevisionEvent {
  seq: string;
  kind: string;
  requestId?: string;
}

/** What a batch of the project's events asks for. */
export interface EventsRead {
  /** The last event's cursor; null for an empty batch. */
  cursor: string | null;
  /** The project was deleted or archived: nothing after it counts. `quiet`: this window archived it. */
  end: { why: 'deleted' | 'archived'; quiet: boolean } | null;
  /** A grant changed: what this account may do is asked again. */
  access: boolean;
  /** Someone else committed a revision: the server is asked which is its newest (once for the batch). */
  newest: boolean;
}

/**
 * Reads a batch of the project's events, in order: `project.deleted` and
 * `project.archived` end it; `project.access` asks the access again;
 * `project.file` from another request than this window's own commits asks
 * the newest revision. The event names no revision number (its
 * `dataRevision` is the project's, not the file's).
 */
export function readEvents(events: readonly RevisionEvent[], own: (requestId: string) => boolean): EventsRead {
  let cursor: string | null = null;
  let access = false;
  let newest = false;
  for (const e of events) {
    cursor = e.seq;
    const mine = !!e.requestId && own(e.requestId);
    if (e.kind === 'project.deleted') return { cursor, end: { why: 'deleted', quiet: false }, access: false, newest: false };
    if (e.kind === 'project.archived') return { cursor, end: { why: 'archived', quiet: mine }, access: false, newest: false };
    if (e.kind === 'project.access') access = true;
    if (e.kind === 'project.file' && !mine) newest = true;
  }
  return { cursor, end: null, access, newest };
}

/** The server's newest revision from its list (`GET …/files`): who saved it and when; null before the first. */
export function newestOf(r: { current?: string; revisions: readonly { revision: string; createdByName: string; createdAt: string }[] }): NewerRevision | null {
  if (!r.current) return null;
  const listed = r.revisions.find((x) => x.revision === r.current);
  return { revision: r.current, by: listed?.createdByName ?? '', at: listed?.createdAt ?? null };
}

// ── Resync ─────────────────────────────────────────────────────────────────

/** How long a resync that got no answer waits before the events are asked again. */
export const RESYNC_RETRY_MS = 30_000;

/** The server's answer about the project (`GET …/projects/{id}`) when the events it missed cannot be replayed. */
export type ProjectAnswer =
  | { kind: 'project'; state: 'active' | 'archived' | 'trashed'; eventCursor: string }
  /** 410: in the trash. */
  | { kind: 'deleted' }
  /** 404: gone for this account (removed, or its access taken). */
  | { kind: 'notFound' }
  /** 403: its organisation may not be used now; the server says why. */
  | { kind: 'forbidden'; message: string }
  /** No answer, after the passing failures were tried again. */
  | { kind: 'unreachable' }
  /** Any other refusal. */
  | { kind: 'failed'; message: string };

/** What a resync does next. */
export type ResyncStep =
  /** Nothing more is saved there (`reason`: the server's words, when it gave some). */
  | { kind: 'end'; why: Ended; reason: string }
  /** The same later (`RESYNC_RETRY_MS`): the drawing and what is known stay. */
  | { kind: 'retry' }
  /** Events are followed from `cursor`; the access is asked again and the newest revision asked, as their events would. */
  | { kind: 'follow'; cursor: string };

/**
 * A resync (the server keeps no events from the cursor any more, or the
 * cursor is beyond its newest): the project is asked what became of it;
 * the drawing is never replaced.
 */
export function resyncStep(a: ProjectAnswer): ResyncStep {
  switch (a.kind) {
    case 'project':
      if (a.state === 'trashed') return { kind: 'end', why: 'deleted', reason: '' };
      if (a.state === 'archived') return { kind: 'end', why: 'archived', reason: '' };
      return { kind: 'follow', cursor: a.eventCursor };
    case 'deleted':
      return { kind: 'end', why: 'deleted', reason: '' };
    case 'notFound':
      return { kind: 'end', why: 'revoked', reason: '' };
    case 'forbidden':
      return { kind: 'end', why: 'revoked', reason: a.message };
    default:
      return { kind: 'retry' };
  }
}

// ── Questions ──────────────────────────────────────────────────────────────

/** What an answer does. */
export type AnswerDoes =
  /** The upload window on file storage, named “<name> (kopya)”: the drawing becomes a new file project (its revision 1) and the open one; this project stays as it is. */
  | 'copy'
  /** Farklı kaydet: the drawing goes to a local .kcad and leaves the cloud project, which stays as it is. */
  | 'local'
  /** The server's newest revision is opened: it replaces the drawing and is its base. */
  | 'latest'
  /** Nothing: the drawing, its unsaved changes, a conflict and a newer revision stay. */
  | 'nothing';

/** What an answer does to the drawing's unsaved changes (their recovery copy on the web, a save kept on the device on the desktop). */
export type AnswerWork =
  /** Written by it (into the copy, or the local file): saved once that is written; the recovery copy goes then. */
  | 'saved'
  /** Dropped on purpose: their recovery copy (a kept save) goes too. */
  | 'dropped'
  /** Kept on screen, unsaved; their recovery copy too. */
  | 'kept'
  /** The drawing has none. */
  | 'none';

export interface RevisionAnswer {
  value: 'copy' | 'local' | 'latest' | 'stay' | 'discard' | 'open' | 'later';
  label: string;
  /** `primary`: the one amber button (Enter); `danger`: removes something. */
  kind?: 'primary' | 'danger';
  /** On the left of the bar, apart from the others. */
  aside?: boolean;
  does: AnswerDoes;
  work: AnswerWork;
}

/** A question about the open file project's revisions. */
export interface RevisionQuestion {
  id: 'conflict' | 'unsaved' | 'newest';
  title: string;
  message: string;
  /** What each choice does, point by point. */
  details: string[];
  /** In the bar's order (the aside ones go left). */
  answers: RevisionAnswer[];
  /** The answer of Esc, × and the backdrop: the one that changes nothing. */
  cancel: RevisionAnswer['value'];
}

/** What asking brings: a question, a line instead of one, or nothing. */
export type Offer = { kind: 'ask'; question: RevisionQuestion } | { kind: 'say'; tone: 'info' | 'warn'; line: string } | { kind: 'none' };

/** ` (who, when)` of a revision, as far as known; '' when neither is. */
export function revisionWho(n: NewerRevision): string {
  const parts = [n.by, n.at ? when(n.at) : ''].filter(Boolean);
  return parts.length ? ` (${parts.join(', ')})` : '';
}

/** “Açık çizim revizyon 4”, or that it has none yet; `lower` inside a sentence. */
function openAt(base: string, lower = false): string {
  const a = lower ? 'a' : 'A';
  return base === '0' ? `${a}çık çizimin henüz revizyonu yok` : `${a}çık çizim revizyon ${base}`;
}

/**
 * The question after a refused Kaydet, or before the newest revision over
 * unsaved work: the drawing needs a place first. Null without a conflict or
 * a newer revision.
 */
export function conflictQuestion(name: string, s: RevisionState): RevisionQuestion | null {
  const c = s.conflict ?? (s.newer ? { expected: s.base, actual: s.newer.revision } : null);
  if (!c) return null;
  // A standing conflict is always with the newer revision known (`step`): who saved it and when are its.
  const who = s.newer ? revisionWho(s.newer) : '';
  const on = c.expected === '0' ? 'çiziminiz henüz kaydedilmiş bir revizyona dayanmıyor' : `çiziminizin dayandığı revizyon ${c.expected}`;
  const w = (had: AnswerWork): AnswerWork => (s.dirty ? had : 'none');
  return {
    id: 'conflict',
    title: 'Dosya başka biri tarafından kaydedildi',
    message: `“${name}” siz çalışırken başka biri tarafından kaydedildi: sunucuda revizyon ${c.actual}${who} var; ${on}. Hiçbir şey yazılmadı; iki dosya birleştirilmez.`,
    details: [
      `Ayrı kopya olarak kaydet: çiziminiz yeni bir bulut dosya projesi olur ve açık proje o olur; “${name}” olduğu gibi kalır.`,
      'Yerel dosyaya kaydet: çiziminiz bu bilgisayara .kcad olarak kaydedilir ve çizim buluttaki projeden ayrılır.',
      s.dirty ? `Son revizyonu aç: revizyon ${c.actual} açılır; bu çizimdeki kaydedilmemiş değişiklikler atılır.` : `Son revizyonu aç: revizyon ${c.actual} açılır.`,
    ],
    answers: [
      { value: 'latest', label: 'Son revizyonu aç', kind: 'danger', aside: true, does: 'latest', work: w('dropped') },
      { value: 'stay', label: 'Vazgeç', does: 'nothing', work: w('kept') },
      { value: 'local', label: 'Yerel dosyaya kaydet', does: 'local', work: w('saved') },
      { value: 'copy', label: 'Ayrı kopya olarak kaydet', kind: 'primary', does: 'copy', work: w('saved') },
    ],
    cancel: 'stay',
  };
}

/** Unsaved work and no newer revision known: the newest may be the drawing's own base, and opening it drops the work. */
export function unsavedQuestion(name: string, base: string): RevisionQuestion {
  return {
    id: 'unsaved',
    title: 'Kaydedilmemiş değişiklikler',
    message: `“${name}” içinde kaydedilmemiş değişiklikler var. Sunucudaki en yeni revizyon açılırsa bu değişiklikler atılır (${openAt(base, true)}). Saklamak için önce Kaydet ile kaydedin.`,
    details: [],
    answers: [
      { value: 'discard', label: 'Kaydetmeden aç', aside: true, does: 'latest', work: 'dropped' },
      { value: 'stay', label: 'Vazgeç', does: 'nothing', work: 'kept' },
    ],
    cancel: 'stay',
  };
}

/** A clean drawing: a plain question. */
export function newestQuestion(name: string, base: string, newer: NewerRevision | null): RevisionQuestion {
  return {
    id: 'newest',
    title: 'Son revizyonu aç',
    message: newer
      ? `“${name}” başka bir yerde kaydedildi: revizyon ${newer.revision}${revisionWho(newer)}. ${openAt(base)}; kaydedilmemiş değişikliği yok.`
      : `“${name}” projesinin sunucudaki en yeni revizyonu açılsın mı? ${openAt(base)}; kaydedilmemiş değişikliği yok.`,
    details: [],
    answers: [
      { value: 'later', label: 'Sonra', does: 'nothing', work: 'none' },
      { value: 'open', label: 'Son revizyonu aç', kind: 'primary', does: 'latest', work: 'none' },
    ],
    cancel: 'later',
  };
}

/**
 * What asking brings. `via: 'newest'`: the save cell's click on a newer
 * revision, or Son revizyonu aç…: a line while a Kaydet is on its way or
 * where the project cannot be read any more; the conflict's question over
 * unsaved work (or a standing conflict); the unsaved question when no newer
 * revision is known; else the plain question. `via: 'conflict'`: a refused
 * Kaydet, or Kayıt çakışmalarını çöz…: the conflict's question, or nothing.
 */
export function offer(t: { name: string; s: RevisionState; busy: boolean; via: 'newest' | 'conflict' }): Offer {
  const { s, name } = t;
  const ask = (question: RevisionQuestion | null): Offer => (question ? { kind: 'ask', question } : { kind: 'none' });
  if (t.via === 'conflict') return ask(conflictQuestion(name, s));
  if (t.busy) return { kind: 'say', tone: 'info', line: REVISION_TEXTS.busy(name) };
  if (s.ended === 'deleted' || s.ended === 'revoked') return { kind: 'say', tone: 'warn', line: REVISION_TEXTS.unreadable[s.ended](name) };
  if (s.conflict || (s.dirty && s.newer)) return ask(conflictQuestion(name, s));
  if (s.dirty) return ask(unsavedQuestion(name, s.base));
  return ask(newestQuestion(name, s.base, s.newer));
}

// ── Words ──────────────────────────────────────────────────────────────────

export const REVISION_TEXTS = {
  /** Son revizyonu aç while a Kaydet is on its way. */
  busy: (name: string) => `“${name}” kaydediliyor; son revizyonu açmak için kaydın bitmesini bekleyin.`,
  /** Son revizyonu aç where the project cannot be read any more. */
  unreadable: {
    deleted: (name: string) => `“${name}” bulut projesi silindi; son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin.`,
    revoked: (name: string) => `“${name}” projesine erişiminiz kaldırıldı; son revizyonu açılamaz. Çizimi saklamak için yerel bir dosyaya kaydedin.`,
  },
  /** A resync got no answer (said once until one comes). */
  resyncFailed: (name: string) =>
    `“${name}” projesinin kaçırılan olayları alınamadı ve sunucu yanıt vermedi; başkasının kaydettiği bir revizyon şimdilik bilinmiyor. Yarım dakika sonra yeniden sorulur; çizim olduğu gibi duruyor.`,
  /** A local file written from the conflict's question. */
  detached: (name: string) => `Çizim yerel dosyaya kaydedildi ve “${name}” bulut projesinden ayrıldı; proje olduğu gibi duruyor.`,
  /** The newest revision could not be opened. */
  openFailed: (name: string, why: string) => `“${name}” son revizyonu açılamadı: ${why}`,
} as const;

/**
 * The log line when a newer revision becomes known (warning): who saved
 * it and when, the drawing's base, and what a click on the save cell
 * offers; over unsaved work, that Kaydet cannot write over it.
 */
export function newerLine(name: string, newer: NewerRevision, base: string, dirty: boolean): string {
  const on = base === '0' ? 'Açık çizimin henüz revizyonu yok' : `Açık çizimin dayandığı revizyon: ${base}`;
  const then = dirty
    ? 'kaydedilmemiş değişiklikleriniz Kaydet ile bu revizyonun üzerine yazılamaz. Ayrı kopya, yerel dosya ya da son revizyon için durum çubuğundaki kayıt durumuna tıklayın.'
    : 'yeni revizyonu açmak için durum çubuğundaki kayıt durumuna tıklayın.';
  return `“${name}” başka bir yerde kaydedildi: revizyon ${newer.revision}${revisionWho(newer)}. ${on}; ${then} Kendiliğinden yeniden yüklenmez.`;
}

/** The tip's line about a newer revision (cellsPlan.ts `fileTip`). */
export function newerTip(newer: NewerRevision, dirty: boolean): string {
  const head = `Sunucuda daha yeni revizyon var: ${newer.revision}${revisionWho(newer)}`;
  return dirty ? `${head}; kaydedilmemiş değişiklikleriniz Kaydet ile onun üzerine yazılamaz. Seçenekler için tıklayın.` : `${head}; açmak için tıklayın.`;
}

export const HISTORY_MARKS = { newest: 'En yeni', base: 'Açık çizim' } as const;

/** The marks of a revision's row in the history: the newest; the one the open drawing is based on (when the project is open here). */
export function revisionMarks(revision: string, current: string | undefined, openBase: string | null): string[] {
  return [...(revision === current ? [HISTORY_MARKS.newest] : []), ...(revision === openBase ? [HISTORY_MARKS.base] : [])];
}
