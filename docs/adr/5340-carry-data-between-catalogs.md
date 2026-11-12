# 5340 — Kataloğa taşı: verinin rapordan yeni kataloğun katmanlarına taşınması

Tarih: 2026-11-12. plan/map-cad.md MAP-04, on birinci tur. Durum: taslak. MAP-04'ün sıradaki işi (ADR 4940'ın “Sonra”sı):
rapordan iki katalog arasında göç adımları üretmek ve uygulamak; bölünen sınıfın nesnelerinin kendi kodlarıyla
dağıtılması.

İlgili:
- [ADR 4940](4940-catalog-migration-report.md): Katalog geçiş raporu (`cad.profiles.compare`), sınıf izlemesi;
- [ADR 0541](0541-data-profiles.md): veri profili ve yazıcısı (`cad.profiles.apply`);
- [ADR 1360](1360-field-codes-and-kodla-ciz.md): saha kodları, Kodla çiz'in kod alanı (`Kod`).

## Dayanak

12 Kasım 2026'da okundu; not `.run/research/map-cad/uygulama/MAP-04.11.md`.

- **ArcGIS Pro, Append:**
  - alan eşlemesiyle hedef şemaya ekler, eşlenmeyen kaynak alanı taşınmaz;
  - hedefin şeması değişmez;
  - alt tür parametresi her satıra tek kod verir; satır satır dağıtım yok.
- **QGIS, Refactor fields:** alanları eşlemeyle yeni katmana yazar, kaynak kalır.
- İkisinde de nesnenin kendi koduna göre birden çok hedefe dağıtılması belgelenmemiş.
- **BÖHHBÜY Ek-2 §7:** eski kodlar “*” ile kullanılabilir; kod göçünün kuralı yok, eşleme kurumundur.

## Karar

1. **Yeni komut `cad.profiles.carry` v1, Kataloğa taşı** (belgeyi değiştirir; tek geri alma adımı “Kataloğa taşı”, ya
   hep ya hiç; kayıtlı çağrı):
   - **Girdi:** `cad.profiles.compare`'inki (from/fromProfile, to/toProfile, drawing), `field` (kod alanı; verilmezse
     `Kod`), `expectedRevision`.
   - **Sınıf izlemesi raporunkidir.** Eski sınıfın katman yolunun götürdüğü katmandaki her nesne:
     - aynı, değişen, yeniden adlandırılan, birleşen sınıfın nesnesi tek yeni sınıfa;
     - bölünen sınıfın nesnesi, kod alanının baştaki büyük harflerini (KN12 → KN) kodlarında taşıyan ilk yeni sınıfa;
       kodu olmayan ya da hiçbir yeni sınıfta olmayan nesne yerinde kalır;
     - kalkan sınıfın nesneleri yerinde kalır;
     - zaten hedefinin katmanında olan nesne taşınmaz.
   - **Katmanlar:** nesne alan her yeni sınıf (yeni kataloğun sırasıyla), tek başına (sınıfı ve alanlarının bağladığı
     değer listeleri), `cad.profiles.apply`'ın yazıcısıyla yazılır: katmanı yoksa yolundaki gruplar ve katman alanlarıyla
     açılır, varsa eksik alanları eklenir (başka türde alan korunur, söylenir). Sonra nesneler katmanlarına gider.
   - **Değerler nesneyle taşınır.** Hiçbir değer silinmez ya da yeniden kodlanmaz; kalkan alan ve kod kararı raporda
     kalır.
   - **Cevap:** nesnesi olan her eski sınıfın taşınan ve kalan nesne sayısı, toplamlar, açılan katmanlar ve alanlar.
     Plan yazmadan aynı sayıları verir (açılacak katmanlar plana yazılmaz).
   - **Uyarılar:**
     - `catalog_objects_left`: yerinde kalanlar sınıfıyla ve nedeniyle (“Ağaç (yeni katalogda yok) 1”);
     - yazıcının `profile_field_differs` ve `profile_domain_differs`'i.
   - **Retler** (sırasıyla):
     - `invalid_revision`, `revision_conflict`;
     - raporunkiler (`unknown_profile`, `invalid_profile`);
     - `layer_locked` (nesnenin bıraktığı ya da gittiği katman kilitli);
     - `layer_refused` (yeni sınıfın yolu bir gruba ya da katmanın içine çıkıyor);
     - `nothing_to_set` (taşınacak nesne yok).
