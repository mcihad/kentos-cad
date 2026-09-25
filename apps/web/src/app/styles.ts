import type { Command } from '../core/commands';
import type { CadDocument } from '../model/document';
import type { LibraryCategory, LibraryItem } from '../model/style';
import { StyleLibrary } from '../style/library';
import { geometryClassOf } from '../style/geometry';
import type { AppContext } from './context';
import { persistedSignals } from './state';

/**
 * The style library in the app (docs/STYLE.md §5): system symbols from
 * the code, the user's own kept in this browser (`kentos.styles.v1`,
 * later the cloud), and the project's kept in the document.
 */
export interface StyleService {
  readonly library: StyleLibrary;
}

interface UserStyles {
  items: LibraryItem[];
  categories: LibraryCategory[];
}

const isItem = (v: unknown): v is LibraryItem => !!v && typeof v === 'object' && ((v as LibraryItem).kind === 'symbol' || (v as LibraryItem).kind === 'asset') && typeof (v as LibraryItem).id === 'string';

/** `system` is the built-in library (style/system), a chunk of its own loaded before the app starts. */
export function createStyles(doc: CadDocument, system: { items: readonly LibraryItem[]; categories?: readonly LibraryCategory[] }): StyleService {
  const library = new StyleLibrary(system);
  const stored = persistedSignals<UserStyles>('kentos.styles.v1', { items: [], categories: [] });
  library.load('user', (stored.items.value ?? []).filter(isItem), stored.categories.value ?? []);
  library.load('project', doc.styles.value.items, doc.styles.value.categories);
  library.events.on('changed', ({ source }) => {
    const d = library.dump(source);
    if (source === 'user') {
      stored.items.set(d.items);
      stored.categories.set(d.categories);
    } else doc.styles.set(d);
  });
  return { library };
}

/**
 * Style commands. The windows are loaded when first opened (most sessions
 * never style anything), so the app's first load stays small.
 */
export function registerStyleCommands(ctx: AppContext): void {
  const cat = 'Stil';
  const manager = () => import('../ui/style/StyleManager');
  const activeLayer = () => {
    const n = ctx.doc.layers.get(ctx.doc.layers.active.value);
    return n?.type === 'layer' ? n : null;
  };
  const withSymbol = () => [...ctx.selection.ids.value].map((id) => ctx.doc.get(id)).filter((e) => !!e?.symbol);
  const list: Command[] = [
    {
      id: 'style.manager',
      title: 'Stil yöneticisi…',
      category: cat,
      icon: 'styles',
      aliases: ['STIL', 'STILLER', 'STYLE', 'SEMBOLLER'],
      description: 'Sembol kitaplığı: sistem, kullanıcı ve proje sembolleri; kopyala, düzenle, içe ve dışa aktar.',
      run: () => void manager().then((m) => m.openStyleManager(ctx)),
    },
    {
      id: 'style.svgEditor',
      title: 'SVG çizim düzenleyicisi…',
      short: 'SVG düzenleyici',
      category: cat,
      icon: 'penTool',
      aliases: ['SVG', 'CIZIMDUZENLE', 'PIKTOGRAM'],
      description: 'İşaret ve desen çizimleri (piktogram) çizer; kaydedince Kitaplığım\'a eklenir.',
      run: () => void import('../ui/svgedit/SvgEditor').then((m) => m.openSvgEditor(ctx)),
    },
    {
      id: 'style.layerStyle',
      title: 'Katman stili…',
      category: cat,
      icon: 'layerStyle',
      aliases: ['KATMANSTILI', 'LAYERSTYLE'],
      description: 'Etkin katmanın nesnelerinin nasıl çizileceği: tek sembol, kategorili, aralıklı ya da kurallarla.',
      isEnabled: () => !!activeLayer(),
      watch: [ctx.doc.layers.active],
      run: () => {
        const n = activeLayer();
        if (n) void import('../ui/style/LayerStyleDialog').then((m) => m.openLayerStyle(ctx, n.id));
      },
    },
    {
      id: 'style.legend',
      title: 'Lejant…',
      category: cat,
      icon: 'legend',
      aliases: ['LEJANT', 'LEGEND', 'ACIKLAMA'],
      description: 'Çizimdeki sembollerin anlamı, katman katman; PNG olarak kaydedilir.',
      run: () => void import('../ui/style/LegendDialog').then((m) => m.openLegend(ctx)),
    },
    {
      id: 'style.assign',
      title: 'Seçili nesnelere sembol ver…',
      short: 'Sembol ver',
      category: cat,
      icon: 'symbolAssign',
      aliases: ['SEMBOLVER', 'SEMBOL'],
      description: 'Kitaplıktan bir sembol seçer ve seçili nesnelere verir; nesnenin sembolü katman stilinin önüne geçer.',
      isEnabled: () => ctx.selection.ids.value.size > 0,
      watch: [ctx.selection.ids],
      run: () => {
        const first = ctx.doc.get([...ctx.selection.ids.value][0]);
        const cls = first ? geometryClassOf(first) : null;
        void manager().then((m) =>
          m.openStyleManager(ctx, {
            pick: {
              kind: cls ?? undefined,
              title: 'Seçili nesnelere sembol verin',
              current: first?.symbol,
              onPick: (id) => assignSymbol(ctx, id),
            },
          }),
        );
      },
    },
    {
      id: 'style.clearSymbol',
      title: 'Nesne sembolünü kaldır',
      short: 'Sembolü kaldır',
      category: cat,
      icon: 'symbolClear',
      aliases: ['SEMBOLKALDIR'],
      description: 'Seçili nesneler yeniden katmanlarının stiliyle çizilir.',
      isEnabled: () => withSymbol().length > 0,
      watch: [ctx.selection.ids],
      run: () => assignSymbol(ctx, undefined),
    },
  ];
  for (const c of list) ctx.commands.register(c);
}

/** Gives (or takes away) the selected objects' own symbol in one undo step; locked layers are skipped. */
export function assignSymbol(ctx: AppContext, id: string | undefined): void {
  const { doc } = ctx;
  const name = id ? (ctx.styles.library.get(id)?.name ?? id) : '';
  let locked = 0;
  const patches: { id: number; symbol: string | undefined }[] = [];
  for (const eid of ctx.selection.ids.value) {
    const e = doc.get(eid);
    if (!e || e.symbol === id) continue;
    if (doc.layers.isLocked(e.layerId)) locked++;
    else patches.push({ id: eid, symbol: id });
  }
  const done = doc.updateMany(patches, id ? `Sembol: ${name}` : 'Sembolü kaldır');
  const skipped = locked ? `; kilitli katmandaki ${locked} nesne atlandı` : '';
  if (id) ctx.log.info(`${done} nesneye “${name}” verildi${skipped}.`);
  else ctx.log.info(`${done} nesnenin sembolü kaldırıldı${skipped}.`);
}
