# 4980 — Eğimle kot ver'de ve Yüzeyden kot ver'de aralığın parçası ve halkası (çok parçalı nesne)

Tarih: 2026-11-11. plan/map-cad.md MAP-06, onuncu tur. Durum: taslak. Plan satırının “Sıradaki”si: çok parçalı nesne;
dönen ve yaylı referansın bağı; yüzeye bağlı (dinamik) kot. Bu dilim ilkini kot araçlarında yapar: ADR 4580'in
“Sonra”sındaki “Çok parçalı nesnede aralığın parçası” ve ADR 1290'dan beri açık “çok parçalı nesne” (Eğimle kot ver).

İlgili:
- [ADR 1290](1290-grade-between-vertices.md), [3380](3380-grade-high-low-point.md): Eğimle kot ver
  (`cad.entities.grade`) ve yüksek/alçak nokta;
- [ADR 4180](4180-elevations-from-surface.md), [4580](4580-drape-relative-height-and-range.md): Yüzeyden kot ver
  (`cad.entities.drape`), göreli kot ve köşe aralığı;
- [ADR 2180](2180-grade-range-picked-on-the-drawing.md), [2980](2980-grade-range-marked-on-the-drawing.md): aralığın
  sahneden seçilmesi ve çizimde işareti;
- [ADR 0143](0143-multi-part-area.md) ve 0174: çok parçalı alan ve çoklu çizgi (`parts`, `holes`);
- [ADR 2580](2580-parallel-shift-reference-and-kept-distance.md): sınır kenarının adresi (`part`, `ring`, `index`).

## Dayanak