2. **Masaüstü:** Katalog geçiş raporu'nda **Kataloğa taşı** düğmesi (yeni şerit düğmesi yok; pencere Ölçme › Noktalar ▾
   ve CBS Veri › Standart'taki yerinde).
   - Taşınan nesne ve açılan katman sayısını ve uyarıları söyler.
   - Raporu yeniden okur: taşınan sınıfların nesneleri artık eski katmanda görünmez.
3. **Python ve MCP:** `kentos.cad.profiles.carry`; MCP aracı `cad.profiles.carry`.
4. **`.kcad` şeması değişmedi.** Web ertelendi (TODOS §0.2).
5. **Kapsam:** Harita + CBS (ADR 4940 gibi).

## Sınırlar

- **Alan ve kod eşlemesi yok:** kalkan alanın değeri nesnede kalır, yeni sınıfın şeması onu tanımaz; kalkan kodlar
  değiştirilmez. Kurumun eşlemesi (eski kod → yeni kod) ayrı bir girdi olarak gelmeli.
- **Geometrisi değişen sınıf:** nesneleri taşınır ama geometrileri çevrilmez (çizgi alan olmaz); rapor karar ister.
- **Kodsuz bölünen sınıf:** kodu olmayan nesne bölünmez, yerinde kalır.
- **Kabul:** gerçek pencerede fare ve klavyeyle kabul yapılmadı.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/profile_carry_cases.py`; KentOS kodu yok; sınıf izlemesi raporun bağımsız
  başvurusundan, katman yazımı `cad.profiles.apply`'ın bağımsız başvurusundan):
  - 26 durum;
  - **taşıma:** bölünen, yeniden adlandırılan, birleşen, yeri değişen sınıf; kalkan sınıf; numaralı kod; kodsuz ve
    eşleşmeyen nesne; başka kod alanı; bulunmayan kod alanı; var olan hedef katman; büyük harfli yol; gizli katman; yalnız
    birleşen sınıflı katalog; eski sınıfa katılan; kodu düşen sınıf;
  - **yazıcının uyarıları:** başka türde alan, farklı değer listesi;
  - **retler:** kilitli kaynak, kilitli hedef, gruba çıkan yol, aynı katalog, ters yön, sınıf katmansız çizim, bütün
    sınıflar kalkıyor, sürüm geçersiz, çakışma, eski katalog yok, geçersiz yeni katalog.
  - Bozma denemesi: bir sınıfın taşınan sayısı ve bir nesnenin katmanı değiştirilince koşucu tam o ikisini yakalar.
- **Python** (`python/tests/test_catalog_carry.py`): bağımsız `.kcad` okuyucusuyla, nesnelerin kendi kodlarından
  beklenen katman adları; kalkan sınıfın nesnesi yerinde; geri alma. Python ortak durumları geçer.
- **MCP** (`crates/native/mcp/tests/catalog_carry.rs`).
- **Masaüstü** (`catalog_carry_tests`): düğme ortak durumun sayısını söyler, kontrol noktası yeni katmanına gider, tek
  geri alma. Çapraz denetimler 139 test. Resimler `.run/gui-evidence/katalog-tasi-5340/`.

## Sonra

- Kurumun alan ve kod eşlemesinin girdisi (eski kod → yeni kod, alan adı → alan adı) ve değerlerin çevrilmesi.
- Geometrisi değişen sınıfın nesnelerinin çevrilmesi (kapalı çizgiden alan).
- AI çıkarımı; gerçek çıkarımın adaylarıyla ölçüm (sahibin işleri).
- Web (TODOS §0.2).
