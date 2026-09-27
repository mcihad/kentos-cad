# Klasik arayüzün ortak kuralları

Menü çubuğu, araç çubuğu ve kayan araç kutusunun ([DESIGN.md](../../DESIGN.md) §7.1–§7.4) iki platformda aynı kuralları ve sözleri kullanması için. Bir adımın sığıp sığmadığı her platformda kendi ölçüsüyle bulunur; burada her adımın ne yaptığı, araç kutusunun yeri, sütunları ve görünmesi, bir komutun menüdeki satırı vardır.

- **Web**: `apps/web/src/ui/shell/shellPlan.test.ts` (Vitest), `ui/shell/shellPlan.ts`, `app/menus.ts` (`menuRowLook`) ve `ui/toolbar/fields.ts`'e karşı. `MenuBar.ts`, `Toolbar.ts` ve `Toolbox.ts` bu kuralları uygular.
- **Masaüstü**: klasik arayüz aynı dosyayı okur.
- **Kaydedici**: `python3 scripts/fixtures/shell_cases.py` (`--check` hiçbir şey yazmadan karşılaştırır). Tablolar ve sözler DESIGN.md'den ve web'in çubuklarından elle yazılmıştır; cevaplar koddan ayrı, betikte bulunur.

| Dosya | İçerik |
|---|---|
| `v1/shell.json` | Klasik arayüzün kuralları ve sözleri |

## Biçim (`kentos.shell`, sürüm 1)

| Alan | Anlamı |
|---|---|
| `format`, `version` | `"kentos.shell"`, `1` |
| `menubar.folds` | Menüler sığmazken sırayla denenen adımlar: 0 bütün çubuk; 1 “KentOS” yazısı gizlenir (`brandWord`); 2 koordinat sisteminin adı da gizlenir (`crsName`; düğmesi ve ipucu kalır); 3 menülerin yan boşluğu 9'dan 6 px'e iner (`menuPadding`). Proje adı hepsinden önce kendiliğinden kısalır. İlk sığan adım kalır |
| `menubar.texts` | Çubuğun erişilebilir adı, kaydedilmemiş noktasının ipucu, koordinat sistemi düğmesinin ipucu (`crsTip`: SRID'den) |
| `toolbar.groups` | Araç çubuğunun grupları sırasıyla: komutlar (`commands`), geçerli özellik listeleri (`fields`: katman, renk, tip, kalınlık) ya da ölçek (`scale`; önünde esnek boşluk). Görünüm grubunun sonunda ⋯ düğmesi durur (katlanınca görünür) |
| `toolbar.viewMore`, `toolbar.widths` | ⋯'e giren komutlar; listelerin genişliği (standart yazıda CSS px, yazı ölçeğiyle çarpılır): tam, dar. Ölçek listesi daralmaz |
| `toolbar.folds` | Araç çubuğunun adımları (0–4): katman listesi dar mı (`layerNarrow`), renk/tip/kalınlık dar mı (`propsNarrow`), üç Görünüm komutu ⋯'de mi (`viewMore`), üç liste tek “Özellikler” listesine katlandı mı (`propsFolded`) |
| `toolbar.texts` | Çubuğun adı, ⋯'nin adı ve ipucu |
| `fields` | Listelerin değerleri: “Katmana göre”, adlarıyla sekiz çizim rengi, çizgi tipleri, kalınlıklar (`0.25 mm` yazılışıyla), ölçekler (`1:1000`) |
| `toolbox.groups`, `toolbox.constants` | Araç kutusunun grupları sırasıyla adlarıyla; kenar payı (8 px), yapışma uzaklığı (14 px), en çok sütun (6), kenardan sürüklenince imlece göre yeri (`undock`: imleç eksi bu) |
| `toolbox.texts` | Araç kutusunun sözleri; grubun katlama ipucu `{ sample, text }` |
| `toolbox.columns` | Saklanan sütun sayısının okunuşu: 3 üçtür, gerisi iki |
| `toolbox.fit` | Sütunlar: seçilen sayıdan başlayıp araçlar yüksekliğe sığana dek birer artar, en çok 6; `fitsFrom`: araçlar bu sütun sayısından itibaren sığar (`null`: hiç sığmaz) |
| `toolbox.place` | Kayan araç kutusunun yeri: istenen `x`, `y`, kutunun `w`, `h`, çizim alanının `hostW`, `hostH` → kenar payı içinde kalan, kenara payı 14 px'ten az kalınca paya yapışan yer (`<` 14; tam 14'te kalır) |
| `toolbox.undock` | Kenara sabitliyken tutamaçtan sürüklenince araç kutusunun yeri: imlecin çizim alanındaki yeri eksi `undock` |
| `toolbox.shown` | Araç kutusu görünür mü: klasik arayüzde `toolboxVisible`, şeritte `ribbonToolbox` |
| `menuRows` | Bir komutun menü satırı: aç/kapa komutunda simge yerine onay; tema (değiştirme dışında), çizim motoru, sembol kipi ve çalışma modu radyodur ve simgesini tutar; araç eylem gibi okunur (simge, onay yok). `checked: null` komutun aç/kapa durumu olmadığı demektir |

## Kurallar

- Her şey tam değerle karşılaştırılır.
- Web'in bugünkü kuralları ve sözleri yazılıdır; biri değişince bu dosya, iki çalıştırıcı ve bu belge birlikte değişir.
- Beklenen değeri hataya göre yenilemek yasaktır (CLAUDE.md §9.4).
