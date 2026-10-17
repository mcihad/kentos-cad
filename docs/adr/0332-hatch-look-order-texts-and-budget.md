# ADR 0332: Taramanın saydamlığı ve sırası, boş bıraktığı yazılar, bütçe ve iptal

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §12 `CAD-HATCH-09`, `10`, `11` (sahibin sözü: “Hatch
  bitene kadar durma”; 2–3 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0186 (ilişkili tarama, Yazılar), 0175 (bağlı yazı), 0260 (nesnenin görünüşü: saydamlık, katman içi
  sıra), 0310 (çizim sırası ve maske), 0145 (yazının zemini), 0306 (Denetle), 0253 §9 (uzun işler), 0062 (yoğun desenin reddi),
  0331 (desenler).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Taramanın saydamlığı ve çizim sırası yalnız sonradan Öznitelikler'den ve `cad.entities.order`'dan verilebiliyordu; dolu
tarama, sınırıyla aynı katmandaysa yeni nesne olarak duvarların üstüne çiziliyor ve onları örtüyordu (AutoCAD'in HPDRAWORDER'ı
“sınırın arkasına” der). Yazının zemini (ADR 0145) ile alanın maskesi (ADR 0310) iki ayrı özelliktir ama taramayla ilişkileri
yazılı değildi.

Taramanın boş bıraktığı yazılar (ADR 0186'nın Yazılar seçeneği) taşınınca boşluk izliyordu; ama taramaya bağlı bir yazı (bağlı
etiket) taramadan sonra yeniden yazılmıyordu: belge önce bağlı yazıları, sonra taramaları izliyor, yeniden kesilen taramanın
etiketi eski yerinde kalıyordu. Taramanın kendi etiketini boş bırakması bir döngüdür (etiketin yeri taramadan okunur, boşluk
taramayı değiştirir); silinen bir yazı listeden sessizce düşüyordu, dosyadan gelen kayıp yazı Denetle'de taramanın bütün
ilişkisini koparıyordu.

Çok yoğun desen bölge başına 20 000 çizgiyle reddediliyordu ama Çoklu tara ya da 1000 noktalı bir komut toplamda sınırsızdı;
komut iş olarak çalışırken durdurulamıyordu. Ekranda sık desen (3 pikselden sık) düz bir tona dönüyordu; bunun paftayı
etkilemediği sınanmamıştı.

## Karar

### 1. Saydamlık ve sıra (`CAD-HATCH-09`)

`cad.entities.hatch`'e iki alan:

- `transparency` (0–0,9; `invalid_transparency`): taramaların kendi saydamlığı (`ObjectLook.transparency`). Dolu, degrade ve
  desenli tarama aynı kuralla görünür: saydamlık çizim programının bütün sembol katmanlarına geçer (dolgu dahil); 0 yazılmaz.
- `drawOrder` (`HatchDrawOrder`): `none` (yeni nesne gibi, varsayılan), `back` (katmanın en küçük sırasının bir altı), `front`
  (en büyüğün bir üstü), `behindBoundary` ve `aboveBoundary` (sınırının kapalı nesnesinin, yüzünü kapatan çizgilerin ve
  adalarının taramanın katmanındaki sıralarının en küçüğünün bir altı ya da en büyüğünün bir üstü). Sıra katmanın içindedir
  (ADR 0260 §4): sınır yalnız başka katmandaysa sıra yazılmaz, `order_other_layer` taramanın katmanını sınırınkinin altına almayı
  söyler. Sırası 32 bitin sınırına gelen `order_limit`.

`cad.entities.create`'in nesnesi bunları taşıyabilsin diye `NewObject`'e `transparency` ve `order` geldi (dosya şeması değişmez:
ikisi de ADR 0260'ın alanları).

Araçta (Tarama ve Çoklu tara) Saydamlık (M, yüzde) ve Sıra (O: atama, arkaya, öne, sınırın arkasına, sınırın önüne; menüsüyle);
Özellik al saydamlığı da alır.

Yazının zemini ile maske ayrı kalır: zemin yazının kendi özelliğidir, yazının kutusunu çizim alanının rengiyle doldurur ve her
zaman yazının altındakini örter; maske bir alanın görünüşüdür, katmanın sırasında kendinden önce çizilenleri örter (taramayı
örtmesi için tarama ondan önce çizilmelidir: Sıra arkaya ya da sınırın arkasına). İkisi taramanın Yazılar seçeneğinden de
ayrıdır: o, yazının kutusunu taramanın bölgesinden keser (tarama orada hiç yoktur; PDF'te de).

### 2. Boş bırakılan yazılar (`CAD-HATCH-10`)

- Belge kayıttan önce bağlı yazıları ve ilişkili taramaları sırayla izler (ADR 0175, 0186); şimdi yeniden kesilen taramaların
  bağlı yazıları da aynı adımda yeniden yazılır, onların boş bıraktığı taramalar yeniden kesilir; en çok dört tur
  (`HATCH_TEXT_ROUNDS`).
- Taramanın kendi bağlı etiketi boş bırakılmaz: kesimde listeden çıkar (`hatch_ties::is_own_label`). Böylece turlar biter.
- Bir adımın sonunda, herhangi bir komutun cevabı söyler: silinen yazının taramaların listesinden çıktığını (`cutout_gone`) ve
  kendi etiketinin listeden çıkarıldığını (`hatch_text_cycle`); yazmayı belgenin kuralı yapar, söylemek komut katmanındadır
  (`hatch_notes.rs`).
- Denetle iki yeni bulgu: `missingCutout` (boş bıraktığı yazı çizimde yok; onarım onu listeden çıkarır, tarama yeniden kesilir;
  ilişki kopmaz) ve `hatchTextCycle` (kendi etiketi listede; onarım çıkarır). `brokenHatch` artık yalnız kapalı nesne, yüzün
  çizgileri ve adalar için.

### 3. Bütçe ve iptal (`CAD-HATCH-11`)

- Bölge başına sınır kalır (ailede 20 000 çizgi, ADR 0062); bir komutun bütün bölgeleri birlikte en çok 200 000 çizgi
  (`MAX_COMMAND_LINES`: paftanın bir tarama partisini kestiği en çok parça, kentos-plot'un `MAX_HATCH_PIECES`'i). Aşan
  `pattern_too_dense` “bölgeleri birkaç komutla tarayın”.
- İş olarak çalışan `cad.entities.hatch` (ADR 0253 §9) noktalar ve bölgeler arasında durdurulmayı sorar, hiçbir şey yazmadan
  `cancelled` döner (`Stop::Cancelled`), bölge bölge ilerlemesini söyler.
- Ekranın sadeleştirmesi yalnız ekranın: 3 pikselden sık çizgiler çizim hatlarında düz bir tona döner (WGSL ve GLSL'in
  `hatchFs`'i), kaynak ve pafta değişmez; paftanın vektörleri ekranda ton olan taramayı çizgi çizgi keser (test
  `a_hatch_the_screen_tints_is_plotted_line_by_line`).

## Sonuçlar

- `cad.entities.hatch.json`'a 16 durum (saydamlık, beş sıra, başka katmandaki sınır, sınır, retler; silinen boş bırakılan
  yazının uyarısı), `cad.entities.create.json`'a saydamlığın reddi, `cad.drawing.audit.json`'a kayıp yazılı tarama; bağımsız
  başvurular `region_command_cases.py`, `create_command_cases.py`, `audit_command_cases.py`, KentOS kodu olmadan. Kendi etiketi
  döngüsü ortak durumla kurulamaz (durumların çiziminde kalıcı kimlik yok); `kentos-domain`'in `hatch_texts` testlerinde.
- Taramanın bağlı etiketi taramanın bölgesinden yazılamıyorsa (etiket kuralı yazmıyorsa) bağ ADR 0175'in kuralıyla kopar.
- İptal yalnız komut iş olarak çalışırken vardır; aracın önizlemesi kendi bütçesiyle (`PREVIEW_PIECES`) sınırlıdır.

## Doğrulama

```bash
python3 scripts/fixtures/region_command_cases.py --check; python3 scripts/fixtures/audit_command_cases.py --check
python3 scripts/fixtures/create_command_cases.py --check
cargo test -p kentos-native-application --test all fixtures; cargo test -p kentos-native-application --lib budget_tests
cargo test -p kentos-domain --test all hatch_texts; cargo test -p kentos-plot --lib a_hatch_the_screen
KENTOS_SHOTS_ONLY=tarama-saydam,tarama-yazi-tasindi,tarama-sira cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
```
