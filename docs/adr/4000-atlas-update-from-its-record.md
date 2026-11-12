# 4000 — Atlasın kaynak kaydı ve önceki atlasa göre değişen sayfalar

Tarih: 2026-11-09. plan/map-cad.md MAP-18, yedinci tur. Durum: taslak. ADR 3600'ün “Sonra”sındaki “Atlasın güncelleme
yayını: önceki çıktının kaynak dökümüyle değişen sayfaların bulunması. Kayıt bugün yalnız cevaptadır; çıktının yanında
bir kayıt dosyası gerekir.”

İlgili:
- [ADR 0534](0534-atlas-pdf-pinned-revision.md): atlas PDF'i sabitlenen durumdan;
- [ADR 1320](1320-sheet-sources-and-update-publication.md): paftanın kaynak dökümü, `file.publish.status`'un
  `changes`'i, güncelleme yayını;
- [ADR 3600](3600-atlas-page-sources.md): atlas sayfalarının kaynak dökümü ve nesne özetleri.

## Dayanak

- **QGIS atlas ve ArcGIS Pro harita serisi** (ADR 1320 ve 3600'ün okuması): hangi sayfanın değiştiğini ürün söylemez;
  dışa aktarılacak sayfaları kullanıcı seçer.
- **ADR 1320 §2 ve §3:** paftalar için KentOS'un kuralı: değişen sayfayı ürün, kaynak dökümünden ve içerik özetinden
  bulur. Değişimin nedeni üç listedir: `changed` (özeti değişen), `added` (yeni listelenen), `removed` (artık
  listelenmeyen). Ayrıca projenin parçaları (`parts`) sayılır. Pafta yayınının kaydı teslim klasöründeki `yayin.json`'dur.
- **KentOS bugün** (ADR 3600):
  - atlasın cevabı her sayfanın kaynaklarını ve nesnelerin özetlerini veriyordu, ama yalnız cevapta kalıyordu;
  - sonraki bir çalışma, önceki atlasın hangi sayfasının değiştiğini bilemiyordu.

## Karar

1. **Kaynak kaydı:** `file.export.atlas`'ın execute'u PDF'in yanına `<ad>.kaynaklar.json` yazar (`atlas.pdf` →
   `atlas.kaynaklar.json`; adı `.pdf` ile bitmeyen dosyada ada eklenir).
   - Biçim `kentos.atlas-sources/1`: PDF'in adı ve SHA-256'sı, pafta, gün, sabitlenen durum (`pin`), sayfalar (kaynaklarıyla)
     ve listelenen nesnelerin özetleri.
   - Önce kayıt, sonra PDF yazılır: kayıt yazılamazsa hiçbir şey yazılmaz.
   - Cevapta kaydın yolu (`record`), plan da söyler.
2. **`previous`** (mutlak yol): önceki atlasın kaydı. Verilince cevaba `changes` eklenir, ADR 1320'nin kuralıyla:
   - sayfa nesnesiyle eşlenir. Nesnesi önceki atlasta sayfa değilse `new`;
   - listelediği nesnelerden özeti değişenler `changed`, yeni listelenenler `added`, artık listelenmeyenler `removed`
     (kalıcı kimlikleriyle, listelendikleri sırayla). Biri doluysa sayfa `changed`, yoksa `same`;
   - projenin sayfaların okuduğu parçalarından biri değiştiyse (`settings`, `layers`, `styles`, `sheets`: sabitlenen
     durumun damgaları) `parts`'ta söylenir ve her sayfa `changed`'dır;
   - nesnesi artık sayfa olmayan önceki sayfalar `gone`'dadır (adı ve nesnesi);
   - ayrıca önceki PDF'in adı, SHA-256'sı ve günü.
3. **“Değişti” sayfanın kaynaklarının değiştiğidir.** Yalnız çizilmeyen bir öznitelik (ör. formun gizlediği malik)
   değiştiyse de sayfa `changed`'dır. Sayfa baytlarının özeti atlas PDF'inde sayfa sayfa tutulmaz (ADR 1320'nin pafta
   yayını tutar). Kural nedeni söyler; sayfanın görünüşünün değiştiğinin kanıtı değildir.
