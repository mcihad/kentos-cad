// Human-readable summary of the web feature inventory (docs/inventory/web.md),
// generated together with web.json by inventory.mjs.

const SECTIONS = {
  commands: 'Komutlar',
  tools: 'Araçlar',
  processing: 'İşlem araçları',
  models: 'İşlem modelleri',
  workspaces: 'Çalışma modları',
  settings: 'Ayarlar',
  storage: 'Tarayıcı depoları',
  fileFields: '`.kcad` v1 alanları',
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
    'Menüde, şeritte ve araç kutusunda yoklar; kimlikleri `src/ui` altındaki hiçbir dosyada geçmiyor. Kısayolla, komut satırından ya da başka bir yoldan çalışıyor olabilirler. Her biri fareyle bulunabilirlik açısından gözden geçirilir.',
    '',
    s.commandsWithoutPlace.map((id) => `\`${id}\``).join(', ') || 'Yok.',
    '',
    '## Test başvurusu',
    '',
    `${s.commandsWithoutTests} / ${s.commands.total} komutun kimliği hiçbir test dosyasında ya da e2e betiğinde geçmiyor. Kimliğin bir testte geçmesi davranışın sınandığını göstermez; kabul kanıtı değildir.`,
    '',
  );
  return lines.join('\n');
}
