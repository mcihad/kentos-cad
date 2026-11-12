# 5040 — Plan değişikliği taslağı; sembol seçimlerinin projede saklanması

Tarih: 2026-11-11. plan/map-cad.md MAP-09, onuncu tur. Durum: taslak. ADR 4240'ın ve 4640'ın “Sonra”sındaki
“kullanıcının sembol seçimlerinin projede saklanması (şema); denetimin onları okuması” ve “plan değişikliği taslağı”.

İlgili:
- [ADR 1420](1420-zoning-plan-typed-check.md): `imar-plani` profili ve İmar planı denetimi;
- [ADR 4240](4240-zoning-symbol-from-decision.md): Sembolü karardan ver, KentOS'un sembol tablosu;
- [ADR 4640](4640-zoning-symbol-mismatch-and-error-geopackage.md): denetimde sembol uyuşmazlığı;
- docs/specs/kcad-v2.md, şema 242.

## Dayanak

- **Mekânsal Planlar Yapım Yönetmeliği** (RG 14.06.2014/29030; son değişiklik 22.01.2026/33145), metin
  `.run/research/map-cad/mevzuat/pdf/mpyy/` (10–11 Kasım 2026'da okundu; not: `mevzuat/MAP-09.10.md`):
  - m. 4: sosyal altyapı alanları eğitim, sağlık, dini, kültürel ve idari tesisler, spor tesisleri, park, çocuk
    bahçesi, meydan ve öbür açık ve yeşil alanlardır; teknik altyapı alanları enerji, su, atık, ulaştırma, haberleşme
    tesisleri ve otoparktır;
  - m. 7(1)(b): plan pafta, gösterim, plan notları ve plan raporuyla bütündür;
  - m. 9(4): plan değişikliğinde değişikliğin gerekçesini ve gereklilik analizlerini açıklayan plan raporu zorunludur;
  - m. 26(1)–(6): değişiklik plan ana kararlarını, sosyal ve teknik altyapı dengesini bozmaz; (3) sosyal ve teknik
    altyapı alanının kaldırılması ya da küçültülmesi zorunluluğa, ilgili kurumun görüşüne ve (b) yol hariç hizmet etki
    alanında eşdeğer yeni alana bağlıdır; (4) kat adedi ya da yüksekliği artıran değişiklik yörenin dokusunu ve siluetini
    gözetir; (5)(a) emsali ya da kat adedini artıran değişiklikte artan nüfusun sosyal ve teknik altyapı alanları ayrılır,
    (b) karşılıklı cepheler arası K = (Y1 + Y2)/2 + 7,00 m;
  - m. 31(3), 32, 33, 34: müellif imzası, plan işlem numarası (PİN), onay, askı ve kesinleşme planın kendi
    sürecidir;
  - m. 10(2)–(3): gösterimler Bakanlığın EK-1'indendir; ilan edilmeyen gösterim uygulanamaz.
- **Netcad Planet yardımı** (`.run/research/map-cad/uygulama/MAP-09.10.md`, resimler `MAP-09.10-resim/`,
  11 Kasım 2026):
  - Proje Özellikleri › Proje Değişkenleri planın değişiklik ve onay bilgisini (plan değişiklik tarihi, onaylayan,
    onama tarihi, onay numarası) **projede** tutar;
  - Sembol İşlemleri'nde hazırlanan sembol “Hatırla” ile sonraki kullanım için kaydedilir;
  - Plan Dönüştür kaynak planı değiştirmez, yeni dosya üretir. Önce ve sonra karşılaştırması belgede yok.
- **ArcGIS Pro, Match Layer Symbology To A Style** ve **ArcGIS Urban** planları (aynı not): değerden sembole eşleme
  katmanda ya da projede saklanır; plan senaryoları yürürlükteki veriden ayrı tutulur ve ölçütleriyle karşılaştırılır.
- **KentOS bugün:** Sembolü karardan ver (ADR 4240) kullanıcının seçimini her çalıştırmada ister; denetimin sembol
  uyuşmazlığı (ADR 4640) yalnız KentOS'un tablosunu okur. Plan değişikliği için bir kayıt yoktu.

## Karar

1. **Projede saklanan veri, `.kcad` şema 250** (`settings.zoningPlan`, JSON'u olduğu gibi; boşsa yazılmaz):
   - `symbols`: kullanıcının sembol seçimleri (`level`, `use`, `symbol`); aynı kademe ve kullanım bir kez;
   - `drafts`: plan değişikliği taslakları (`id`, `plan`, `revision`, `reason`, `changes`: `area`, `field`, `before`,
     `after`);
   - okuyucu ve yazıcı yapıyı denetler (`zoning_plan_problem`; bağımsız okuyucu `tools/kcad/kcad.py`); değişiklik tek
     geri alma adımıdır (`Op::ZoningPlan`).
   - Şema 249 başka bir dalındır; bilinmeyen sürüm örneği `broken/schema-version-251.kcad`'dir (birleştirmede yeniden
     numaralanır).
2. **`cad.zoning.symbolize` v1** (sürüm aynı, yeni alanlar):
   - `keep`: verilen seçimler projenin saklı seçimleri olur, sembollerle aynı geri alma adımında; seçim verilmezse
     proje seçim saklamaz. `keep`'te eski saklı seçimler okunmaz; cevap `kept` ile saklananları verir;
   - `keep` yoksa sıra: verilen seçim, projenin saklı seçimi (`from` `project`), KentOS'un tablosu;
   - kademenin MPYY alan sembollerinden olmayan ya da kullanımı listede olmayan saklı seçim kullanılmaz; yeni uyarı
     `kept_symbol_ignored`. KentOS sessizce yerine bir şey koymaz.
3. **`cad.zoning.check`**:
   - `symbolMismatch` projenin saklı seçimini tablodan önce okur. ADR 4640'ın sınırı (tablonun eşlemediği kullanıma
     verilen sembol denetlenmiyordu) saklı seçimle kalkar;
   - yeni bulgu `draftStale` (“Taslak eskidi”; türlerin sonunda): taslağın planı çizimde yok; değiştirdiği karar alanı
     yok; alanın değeri taslak yazılırken okunandan farklı.