4. **Ret** `no_previous` (`previous`): yol mutlak değil; dosya okunamıyor ya da kayıt değil; başka bir paftanın atlasının
   kaydı. Ret dosya yolu ve var olan dosya denetimlerinden sonra, sayfalar yapılmadan önce gelir.
5. **Masaüstü** (Atlası PDF yap):
   - **Önceki kayıt…** düğmesi: kaydı seçtirir, adını ve Kaldır'ı gösterir;
   - önizlemede “Önceki atlasa (atlas.pdf, gün) göre: n sayfa değişti, m yeni, k aynı” satırı; varsa artık olmayan
     sayfalar ve değişen parçalar;
   - sayfa listesinde **Değişim** sütunu: aynı, değişti (neden sayısı), yeni;
   - dışa aktarma kaydı verir. Yazıldı iletisi kaydın adını söyler.
6. Yalnız değişen sayfaların PDF'e yazılması bu ADR'de yoktur (Sonra).
7. **Kapsam işareti:** ADR 3600'deki gibi **CAD + CBS** (pafta).

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/atlas_cases.py`; KentOS kodu yok): yalnız atlas haritalı kurulumda (`plain`)
  kural elle. Bir sayfa, önceki ve şimdiki listesinde düzenlenen parsel varsa `changed` (o nesneyle), yoksa `same`.
  Üç yeni durum (40):
  - execute kaydı yazar, 2 numaralı parselin maliki değişir; önceki kayda göre plan: onu listeleyen altı sayfa
    değişti, öbürleri aynı;
  - 3 numaralı parsel iptal edilir: sayfası `gone`'dadır, sonraki sayfalar bir sıra kayar, komşuları onu değişmiş
    listeler;
  - olmayan kayıt `no_previous`.

  Koşucu girdilerde de `$uidOf:N` çözer. Eski 37 durum bayt bayt aynı bırakıldı: bu Mac'in PROJ'u başka sistemdeki iki
  durumun `$near` basamaklarını oynatıyor, o ikisi yeniden yazılmadı. Başsız koşucu 40 durumu geçer.
- **Python** (`python/tests/test_atlas_update.py`), testin kendi okumasıyla:
  - kaydın biçimi, PDF'in adı ve SHA-256'sı (PDF'in baytlarından), 100 sayfa;
  - değişikliklerin her sayfası, kaydın o parseli listeleyip listelemediğine göre `changed` ya da `same`;
  - bozuk kayıt reddedilir.
- **MCP** (`crates/native/mcp/tests/atlas_update.rs`): kaydın yolu; parselin bir özniteliği değişince sayfaların durumları
  durumunki; olmayan kayıt reddedilir. Formun gizlediği malike MCP'den yazılamaz (`hidden_field`), test başka bir
  öznitelikle yapılır.
- **Masaüstü:**
  - `exchange::atlas_tests::the_preview_says_each_page_against_the_previous_atlas`: pencereyle yazılan atlasın kaydı,
    değişiklik, önceki kayıt seçilince sayfaların durumu durumunki; Kaldır geri alır;
  - atlas pencere testleri ve çapraz denetimler geçer;
  - resimler `.run/gui-evidence/atlas-guncelleme-4000/`.

## Sonra

- Yalnız değişen ve yeni sayfaların ayrı bir PDF'e yazılması (güncelleme teslimi), sayfa numaralarıyla:
  [ADR 4400](4400-atlas-update-pdf-of-changed-pages.md)'de.
- Atlas sayfalarının bayt özetiyle “görünüşü değişti” ayrımı (ADR 1320'nin pafta yayınındaki gibi): [ADR 4820](4820-atlas-page-look.md)'de (bayt yerine çizdiğinin özeti).
- Total station ve nivelman kalitesi (MAP-18'in öbür maddesi): projede karasal ve nivelman dengelemelerinin
  doğrulukları saklanmalı.
- Web (TODOS §0.2).
