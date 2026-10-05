import type { Command } from '../core/commands';
import type { CadDocument } from '../model/document';
import type { LibraryCategory, LibraryItem } from '../model/style';
import { StyleLibrary } from '../style/library';
import { geometryClassOf } from '../style/geometry';
import { setProperties, uidsOf } from '../ui/properties/write';
import type { AppContext } from './context';
import { applyTemplate, drawWithTemplate, templateFromSelection } from './objectTemplates';
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

const isItem = (v: unknown): v is LibraryItem =>
  !!v && typeof v === 'object' && ['symbol', 'asset', 'template'].includes((v as LibraryItem).kind) && typeof (v as LibraryItem).id === 'string';

/** `system` is the built-in library (style/system), a chunk of its own loaded before the app starts. */
export function createStyles(doc: CadDocument, system: { items: readonly LibraryItem[]; categories?: readonly LibraryCategory[] }): StyleService {
  const library = new StyleLibrary(system);
  const stored = persistedSignals<UserStyles>('kentos.styles.v1', { items: [], categories: [] });
  library.load('user', (stored.items.value ?? []).filter(isItem), stored.categories.value ?? []);
  library.load('project', doc.styles.value.items, doc.styles.value.categories);
  let writing = false;
  library.events.on('changed', ({ source }) => {
    const d = library.dump(source);
    if (source === 'user') {
      stored.items.set(d.items);
      stored.categories.set(d.categories);
    } else {
      writing = true;
      try {
        doc.styles.set(d);
      } finally {
        writing = false;
      }
    }
  });
  // The drawing's own part follows the drawing: another one opened, a change taken in from the cloud (as the desktop's
  // `Styles::follow_project`); what the library itself wrote there is already in it.
  doc.styles.subscribe((s) => {
    if (!writing) library.load('project', s.items, s.categories);
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
      id: 'template.draw',
      title: 'Şablonla çiz',
      category: cat,
      icon: 'templateDraw',
      aliases: ['SABLONLACIZ', 'NESNESABLONU'],
      description: 'Bir nesne şablonuyla çizer: şablonun katmanı etkin olur, aracı başlar; nesneler şablonun sembolünü, özniteliklerini ve etiketini alır.',
      // With a template's id, it draws; without one, the Şablonlar panel shows the templates to choose from.
      run: (args) => {
        if (typeof args === 'string') return drawWithTemplate(ctx, args);
        ctx.commands.execute('template.panel');
      },
    },
    {
      id: 'template.apply',
      title: 'Şablonu uygula',
      category: cat,
      icon: 'templateApply',
      aliases: ['SABLONUYGULA'],
      description: 'Seçili nesnelere bir nesne şablonunun katmanını, rengini, kalınlığını, sembolünü, özniteliklerini ve etiketini tek adımda verir; şablonun türünde olmayanlar değişmez.',
      // With a template's id, it applies it; without one, the Şablonlar panel shows the templates to choose from.
      run: (args) => {
        if (typeof args === 'string') return applyTemplate(ctx, args);
        ctx.commands.execute('template.panel');
      },
    },
    {
      id: 'template.new',
      title: 'Yeni şablon…',
      category: cat,
      icon: 'templateNew',
      aliases: ['YENISABLON', 'SABLONEKLE'],
      description: 'Yeni bir nesne şablonu tanımlar: adı, aracı, katmanı, görünüşü, öznitelikleri ve etiketi.',
      run: () => void import('../ui/templates/TemplateEditor').then((m) => m.openTemplateEditor(ctx)),
    },
    {
      id: 'template.fromSelection',
      title: 'Seçili nesneden şablon…',
      short: 'Nesneden şablon',
      category: cat,
      icon: 'templateFromSelection',
      aliases: ['NESNEDENSABLON'],
      description: 'Seçili nesnenin katmanını, görünüşünü, özniteliklerini ve etiketini yeni bir şablona alır; düzenleyicide adlandırılıp kaydedilir.',
      isEnabled: () => ctx.selection.ids.value.size > 0,
      watch: [ctx.selection.ids],
      run: () => templateFromSelection(ctx),
    },
    {
      id: 'template.panel',
      title: 'Şablonlar',
      category: cat,
      icon: 'templates',
      aliases: ['SABLONLAR', 'NESNESABLONLARI'],
      description: 'Nesne şablonlarını kategorilerine göre gösteren paneli açar: tıklanan şablonla çizilir.',
      run: () => {
        ctx.ui.rightVisible.set(true);
        ctx.ui.dockTab.set('templates');
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

/**
 * Gives (or takes away) the selected objects' own symbol in one undo step,
 * through `cad.entities.set`; locked layers are skipped and counted.
 */
export function assignSymbol(ctx: AppContext, id: string | undefined): void {
  const { doc } = ctx;
  const name = id ? (ctx.styles.library.get(id)?.name ?? id) : '';
  let locked = 0;
  const ids: number[] = [];
  for (const eid of ctx.selection.ids.value) {
    const e = doc.get(eid);
    if (!e || e.symbol === id) continue;
    if (doc.layers.isLocked(e.layerId)) locked++;
    else ids.push(eid);
  }
  const write = () => setProperties(ctx, { uids: uidsOf(ctx, ids), symbol: id ?? null, operation: 'symbol' });
  // The step keeps the symbol's name, “Sembol: …”: the command knows no library and would say “Sembol ata”.
  const out = !ids.length ? null : id ? doc.transact(`Sembol: ${name}`, write) : write();
  const done = out?.changed.length ?? 0;
  const skipped = locked ? `; kilitli katmandaki ${locked} nesne atlandı` : '';
  if (id) ctx.log.info(`${done} nesneye “${name}” verildi${skipped}.`);
  else ctx.log.info(`${done} nesnenin sembolü kaldırıldı${skipped}.`);
}
