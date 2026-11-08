# 2800 — Paftada haritanın gösterdiği noktaların ölçme kalitesi

Tarih: 2026-11-08. plan/map-cad.md MAP-18, dördüncü tur. Durum: taslak. ADR 2400'ün “Sonra”sındaki “notun haritanın
kapsadığı noktalarla sınırlanması (bağlı haritanın çerçevesi)”.

İlgili:
- [ADR 2400](2400-sheet-survey-quality-note.md): paftada ölçme kalite notu;
- [ADR 1900](1900-sheet-north-facts.md): kuzey bilgisi; bağlı haritanın ya da ilk haritanın değeri.

## Dayanak

- **ISO 19157 / ISO 19115, veri kalitesinin kapsamı** (OGC GeoAPI'nin
  [DataQuality](https://www.geoapi.org/snapshot/javadoc/org.opengis.geoapi/org/opengis/metadata/quality/DataQuality.html)
  belgesi, 8 Kasım 2026'da okundu): kalite değerlendirmesinin kapsamı uzamsal ya da zamansal bir yayılımla ya da ortak
  bir nitelikle belirlenen veridir. Bütün veri kümesi yerine bir alt küme olabilir. Pafta için alt küme, haritanın
  gösterdiği yerdir.
- **ADR 2400 §4:** not proje düzeyindeydi. Pafta projenin bir bölümünü gösterse de bütün kabul noktalarının en büyük
  hatasını söylüyordu. Bu doğruluğu abartmaz, ama başka yerin noktası paftanın notunu büyütebilir; paftada hiç nokta
  yokken bile not nokta sayardı.
- **ADR 1900:** kuzey bilgisi metnin bağlı olduğu haritanın, yoksa paftanın ilk haritasının merkezinden okunur. Kalite
  notu aynı kuralı izler.
- **Pafta çekirdeğinin kuralı** (tasarım §7): değeri olmayan ad kâğıda `‹ad?›` olarak basılır, ön denetim bunu söyler.

## Karar

1. **Noktanın yeri:** kabul noktasının çizimdeki nesnesi. Arşiv nesnenin kalıcı kimliğini tutar; yer projenin
   sistemindedir. Nesnesi çizimde olmayan nokta hiçbir haritada değildir.
2. **Haritanın gösterdiği yer:**
   - içeriğin dört köşesi yerde (`MapPrim::ground_corners`): çerçeve, haritanın ölçeğinde, merkezinin çevresinde,
     görünümün dönüklüğüyle;
   - çerçevenin kâğıttaki dönüklüğü gösterilen yeri değiştirmez;
   - kenar içeridir; karar kesin yönelim yüklemiyle (`orient2d`) verilir. Dönük haritada çevre kutusunun köşeleri
     dışarıdadır;
   - özel kırpma şekli ve atlas nesnesine kırpma bu ADR'de yoktur: kapsam çerçevedir.
3. **Haritanın sözleri** (`kentos_project::survey_facts::survey_facts_where`): ADR 2400'ün kuralları, yalnız
   haritanın gösterdiği noktalarla:
   - ağ dengelemesi yalnız o noktaların dayandıklarını anar;
   - nokta göstermeyen haritanın notu “Haritanın içinde GNSS ile kabul edilmiş nokta yok.”, öbür değerleri boştur;
   - arşivsiz projede hiçbiri yoktur (değişmedi).
4. **Taşınma:**
   - barındırıcı paftayı bir kez çizer; haritaların köşelerinden her haritanın sözlerini çıkarır
     (`kentos_plot::survey_scope::map_surveys`, `SheetData.surveys`), sonra yeniden çizer. Tablo ve lejant da böyle
     doldurulur;
   - `RenderInputs.map_surveys` (pafta türü `MapSurveyInput`); atlas sayfasında `PdfAtlasPage.surveys`: her sayfanın
     yeri kendinindir;
   - baskı, ön denetim, masaüstünün pafta önizlemesi ve PDF'i aynı işlevden doldurur.
5. **Okuma:**
   - haritaya bağlı yazı o haritanın sözlerini okur;
   - bağsız yazı paftanın ilk haritasınınkini (ADR 1900'deki gibi);
   - haritası olmayan pafta projenin notunu okur;
   - paftanın, kitabın ya da projenin aynı adlı değişkeni önce gelir (değişmedi).
6. **Ön denetim `survey_quality_missing`:**
   - yazının basacağı değer boşsa söylenir: haritanınki ya da projeninki;
   - neden ayrılır: “haritanın içinde GNSS ile kabul edilmiş nokta yok” ya da “projenin GNSS arşivinde kabul edilmiş
     nokta yok”;
   - ileti basılacak yer tutucuyu söyler (`‹konum_dogrulugu?›`). ADR 2400'ün “değer boş basılır” sözü düzeltildi.
7. **Kapsam işareti:** ADR 2400'deki gibi **CAD + CBS** (pafta); arşiv Harita projesindedir.

## Kanıt

- **Bağımsız başvuru:** `scripts/fixtures/sheet_survey_scope_cases.py` → `fixtures/sheet/v1/survey-scope.json`,
  KentOS kodu yok.
  - Çizim map-survey-network.json'dur, üstüne: kn-2'nin nesnesi; kn-3 (da-2, C2); kn-4 (büyük σ); üç harita ve
    sekiz yazı.
  - Haritalar:

    | Harita | Gösterdiği | Konum | Yükseklik | Ağ dengelemesi |
    |---|---|---|---|---|
    | A | kn-1, kn-2 | ±0,5 cm | ±0,9 cm | da-1 (C3) |
    | B (45° dönük) | kn-3 | ±1,0 cm | ±1,2 cm | da-2 (C2) |
    | C | — | — | — | — |

  - kn-4 B'nin çevre kutusunda, dönük çerçevenin dışındadır. Sayılsaydı B ±2,8 cm derdi.
  - Bağsız yazı A'nınkini okur. C'nin konum yazısı `‹konum_dogrulugu?›` basar; tek bulgu odur.
- **Rust:**
  - `kentos_project` (`survey_facts::tests`): yalnız kn-2, yalnız kn-1, hiçbiri;
  - `kentos_plot` (`survey_scope::tests`): kenar içeride, iki yöndeki köşeler, büyük koordinatta 1 µm dışarısı dışarıda;
  - `kentos_plot` (`tests/all/survey_facts.rs`): dışa aktarmanın iki geçişiyle her haritanın sözleri ve her yazı
    başvurununki; ön denetim yalnız C'nin konum yazısını söyler;
  - eski kalite notu, kuzey ve atlas testleri aynen geçer; pafta çekirdeğinin testleri geçer.
- **PDF:** `.run/gui-evidence/harita-kalitesi-2800/pafta.pdf`; pdftotext sözleri başvurununki gibi okur.
- **MCP** (`crates/native/mcp/tests/map_survey_scope.rs`): `file.preflight.sheets` yalnız C'nin yazısını, yer tutucuyu
  anarak söyler. ADR 2400'ün MCP testi aynen geçer.
- **Masaüstü** (`sheets::map_survey_scope_tests`): pafta önizlemesinin yazıları başvurununki. Çapraz denetimler geçer.
  Resimler `.run/gui-evidence/harita-kalitesi-2800/`.

## Sonra

- Haritanın özel kırpma şekliyle ve atlas nesnesiyle sınırlanması.
- Total station ve nivelman ölçmelerinin kalitesi (ADR 2400 Sonra).
- Atlas sayfalarının kaynak dökümü (ADR 1320 Kalan).
- Web (TODOS §0.2).
