# 4640 — İmar denetiminde sembol uyuşmazlığı; hata katmanlarının GeoPackage'a yazılması

Tarih: 2026-11-10. plan/map-cad.md MAP-09, dokuzuncu tur. Durum: taslak. ADR 4240'ın “Sonra”sındaki “imar denetiminde
sembol ile karar uyuşmazlığı” ve ADR 3840'ın “Sonra”sındaki “Hata katmanının GeoPackage'a yazılması (İşlem penceresinin
Sonucun yeri ile; ADR 2200 Sonra)”.

İlgili:
- [ADR 4240](4240-zoning-symbol-from-decision.md): Sembolü karardan ver, KentOS'un sembol tablosu;
- [ADR 3840](3840-zoning-check-in-processing-and-error-layer.md) ve
  [ADR 2200](2200-topology-findings-on-an-error-layer.md): imar ve topoloji bulguları hata katmanında;
- ADR 0508 §4, 0592: İşlem'in sonucu bir GeoPackage'a ya da PostGIS'e (sink).

## Dayanak

- **ADR 4240'ın okuması** (QGIS'in Categorized renderer'ı): sembol verinin ayrık değerinden gelir; KentOS'ta karar
  alanının sembolü kararından (kademe ve kullanım) verilir, sembolden karar okunmaz. Sonradan elle değişen sembol kararı
  göstermeyebilir; bugün bunu söyleyen bir denetim yoktu.
- **OGC GeoPackage Encoding Standard**, [§2.1.3 Geometry Encoding](https://www.geopackage.org/spec/) (10 Kasım 2026'da
  okundu): nesne geometrisi StandardGeoPackageBinary'dir. Başlık sırasıyla:
  - “GP” sihirli baytları, sürüm baytı (0: sürüm 1);
  - bayraklar: X genişletilmiş tür, Y boş geometri, E zarf göstergesi (0: yok; 1: 32; 2 ve 3: 48; 4: 64 bayt), B
    başlığın bayt sırası;
  - srs_id ve zarf;
  - ardından WKB geometri gelir; WKB'nin kendi bayt sırası bayrağı vardır.

  Bağımsız okuyucu bu tanımla yazıldı.
- **KentOS bugün** (ADR 3840, 2200): İmar planını denetle ile Topolojiyi kurallarla denetle'nin “Bulguları hata
  katmanına yaz”ı çizime yazıyordu. İşlem penceresinin “Sonucun yeri” (çizim, GeoPackage dosyası, PostGIS) her araçta
  vardı, ama hata katmanlarının dosyaya yazılması sınanmamış, kanıtlanmamıştı.

## Karar

1. **`cad.zoning.check`'e `symbolMismatch`** (“Sembol uyuşmazlığı”; sürüm aynı, yeni bulgu türü, türlerin sonunda):
   - kural Sembolleri karardan ver'inkidir (ADR 4240): alanın `Plan`'ı plan sınırını adlandırır; adlandırmıyorsa ve tek
     plan sınırı varsa o plandır; kademe planın `Seviye`'si;
   - kullanıcının seçimi olmadan KentOS'un tablosu okunur;
   - bulgu, karar alanının kitaplık sembolü kararının sembolü değilse çıkar. Sembolü olmayan alan, tablonun eşlemediği
     kullanım, planı ya da kademesi olmayan alan değerlendirilmez;
   - sözü: “Karar alanının sembolü “Park”, kararı “Ticaret alanı” (Uygulama imar planı) “Ticaret alanı” sembolünü verir:
     sembol kararı göstermiyor. Sembolleri karardan ver ile düzeltin.”;
   - okuma kilitli katmanda reddedilmez: kural yazmadan çalışır (`zoning_symbolize::rows`).
2. **Masaüstü** (İmar planı denetimi): bulgu **Karar ve değer** grubunda; Seç ve göster alanı seçer. İşlem'in İmar
   planını denetle aracı aynı bulguyu tabloda verir (bölgesi yoktur, “yeri yok” sayılır).
3. **Hata katmanlarının GeoPackage'a yazılması:** İşlem penceresinde “Sonucun yeri: GeoPackage dosyasına” ile
   “Bulguları hata katmanına yaz” birlikte seçilince:
   - araç çizimin bir kopyasında koşar; hata katmanı dosyada katmanın adıyla bir tablo olur. İmar bulguları alan
     (POLYGON), topoloji bulguları nokta (POINT) olarak, projenin SRID'iyle ve bulgunun alanlarıyla yazılır;
   - çizim değişmez;
   - ileti: “… hata katmanına yazıldı. Sonuç imar-bulgulari.gpkg dosyasına yazıldı: İmar bulguları (2 satır).”
   - Kod değişmedi: sink ADR 0508 §4'teki gibidir. Bu ADR yolu kanıtlar ve belgeler.
4. **Kapsam işareti:** ADR 4240 ve 3840 gibi **Harita + CBS**.

## Sınırlar

- Kullanıcının kademe ve kullanım seçimleri projede saklanmadığı için denetim yalnız KentOS'un tablosunu okur. Tablonun
  eşlemediği bir kullanıma kullanıcının verdiği sembol değerlendirilmez.
- PostGIS'e yazma bu dilimde sınanmadı: sink aynıdır, ama test veritabanı gerekir.
- Yasal sonuç ya da resmî uygunluk iddia edilmez.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/zoning_check_cases.py`; kural elle, sembol tablosu
  `zoning_symbols_cases.py`'nin kitaplıktan kendi türettiği; KentOS kodu yok): 4 yeni durum (51):
  - park sembollü ticaret alanı bulgudur; kendi sembollü park, tabloda eşi olmayan konut ve sembolsüz alan değerlendirilmez;
  - sembolü kararınınki: bulgu yok;
  - Plan yazılmamış alan tek plan sınırının kademesiyle;
  - nazım planın spor sembollü sanayi alanı.

  Eski 47 durum aynı. Rust ve Python geçer. Bozma denemesi: aynı sembol de bulgu sayılınca koşucu yakalar.
- **Python** (`python/tests/test_zoning_symbol_and_error_geopackage.py`), testin kendi kurallarıyla:
  - sembolü kitaplıktan kendi bulur: yalnız S1;
  - imar ve topoloji hata katmanları GeoPackage'a yazılır, çizim değişmez;
  - dosya sqlite3 ve testin kendi GeoPackage geometri okuyucusuyla okunur: imar bölgeleri alan, alanları (testin
    shoelace'i) ortak durumun bölgeleri; topoloji bulguları nokta, anahtarları denetiminkiler; srs_id projeninki.
- **MCP** (`crates/native/mcp/tests/zoning_symbol_geopackage.rs`): S1'in bulgusu; sink'li koşu yedi bölgeyi POLYGON
  tablosuna yazar, çizim aynı kalır.
- **Masaüstü:**
  - `zoning_window_tests`: bulgu, sözü ve grubu; Seç ve göster S1'i seçer;
  - `processing::geometry_tests::the_zoning_error_layer_goes_to_a_geopackage`: Sonucun yeri GeoPackage, dosya yazılır,
    ileti söyler, çizim değişmez.

  İmar, İşlem ve çapraz denetimler geçer. Resimler `.run/gui-evidence/imar-sembol-4640/`.

## Sonra

- Kullanıcının sembol seçimlerinin projede saklanması (şema); denetimin onları okuması.
- Plan değişikliği taslağı; resmî şema ve eşleme (MAP-09'un kalanı).
- Web (TODOS §0.2).
