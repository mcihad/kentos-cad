# 3200 — Haritanın ölçme kalitesinin kırpma şekliyle ve atlas nesnesiyle sınırlanması

Tarih: 2026-11-09. plan/map-cad.md MAP-18, beşinci tur. Durum: taslak. ADR 2800'ün “Sonra”sındaki “haritanın özel
kırpma şekliyle ve atlas nesnesiyle sınırlanması”.

İlgili:
- [ADR 2400](2400-sheet-survey-quality-note.md): paftada ölçme kalite notu;
- [ADR 2800](2800-map-scoped-survey-quality.md): notun haritanın çerçevesinin gösterdiği noktalarla sınırlanması;
- [ADR 0638](0638-atlas-systems-clip-sources-and-withheld-fields.md) §2: atlas haritasının atlas nesnesine kırpılması.

## Dayanak

- **ISO 19157 / ISO 19115, veri kalitesinin kapsamı** (ADR 2800'ün okuması): kalite değerlendirmesinin kapsamı uzamsal
  bir yayılımla belirlenen alt küme olabilir. Pafta için alt küme haritanın gösterdiği yerdir; gösterdiği yer de çizdiği
  şekildir.
- **Pafta çekirdeği:**
  - haritanın kırpma şekli (`Clip`) içerik çerçevesinin milyonda birlerinde: çerçeve, dikdörtgen ya da çokgen;
  - atlas haritası nesnesine kırpılınca nesnenin dış çizgisi çerçeveye çevrilip çokgen olarak haritanın şekline
    yazılır (inceltilerek, en çok `CLIP_CORNERS` köşe). Bağlı görünümün kırpması da öyle;
  - çizilen ve basılan şekil budur.
- **KentOS bugün** (ADR 2800 §2): kapsam çerçevenin dört köşesiydi. Kırpılmış haritanın kâğıtta görünmeyen yerindeki
  nokta notu etkiliyordu: L biçiminde kırpılmış bir haritanın kesilen köşesindeki nokta onun notunu büyütürdü.

## Karar

1. **Haritanın gösterdiği yer haritanın şeklidir:**
   - çerçeve için eskisi gibi dört köşe;
   - dikdörtgen ve çokgen için köşeler (içerik çerçevesinin milyonda birleri) kâğıtta çerçevenin içindeki yere, oradan
     haritanın kendi dönüşümüyle yere taşınır. Dört köşenin kuralıyla aynıdır: görünümün dönüklüğü yere geçer, çerçevenin
     kâğıttaki dönüklüğü geçmez;
   - atlas sayfasında ve bağlı görünümün kırpmasında şekil çizildiği gibidir (inceltilmiş dış çizgi). Çizilen yer ile
     notun yeri aynıdır.
2. **İçerilme:** yerdeki halkanın içi ya da kenarı.
   - Kenarda olmak kesin yönelim yüklemiyle (`orient2d`) ve kenarın kutusuyla;
   - içerisi sağa giden ışının kesişmeleriyle. Her kesişmenin hangi yanda olduğu da kesin yüklemle karar verilir;
   - iç bükey şekil doğru sayılır; halkanın yönü fark etmez.
3. Notun sözleri, yazıların okuması, ön denetim ve taşınma ADR 2800'deki gibidir. Yalnız kapsam değişti.
4. Komut ya da sözleşme değişmez; `file.preflight.sheets` ve basılan pafta aynı işlevden doldurulur.
5. **Kapsam işareti:** ADR 2800'deki gibi **CAD + CBS** (pafta); arşiv Harita projesindedir.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/sheet_survey_scope_cases.py` → `fixtures/sheet/v1/survey-scope.json`, KentOS
  kodu yok):
  - iki harita eklendi:
    - D: çerçevesi kn-3 ile kn-4'ü içerir, L biçimli kırpması kn-4'ün olduğu sağ üst çeyreği dışarıda bırakır;
    - E: çerçevesi kn-1 ile kn-2'yi içerir, dikdörtgen kırpması alt yarıyı tutar.
  - Başvuru köşeleri kendi dönüşümüyle yere taşır ve içerilmeyi kesin kesirlerle karar verir;
  - sonuç: D yalnız kn-3'ü gösterir, “D: ±1,0 cm ±1,2 cm”. Çerçeve sayılsaydı kn-4'le ±2,8 cm derdi. E yalnız kn-1'i
    gösterir, “E: GNSS ile 1 nokta: doğrulukları kayıtlı değil.”;
  - yazı 10, bulgu 1 (C'nin konum yazısı); A, B, C ve eski yazılar aynı.
- **Rust:**
  - `kentos_plot` (`survey_scope::tests`): eski dört köşeli denetimler (kenar, iki yön, büyük koordinatta 1 µm dışarısı)
    yeni halka denetimiyle de geçer; iç bükey L'de çentik dışarıda, çentiğin köşesi ve kenarları içeride, iki yön;
  - `kentos_plot` (`tests/all/survey_facts.rs`): beş haritanın sözleri ve on yazı başvurununki; ön denetim yalnız C'nin
    yazısını söyler.
- **MCP** (`crates/native/mcp/tests/map_survey_scope.rs`) ve **masaüstü** (`map_survey_scope_tests`): durum dosyasının
  beş haritalı paftasıyla aynen geçer. Çapraz denetimler geçer.
- **PDF:** `.run/gui-evidence/harita-kirpma-3200/pafta.pdf` (ve `pafta-1.png`). D L biçiminde, E alt yarısıyla basılır;
  sözler başvurununki.

## Sonra

- Atlas nesnesinin inceltilmemiş dış çizgisiyle kapsam (bugün çizilen inceltilmiş şekil kullanılır; aradaki fark çizim
  toleransı kadardır).
- Total station ve nivelman ölçmelerinin kalitesi (ADR 2400 Sonra).
- Atlas sayfalarının kaynak dökümü (ADR 1320 Kalan).
- Web (TODOS §0.2).