- **Mevzuat** (`.run/research/map-cad/mevzuat/MAP-06.10.md`, 10 Kasım 2026'da okundu): BÖHHBÜY Ek-2 UVDF §3.2.1 çok
  parçalı geometriyi GRUP kullanım tipiyle, §3.2.5 poligonun dış alanını ve boşluk alanlarını (delikler) ayrı koordinat
  listeleriyle tanımlar. Yüzeyden okunan kot için kod yoktur (Ek-1 KOT_DEGERI 01–05 ölçülmüş kotlardır; m. 46(3)).
  Aralığın parçası için mevzuatta hüküm yoktur; bu bir düzenleme kolaylığıdır.
- **QGIS API**, [QgsVertexId](https://api.qgis.org/api/structQgsVertexId.html) (11 Kasım 2026'da okundu): bir geometride
  köşe “part”, “ring” ve “vertex” numarasıyla adreslenir; iki köşenin aynı parçada (`partEqual`) ve aynı halkada
  (`ringEqual`) olup olmadığı ayrı sorulur. Köşe düzenleyicisinin resmi
  ([kullanım kılavuzu](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/editing_geometry_attributes.html),
  `.run/research/map-cad/uygulama/MAP-06.10-resim/vertex_editor_panel.png`): seçilen köşeler (27–29) tabloda ve
  haritada birlikte işaretlidir, aralık tek bir nesnenin ardışık köşeleridir.
- **Autodesk Civil 3D 2024**, To Set Feature Line Elevations From a Surface (ADR 4580'in kaynağı): “Partial” nesnenin
  kot verilecek bir kısmını tanımlar; özellik çizgisi tek parçalıdır, delik ve parça adresi yoktur.
- **Netcad:** wiki.netcad.com.tr'nin TLS sertifikası geçersiz (11 Kasım 2026, `ERR_CERT_DATE_INVALID`); denetim
  atlanmadı, Netcad kaynağı bu turda okunamadı.
- **KentOS bugün** (ADR 1290, 4580): Eğimle kot ver'in aralığı ve Yüzeyden kot ver'in `range`'i yalnız ilk parçanın
  dış halkasındadır; deliğe ya da ikinci parçaya kot aralığı verilemez, çok parçalı adanın ikinci parçası eğimle hiç,
  yüzeyden ancak bütün nesneyle birlikte kot alır.

## Karar

1. **`cad.entities.drape`'in `range`'i** (sürüm aynı, ek alanlar; QGIS'in adresi gibi):
   - `part`: 0 nesnenin kendi köşeleri (ilk parça), 1'den `parts` sırasıyla; yoksa 0;
   - `ring`: 0 parçanın dış halkası, 1'den delikleri sırasıyla; yoksa 0;
   - aralık (`from`, `to`) o halkanın köşeleridir (0'dan); kapanış kenarı aralıkta değildir. Delik kapalıdır; çoklu
     çizginin sonraki parçası açık, alanınki kapalıdır. Nesnenin öbür halkaları olduğu gibi kalır.
2. **`cad.entities.grade`** (sürüm aynı, ek alanlar `part`, `ring`; anlamları aynı): eğim, bitiş kotu ve yüksek/alçak
   nokta o halkanın köşelerine verilir; yüksek/alçak noktanın köşesi o halkaya eklenir (yay yarıları aynı çemberde),
   ölçüler (M) eski kuralla taşınır; öbür halkalar olduğu gibi kalır.
3. **Retler** (`wrong_place`; Yüzeyden kot ver'de ADR 4580'in sırasında, `range.part`/`range.ring`; Eğimle kot ver'de
   `not_a_path`'ten sonra, `part`/`ring`, halkanın köşelerinin dışındaki aralık `to`'da): olmayan parça (`range.part`: “Nesnenin 2 parçası var
   (0'dan): 2. parça yok.”), olmayan delik (`range.ring`: “0. parçanın 1 deliği var: 2. halka yok (0 dış halka, 1'den
   delikler).”), çizgide 0 olmayan parça ya da halka (“Çizginin tek parçası ve tek halkası var …”), halkanın köşelerinin
   dışındaki aralık (`range`: “… 1. parçanın 1. halkasının 4 köşesi var (0'dan) …”). Yeni kod yok.
4. **Masaüstü** (Eğimle kot ver; Hedef Eğim, Bitiş kotu, Yüksek/alçak nokta ve Yüzeyden › Aralık):
   - nesnenin birden çok halkası varsa **Halka** açılır listesi: “1. parça, dış halka”, “1. parça, 1. delik”,
     “2. parça, dış halka” …; seçilince aralık o halkanın ilk ve son köşesi olur;
   - **Sahneden seç**'in ilk tıklaması bütün halkalarda en yakın köşeyi alır ve halkayı seçer; ikinci tıklama aynı
     halkanın köşesini alır;
   - köşe numaraları, yardım satırı (“Köşeler 1'den 4'e (2. parça, 1. delik)”) ve çizimdeki işaret seçilen halkanındır.
5. **Kapsam işareti:** ADR 1290 ve 4180 gibi **CAD (Harita + Yol)**.

## Sınırlar

- Referanstan kot ver (`cad.entities.raise`) zaten bütün nesneyi kaydırır; köşesi (`at`) ilk parçanın dış halkasındadır.
- Dönen ve yaylı referansın bağı (ADR 2580'in “Sonra”sı) ve yüzeye bağlı (dinamik) kot bu dilimde yoktur.

## Kanıt

- **Bağımsız başvuru, Eğimle kot ver** (`scripts/fixtures/grade_cases.py`; KentOS kodu yok), 8 yeni durum (43): iki
  parçalı, delikli alanın (yeni nesne, 6) ikinci parçası eğimle, ikinci parçanın deliği bitiş kotuyla, ilk parçanın
  deliği plan; iki parçalı çoklu çizginin (yeni nesne, 7) ikinci parçasının yayında yüksek nokta (köşe eklenir, yarılar
  aynı çemberde); dört ret. Eski 35 durum aynı.
- **Bağımsız başvuru, Yüzeyden kot ver** (`scripts/fixtures/drape_cases.py`; KentOS kodu yok, kesirlerle), 9 yeni durum (43): iki
  parçalı çoklu çizginin ikinci parçası; iki parçalı, delikli alanın (yeni nesne, 18) ikinci parçasının 0–2 aralığı
  (köşegeni kesen kenar bölünür, kapanış kenarı, delikler ve ilk parça olduğu gibi); ikinci parçanın deliğinin 1–3
  aralığı kot farkıyla; ilk parçanın deliği; plan; dört ret. Eski 34 durum aynı.
- **Bozma denemesi:** her komutta iki beklenen değer (deliğin kırık noktası sayısı ve ikinci parçanın bir kotu; deliğin
  bitiş kotu ve yüksek noktanın yeri) değiştirildi, koşucu yalnız onları yakaladı, geri alındı.
- **Python** (`python/tests/test_grade_part_ring.py`): deliğin kotları, yüksek noktanın yayda yeri ve yarıları, ilk
  parça olduğu gibi, iki ret; (`python/tests/test_drape_relative_range.py`) testin kendi düzlemleri ve bağımsız `.kcad` okuyucusuyla:
  ikinci parçanın deliğinin köşeleri ve (81,25, 65)'teki kırık noktası farkla, ilk köşe kotsuz, öbür halkalar olduğu
  gibi; üç yeni ret.
- **MCP** (`crates/native/mcp/tests/drape_relative_range.rs`, `grade_peak.rs`): iki komutun delik aralığı, olmayan
  parça ya da delik `range.part`'ta, `ring`'de.
- **Masaüstü** (`grade_surface_relative_tests`): Sahneden seç'in ilk tıklaması ikinci parçanın deliğini seçer, önizleme
  3 köşe ve 1 kırık noktası, işaret deliğin üstünde; Uygula deliğin köşelerini yazar, öbür halkalar kotsuz; tek geri
  alma; (`grade_window_tests`) Halka ikinci parçanın deliğini seçer, aralık o halkanın ilk ve son köşesi olur, eğim
  %2 ve 30 m, Uygula yalnız deliğe yazar, tek geri alma. Resimler `.run/gui-evidence/cok-parcali-aralik-4980/`.

## Sonra

- Referanstan kot ver'in köşesinin parçası ve halkası.
- Yüzeye bağlı (dinamik) göreli kot: kalıcı bağ ve `.kcad` şeması.
- Dönen referansı izleyen bağ ve yaylı (eş merkezli) bağ.
- Web (TODOS §0.2).