4. **`cad.zoning.draft` v1** (“Plan değişikliği taslağı”; yeni komut):
   - **Girdi:** `id`, `plan` (plan sınırının `PlanKimlik`'i), `reason`, `changes` (`uid`, `field`, `value`). Aynı
     kimlikteki taslağın yerine geçer; `remove` taslağı siler.
   - **Taslak:** çizimin sürümü ve her alanın önceki değeri saklanır. Değişebilen alanlar `Kullanim`, `Nizam`, `TAKS`,
     `KAKS`, `KatAdedi`, `Hmax`, `OnCekme`, `YanCekme`, `ArkaCekme`'dir. Yürürlükteki karar alanları hiç yazılmaz.
   - **Cevap:**
     - planın karar alanları kullanıma göre önce ve sonra (`uses`, m², çekirdeğin düzlemsel alanı; toplam korunur);
     - m. 26'nın sorduğu konular (`findings`):
       - `infrastructureLess`: m. 4'ün sosyal ya da teknik altyapı kullanımının alanı küçülüyor; yol m. 26(3)(b) gibi
         dışarıda; ne kadar küçüldüğü `area`'da (m. 26(3));
       - `densityUp`: KAKS ya da kat adedi artıyor (m. 26(5)(a));
       - `heightUp`: kat adedi ya da Hmax artıyor (m. 26(4), (5)(b)).
     - Artış iki değer de sayıyken denetlenir; boş değer 0 sayılmaz.
     - Eşdeğerlik, hizmet etki alanı ve K uzaklığı ölçülmez; uygunluk iddia edilmez.
   - **Retler,** sırasıyla:
     - `invalid_revision`, `revision_conflict`;
     - `invalid_input`: kimlik;
     - `unknown_draft`: yeni; silinecek taslak yok;
     - `layer_not_found`;
     - `invalid_input`: plan boş;
     - `unknown_plan`: yeni; plan sınırı yok;
     - `invalid_input`: gerekçe boş; m. 9(4) iletide;
     - `invalid_input`: değişiklik yok;
     - `changes[i]`:
       - `invalid_input`: aynı alanın aynı alanı iki kez;
       - `invalid_uid`, `entity_not_found`, `not_a_decision`;
       - `invalid_input`: başka planın alanı; taslağın değiştirmediği alan; boş değer; listede olmayan kullanım ya
         da nizam; 0'dan küçük ya da sayı olmayan değer; zaten o değer (sayılar sayı olarak karşılaştırılır).
   - **Taslak onaylı değişiklik değildir.** Plan raporu, müellif imzası, PİN, onay ve askı KentOS dışındadır; komut ve
     pencere bunu söyler.
5. **Masaüstü:**
   - İmar planı denetiminde **Plan değişikliği taslağı…** penceresi açar. Kit'in Formu:
     - Taslak: Taslak, Kimlik, Plan, Gerekçe;
     - Değişiklik: Karar alanı, Alan, Yeni değer (yürürlükteki değer yardımda); Değişikliğe ekle, Satırı çıkar;
     - Kaydet: Taslağı kaydet, Taslağı sil.
   - Plan form doldurulurken söylenir. Kitin veri tablosu tek seferde Değişiklikler, Kullanımlar ya da Mevzuatın
     sordukları'nı gösterir.
   - **Sembolü karardan ver** projenin saklı seçimleriyle açılır. Kaynak “Projede” olur. **Seçimleri projede sakla**
     anahtarı ilk değeriyle açıktır. **Seçimi kaldır** satırın seçimini tabloya bırakır.
   - İmar denetiminin `draftStale`'i Plan ve kademe grubundadır.
6. **MCP:** `cad.zoning.draft` taslağın yerine geçtiği ve sildiği için yıkıcı listesindedir.
7. **Kapsam işareti:** **Harita + CBS**. Bakanlığın şemasına ya da mevzuata uygunluk iddiası yok.

## Sınırlar

- Taslak yalnız var olan karar alanlarının değerlerini değiştirir. Alan bölme, yeni alan, geometri değişikliği ve
  plan notu değişikliği taslakta yok.
- Taslağı yürürlükteki plana uygulayan bir komut yok: onaydan sonra plan kendi kayıt sürecinde değişir. Önce ve sonra
  paftaları çizilmez.
- `Belirtilmeyen` ve `PlanNotlari` taslakta değişmez.
- m. 26(2)'nin standartları (EK-2) ve m. 26(6)'nın yol genişlikleri denetlenmez.
- Kullanıcı başına (proje dışı) sembol seçimi saklanmaz. Saklı seçimin EK-1 sürümü ayrıca yazılmaz: seçim kitaplığın
  sembol kimliğidir.

## Kanıt

- **Bağımsız başvurular** (KentOS kodu yok):
  - `scripts/fixtures/zoning_draft_cases.py`, 33 durum:
    - park ticarete, yoğunluk ve yükseklik artışı, eğitim parka, azalan yoğunluk, boş önceki değer, virgüllü sayı;
    - aynı kimlikli taslak yenisiyle değişir; başka kimlikli eklenir; silme ve geri alma; plan ve doğrulama;
    - 22 ret.
  - `zoning_symbols_cases.py`: 6 yeni durum (32): keep, saklı seçim önce, verilen saklıdan önce, boş keep, saklı seçim
    değişir, geçersiz saklı seçim uyarısı.
  - `zoning_check_cases.py`: 5 yeni durum (56): saklı konut ve park seçimiyle uyuşmazlık, okunmayan saklı seçim,
    eskimeyen ve eskiyen taslak.
  - Rust ortak durumları ve Python (`test_command_cases`) geçer; yetenekler testi her kodu sayar.
- **KCAD:** `zoning-plan.json/.kcad` ve dört bozuk örnek (`kcad_v2_reference.py`, bağımsız okuyucu `kcad.py`
  `--check`'te); `fixtures`/`addresses` testleri geçer.
- **Python** (`python/tests/test_zoning_plan_draft.py`), testin kendi shoelace'i ve okuyucusuyla:
  - taslak yürürlükteki planı değiştirmez; dosyada önceki değerle okunur;
  - kullanımların toplamı korunur, park m. 26(3)'te;
  - değişen alan `draftStale` verir;
  - saklı seçim kaydedilip açılınca `project` kaynağıyla okunur.
- **MCP** (`crates/native/mcp/tests/zoning_draft.rs`): araç yıkıcı; taslak, bulgu, karar alanı aynı, denetim temiz;
  saklı seçim sonraki koşuda okunur; geri alma.
- **Masaüstü:**
  - `zoning_draft_window_tests` (2): formdan yazılan taslak durumun cevabı ve sakladığı; karar alanları aynı; geri
    alma; saklı taslak forma yüklenir ve silinir;
  - `zoning_symbols_window_tests::a_kept_choice_is_read_when_the_window_opens_again`.
  - Resimler `.run/gui-evidence/plan-taslak-5040/` ve `map-09-10/`.
- **Bozma denemesi:** taslak durumlarında iki beklenen değer değiştirilince koşucu yalnız ikisini yakalar.

## Sonra

- Taslakta geometri değişikliği (alan bölme, yeni karar alanı) ve plan notu; taslağın önce ve sonra paftaları.
- m. 26(2)'nin EK-2 standartları ve m. 26(6)'nın yol kuralları denetimde.
- Resmî şema ve eşleme (MAP-09'un kalanı).
- Web (TODOS §0.2).
