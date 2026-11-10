# 4980 — Yüzeyden kot ver'de aralığın parçası ve halkası (çok parçalı nesne)

Tarih: 2026-11-11. plan/map-cad.md MAP-06, onuncu tur. Durum: taslak. Plan satırının “Sıradaki”si: çok parçalı nesne;
dönen ve yaylı referansın bağı; yüzeye bağlı (dinamik) kot. Bu dilim ilkini yapar: ADR 4580'in “Sonra”sındaki
“Çok parçalı nesnede aralığın parçası”.

İlgili:
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
- **KentOS bugün** (ADR 4580): `range` yalnız ilk parçanın dış halkasına verilir; deliğe ya da ikinci parçaya kot
  aralığı verilemez, çok parçalı adanın ikinci parçası ancak bütün nesneyle birlikte kot alır.

## Karar

1. **`cad.entities.drape`'in `range`'i** (sürüm aynı, ek alanlar; QGIS'in adresi gibi):
   - `part`: 0 nesnenin kendi köşeleri (ilk parça), 1'den `parts` sırasıyla; yoksa 0;
   - `ring`: 0 parçanın dış halkası, 1'den delikleri sırasıyla; yoksa 0;
   - aralık (`from`, `to`) o halkanın köşeleridir (0'dan); kapanış kenarı aralıkta değildir. Delik kapalıdır; çoklu
     çizginin sonraki parçası açık, alanınki kapalıdır. Nesnenin öbür halkaları olduğu gibi kalır.
2. **Retler** (`wrong_place`, yerleri ADR 4580'in sırasında): olmayan parça (`range.part`: “Nesnenin 2 parçası var
   (0'dan): 2. parça yok.”), olmayan delik (`range.ring`: “0. parçanın 1 deliği var: 2. halka yok (0 dış halka, 1'den
   delikler).”), çizgide 0 olmayan parça ya da halka (“Çizginin tek parçası ve tek halkası var …”), halkanın köşelerinin
   dışındaki aralık (`range`: “… 1. parçanın 1. halkasının 4 köşesi var (0'dan) …”). Yeni kod yok.
3. **Masaüstü** (Eğimle kot ver › Hedef Yüzeyden › Aralık):
   - nesnenin birden çok halkası varsa **Halka** açılır listesi: “1. parça, dış halka”, “1. parça, 1. delik”,
     “2. parça, dış halka” …; seçilince aralık o halkanın ilk ve son köşesi olur;
   - **Sahneden seç**'in ilk tıklaması bütün halkalarda en yakın köşeyi alır ve halkayı seçer; ikinci tıklama aynı
     halkanın köşesini alır;
   - köşe numaraları, yardım satırı (“Köşeler 1'den 4'e (2. parça, 1. delik)”) ve çizimdeki işaret seçilen halkanındır.
4. **Kapsam işareti:** ADR 4180 gibi **CAD (Harita + Yol)**.

## Sınırlar

- Eğimle kot ver'in (`cad.entities.grade`) aralığı hâlâ ilk parçanın dış halkasındadır.
- Dönen ve yaylı referansın bağı (ADR 2580'in “Sonra”sı) ve yüzeye bağlı (dinamik) kot bu dilimde yoktur.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/drape_cases.py`; KentOS kodu yok, kesirlerle), 9 yeni durum (43): iki
  parçalı çoklu çizginin ikinci parçası; iki parçalı, delikli alanın (yeni nesne, 18) ikinci parçasının 0–2 aralığı
  (köşegeni kesen kenar bölünür, kapanış kenarı, delikler ve ilk parça olduğu gibi); ikinci parçanın deliğinin 1–3
  aralığı kot farkıyla; ilk parçanın deliği; plan; dört ret. Eski 34 durum aynı.
- **Python** (`python/tests/test_drape_relative_range.py`), testin kendi düzlemleri ve bağımsız `.kcad` okuyucusuyla:
  ikinci parçanın deliğinin köşeleri ve (81,25, 65)'teki kırık noktası farkla, ilk köşe kotsuz, öbür halkalar olduğu
  gibi; üç yeni ret.
- **MCP** (`crates/native/mcp/tests/drape_relative_range.rs`): aynı delik aralığı, olmayan parça `range.part`'ta.
- **Masaüstü** (`grade_surface_relative_tests`): Sahneden seç'in ilk tıklaması ikinci parçanın deliğini seçer, önizleme
  3 köşe ve 1 kırık noktası, işaret deliğin üstünde; Uygula deliğin köşelerini yazar, öbür halkalar kotsuz; tek geri
  alma. Resimler `.run/gui-evidence/cok-parcali-aralik-4980/`.

## Sonra

- Eğimle kot ver'in (`cad.entities.grade`, yüksek/alçak nokta dahil) aralığının parçası ve halkası.
- Yüzeye bağlı (dinamik) göreli kot: kalıcı bağ ve `.kcad` şeması.
- Dönen referansı izleyen bağ ve yaylı (eş merkezli) bağ.
- Web (TODOS §0.2).
