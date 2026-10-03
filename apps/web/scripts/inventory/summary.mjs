// Human-readable summary of the web feature inventory (docs/inventory/web.md),
// generated together with web.json by inventory.mjs.

const SECTIONS = {
  commands: 'Komutlar',
  tools: 'Araçlar',
  processing: 'İşlem araçları',
  models: 'İşlem modelleri',
  workspaces: 'Proje türleri',
  settings: 'Ayarlar',
  storage: 'Tarayıcı depoları',
  fileFields: '`.kcad` alanları (v1 okunur, v2 yazılır)',
  screens: 'Pencereler ve paneller',
};

export function summaryMarkdown(inv) {
  const status = inv.vocabulary.status;
  const s = inv.summary;
  const name = (i) => i.title ?? i.label ?? i.name ?? '';
  const why = (i) => (i.note ? ` — ${i.note}` : i.pendingNote ? ` — ${i.pendingNote}` : '');
  const listed = (st) => Object.keys(SECTIONS).flatMap((k) => inv[k].filter((i) => i.status === st).map((i) => `- ${SECTIONS[k]}: \`${i.id}\` ${name(i)}${why(i)}`.trimEnd()));
  const lines = [
    '# Web özellik envanteri: özet',
    '',
    'Üretilmiş dosyadır, elle düzenlenmez. Yöntem ve alanlar: [README.md](README.md). Tam veri: [web.json](web.json).',
    '',
    `| Bölüm | Toplam | ${status.join(' | ')} |`,
    `|---|---|${status.map(() => '---').join('|')}|`,
    ...Object.entries(SECTIONS).map(([k, label]) => `| ${label} | ${s[k].total} | ${status.map((st) => s[k][st]).join(' | ')} |`),
    '',
  ];
  for (const [st, title] of [
    ['partial', 'Kısmi'],
    ['pending', 'Bekleyen'],
  ]) {
    const items = listed(st);
    lines.push(`## ${title} (${items.length})`, '', ...(items.length ? items : ['Yok.']), '');
  }
  lines.push(
    `## Arayüzde yeri görünmeyen komutlar (${s.commandsWithoutPlace.length})`,
    '',
    'Menüde ve şeritte yoklar; kimlikleri `src/ui` altındaki hiçbir dosyada geçmiyor. Kısayolla, komut satırından ya da başka bir yoldan çalışıyor olabilirler. Her biri fareyle bulunabilirlik açısından gözden geçirilir.',
    '',
    s.commandsWithoutPlace.map((id) => `\`${id}\``).join(', ') || 'Yok.',
    '',
    ...desktopMarkdown(inv, name),
    '## Test başvurusu',
    '',
    `${s.commandsWithoutTests} / ${s.commands.total} komutun kimliği hiçbir test dosyasında ya da e2e betiğinde geçmiyor. Kimliğin bir testte geçmesi davranışın sınandığını göstermez; kabul kanıtı değildir.`,
    '',
  );
  return lines.join('\n');
}

/** The desktop column: per section what the desktop has, then, by section, what the web has and it does not. */
function desktopMarkdown(inv, name) {
  const d = inv.summary.desktop;
  const where = (i) => (i.desktopWhere ? ` (masaüstünde: ${i.desktopWhere})` : '');
  const note = (i) => (i.desktopNote ? ` — ${i.desktopNote}` : i.note ? ` — ${i.note}` : '');
  const out = [
    '## Masaüstü',
    '',
    'Masaüstü sütunu şuralardan gelir, her biri öncekinin üstüne: `apps/desktop/equivalents.json`\'ın bütün bir bölüm için dediği; masaüstü kabuğunun çalıştırdığı komutlar (`apps/desktop/ported.json`) ve onlarla araçları (`tool.<kimlik>`), işlem araçları ve modelleri (`processing.run.…`, `processing.model.…`), proje türleri (`workspace.<kimlik>`); şeması masaüstünü de barındıran tipli ayarlar; tablonun öğe öğe dediği (masaüstündeki yeri ya da orada neden anlamsız olduğu); en son `annotations.json`. Bilinmeyen `none`dır. Masaüstünde komutu olmayanlar şeritte soluk durur ve “masaüstüne henüz taşınmadı” der (docs/adr/0017).',
    '',
    '| Bölüm | Masaüstünde | Kısmi | Yok | Bekliyor | Anlamsız | Toplam |',
    '|---|---|---|---|---|---|---|',
    ...Object.entries(SECTIONS).map(([k, label]) => `| ${label} | ${d[k].implemented} | ${d[k].partial} | ${d[k].none} | ${d[k].pending} | ${d[k]['n/a']} | ${d[k].total} |`),
    '',
    'Bekliyor: web\'de de yapılmamış (`pending`); masaüstü onları web\'in notuyla soluk gösterir, web gibi.',
    '',
  ];
  const whole = Object.entries(inv.summary.desktopSections ?? {});
  if (whole.length) out.push(...whole.map(([k, e]) => `- ${SECTIONS[k] ?? k}, bütünüyle: ${e.desktop}${e.where ? `, ${e.where}` : ''}${e.reason ? ` — ${e.reason}` : ''}`), '');
  const na = Object.keys(SECTIONS).flatMap((k) => inv[k].filter((i) => i.platforms.desktop === 'n/a').map((i) => `- ${SECTIONS[k]}: \`${i.id}\` ${name(i)}${note(i)}`.trimEnd()));
  out.push(`### Masaüstünde anlamsız (${na.length})`, '', ...(na.length ? na : ['Yok.']), '');
  out.push('### Web\'de olup masaüstünde olmayanlar', '', 'Kısmi olanlar notlarıyla; bölüm bölüm.', '');
  for (const [k, label] of Object.entries(SECTIONS)) {
    const left = inv[k].filter((i) => ['none', 'partial'].includes(i.platforms.desktop));
    // Not done on the web either: listed after the others, marked, and not counted as missing on the desktop.
    const waiting = inv[k].filter((i) => i.platforms.desktop === 'pending');
    out.push(`#### ${label} (${left.length} / ${inv[k].length}${waiting.length ? `; ayrıca ${waiting.length} iki platformda da bekliyor` : ''})`, '');
    const lines = [
      ...left.map((i) => `- \`${i.id}\` ${name(i)}${i.platforms.desktop === 'partial' ? ' (kısmi)' : ''}${where(i)}${note(i)}`.trimEnd()),
      ...waiting.map((i) => `- \`${i.id}\` ${name(i)} (iki platformda da bekliyor)${where(i)}${note(i)}`.trimEnd()),
    ];
    out.push(...(lines.length ? lines : ['Yok.']), '');
  }
  return out;
}
