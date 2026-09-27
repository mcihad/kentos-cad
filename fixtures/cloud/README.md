# Bulut arayüzünün ortak durumları

Bulut projeleri penceresinin (katalog, [ADR 0028](../../docs/adr/0028-project-catalog.md)) iki platformda aynı sözleri ve kuralları kullanması için. Sunucunun işi (arama, sıralama, sayfalama, erişim denetimi) burada değildir; burada pencerenin ne yazdığı ve neyi sunduğu vardır.

- **Web**: `apps/web/src/app/cloud/catalogPlan.test.ts` (Vitest), `app/cloud/catalog.ts` ve `app/cloud/catalogPlan.ts`'e karşı. Pencere (`ui/cloud/CatalogDialog.ts`, `catalogRows.ts`, `catalogDetails.ts`, `ProjectActions.ts`) bu kararları yalnız çizer.
- **Masaüstü**: katalog penceresi (`apps/desktop/src/cloud/`) aynı dosyayı okur.

| Dosya | İçerik |
|---|---|
| `v1/catalog.json` | Katalogun sözleri ve kuralları |

## Biçim (`kentos.catalog`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.catalog"`, `1` |
| `timeZone` | Zamanlar cihazın yerel saatiyle yazılır; durumlar bu bölgeyle (`Europe/Istanbul`) denetlenir |
| `views` | Listeler, pencerenin sırasıyla: `id`, `label`, `sorts` (sunulan sıralamalar; ilki listenin kendi sırası), `empty` (arama yokken boş listenin sözü), `note` (listenin üstündeki not; varsa) |
| `sorts`, `states`, `types`, `typeHint` | Sıralama, durum ve proje türü adları; türün ne anlama gelmediğini söyleyen not |
| `emptySearch`, `noOrganization` | Arama ya da tür süzgeci varken boş liste; etkin kurum üyeliği yokken “Kurum projeleri” |
| `tips` | Yetkinin olmadığı işin nedeni (`denied`: `{name}`, `{what}`, `{permission}` yerlerine konur) ve arşivlenmiş projenin nedeni (`archived`) |
| `project` | Durumların temel projesi (`ProjectSummary`); her durum üstüne değişikliklerini yazar (`access` alan alan birleşir) |
| `details` | Seçili projenin bölmesi: `{ project, open }` → `favorite` (düğmenin adı; çöpte `null`), `chips`, `tabs` (çöpte `null`), `facts` (gösterilen satırların adları, sırasıyla), `actions` (sırasıyla `id`, `label`, `icon`, `why`: kapalıysa nedeni, açıksa `null`, `danger`: silen eylem) |
| `primary` | Pencerenin tek amber düğmesi: `{ view, project }` (`project` `null`: seçim yok) → `label`, `enabled`, `why` (düğmenin ipucu) |
| `rows` | Listenin satırı: `{ view, sort, open, place, project }` (`place`: projenin nerede olduğu, hesabın adlandırdığı gibi) → `marks` (adın yanındakiler: yıldızın ipucu “Favorilerinizde”, “Açık”, “Arşivde”), `sub` (altında: tür ve yeri ya da sahibi; çöpte taşıyan), `side` (sağda: paylaşılanlarda rol, çöpte silinme günü, listenin sıraladığı zaman) |
| `questions` | Çöpe taşıma, kalıcı silme ve arşivleme soruları: `args` → `title`, `message`, `details`, `action` |
| `lines` | Eylemlerin günlüğe ve listenin altına yazdığı satırlar: `line` (adı), `args`, `text` |

## Kurallar

- Her şey tam metinle karşılaştırılır.
- Web'in bugünkü sözleri ve kuralları yazılıdır; biri değişince bu dosya, iki çalıştırıcı ve bu belge birlikte değişir.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
