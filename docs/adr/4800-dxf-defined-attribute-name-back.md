# 4800 — DXF'te tanımlı özniteliğin boşluklu etiketinin geri adı

Tarih: 2026-11-10. plan/map-cad.md MAP-17, dokuzuncu tur. Durum: taslak. ADR 4380'in “Sonra”sındaki “tanımlı özniteliğin
boşluklu etiketinin geri adı (ATTDEF'in KentOS verisi; işlenmiş baytlar ve bağımsız denetçi)”. Dilim maddenin sıradaki
işinin bütünüdür.

İlgili:
- [ADR 0144](0144-blocks.md) §7: blok öznitelikleri; tanımın etiketinde boşluk “_”;
- [ADR 3980](3980-dxf-attributes-without-definition.md): tanımı olmayan öznitelik görünmez ATTRIB;
- [ADR 4380](4380-dxf-attribute-tag-for-a-name-with-a-space.md): tanımı olmayanın boşluklu adı eklemenin KentOS
  verisinde (`tagname`), dosya kazanır.

## Dayanak

- **AutoCAD DXF başvurusu**, ATTDEF ve ATTRIB
  ([2012 başvurusu](https://images.autodesk.com/adsk/files/autocad_2012_pdf_dxf-reference_enu.pdf), s. 68–72;
  [2018 ATTDEF](https://help.autodesk.com/cloudhelp/2018/ENU/AutoCAD-DXF/files/GUID-F0EA099B-6F88-4BCC-BEC7-247BA64838A4.htm);
  10 Kasım 2026'da okundu):
  - ATTDEF grup 2 “Tag string (cannot contain spaces)”, grup 3 istem (prompt) dizgisi; istemde boşluk sınırı yok;
  - ATTRIB grup 2 de boşluk içeremez; ATTRIB'de istem yoktur.
- **AutoCAD'in Attribute Definition penceresi ve BATTMAN**
  ([LT 2026](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-92A7CE9D-2E1C-415E-A1BF-5CB35D5E2D38.htm),
  [BATTMAN](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-LT/files/GUID-4920952B-6809-49E7-956E-C6A94BEA6FA1.htm)):
  - etikette küçük harf büyüğe çevrilir;
  - istem boşsa etiket istem olur;
  - BATTMAN tanımın etiketini değiştirir, değişiklik eklemelere Sync ile geçer.
- **Ortak örüntü** (GDAL DXF sürücüsü, Esri alan adı ve takma ad, Map 3D nesne verisi): eşleşme boşluksuz makine adıyla,
  görünen ad ayrı tutulur. GDAL “BlockAttributes”i “etiket + boşluk + değer” diye yazar; boşluklu etiket orada
  bölünür.
- **Türk veri standartları** (TUCBS GKM Tavsiye 12–13, VTS §5.2.3; BÖHHBÜY Ek-2 UVDF §4.4.1): okunur ad ile teknik ad
  ayrıdır. Okunur adda boşluk olur, teknik adda olmaz. Ad dönüşümünün biçimi (lowerCamelCase, BÜYÜK_HARF) kullanıcının
  profilidir; KentOS'un “_”'li etiketi “TUCBS'ye uygun” sayılmaz.

  Araştırma notları: `.run/research/map-cad/mevzuat/MAP-17.9.md`, `uygulama/MAP-17.9.md`.
- **KentOS bugün:**
  - Tanımın “Kol boyu” etiketi “Kol_boyu” yazılıyor ve söyleniyordu.
  - Geri okumada tanımın etiketi de eklemelerin değeri de “Kol_boyu” geliyordu; aktarma kaybı denetimi DXF'te
    özniteliği kayıp ve fazla sayıyordu.

## Karar

1. **Ad ATTDEF'in KentOS verisinde:** etiketi tanımın adından başka yazılan (boşluk ve denetim karakteri “_”, çakışan
   “_2”) her ATTDEF'e KentOS'un XDATA'sında `tagname` kaydı: yazılan etiket ve ad (ADR 4380'in kaydı).
   - Öbür ATTDEF'ler kayıt taşımaz.
   - Eklemeler kayıt taşımaz: değerleri tanımın adını alır.
   - Rapor: “… “Kol boyu” etiketi “Kol_boyu” olarak yazıldı (…); adı ATTDEF'in KentOS verisinde, KentOS geri okurken
     düzeltir”.
2. **Geri okuma, dosya kazanır** (ADR 4380'in kuralı tanıma da):
   - Tanımın etiketi kendi kaydının etiketine büyük küçük harf ayırmadan uyuyorsa tanımın özniteliği kaydın adını
     alır. AutoCAD'in büyük harfe çevirdiği “KOL_BOYU” da uyar.
   - Eklemelerinin aynı etiketli ATTRIB değerleri de o adla gelir; değer dosyanınkidir.
   - Kaydın adı bloğun başka bir ATTDEF'inin etiketiyse (KentOS verisi elle değişmiş) ya da önceki bir etiket o adı
     almışsa etiket kalır.
   - Etiket başka programda değişmişse (BATTMAN: “KOL_UZUNLUK”) yeni etiket ad olur, kayıt alınmaz. Okuma raporu söyler:
     “… “KOL_UZUNLUK” etiketi KentOS'un yazdığı “Kol_boyu” değil (başka bir programda değişmiş); KentOS verisindeki “Kol
     boyu” adı alınmadı, etiket ad oldu”.
   - KentOS verisi silinmişse etiket ad olur (bilinemez, söylenmez).
3. **İstem ad kaynağı değildir:** okunur ad istem (grup 3) olarak yazılmaz. İstem kullanıcının sorusudur, AutoCAD'de
   değişir; geri okumada gidiş-dönüşü bozar (boş istem dolu gelir).
4. **Kapsam işareti:** ADR 4380 gibi **CAD + CBS** (DXF dışa aktarımı her modülde). Arayüz değişmedi.

## Sınırlar

- Okunur ad yalnız KentOS'un verisindedir: başka program “Kol_boyu”yu görür. Boşluksuz etiket GDAL'ın “etiket değer”
  biçiminde bölünmemelidir; GDAL ile sınanmadı.
- Kaydı silen bir program (ör. XDATA'yı tutmayan dönüştürücü) adı götürür; KentOS bunu dosyadan bilemez.
- Blok tanımı içinde iç içe eklenen bloğun öznitelikleri de aynı yolla okunur; ayrı sınanmadı.

## Kanıt

- **Yazıcı ve okuyucu testleri** (`crates/shared/formats/tests/dxf_write.rs`):
  - yeni `a_definition_s_tag_comes_back_by_its_attdef_s_kentos_data_and_the_file_wins`:
    - kayıt yalnız Kol_boyu'nun ATTDEF'inde;
    - geri okumada tanım “Kol boyu”, değer “Kol boyu” altında;
    - etiket büyük harfle yine ad;
    - BATTMAN'le değişen etiket yeni ad ve söylenir;
    - silinen kayıt etiket;
    - adı başka ATTDEF'in etiketi olan kayıt alınmaz.
  - Eski dört test artık “Kol boyu”yu bekler.
  - İşlenmiş `blocks.dxf` baytları: yalnız Kol_boyu ATTDEF'ine 12 satırlık kayıt eklendi (fark okundu).
  - Biçim kitaplığının bütün testleri geçer.
- **Bağımsız denetçi** (`scripts/fixtures/dxf_write_reference.py`, KentOS kodu yok):
  - etiketi addan başka ATTDEF'in kaydı etiket ve ad, öbürlerinde kayıt yok;
  - ADR 3980'den beri eskimiş ATTRIB denetimi de düzeltildi: tanımı olmayan tek satırlık öznitelikler görünmez ATTRIB
    (etiket, 70 = 1, değer), KentOS verisinde kalmayanlar, `tagname` kayıtları.
  - 11 işlenmiş dosyanın hepsi geçer (blocks.dxf ADR 3980'den beri kalıyordu).
  - Bozma denemesi: kayıttaki ad “Kol boy” yazılınca yakalanır.
- **Bağımsız okuyucu** (`python/tests/test_dxf_defined_attribute_names.py`, ezdxf):
  - ATTDEF ve ATTRIB etiketi “Kol_boyu”, ATTDEF'in KENTOS verisi `tagname`, No'nunki yok;
  - KentOS'un okuması “Kol boyu”;
  - ezdxf etiketleri büyük harfe çevirince yine “Kol boyu”;
  - etiketi “KOL_UZUNLUK” yapınca o;
  - KENTOS verisini silince “Kol_boyu”.

  ADR 3980/4380'in ezdxf testi geçer.
- **MCP** (`crates/native/mcp/tests/dxf_defined_attribute_names.rs`): komutlarla kurulan tanım ve ekleme;
  dışa aktarmanın raporu; dosyada ATTDEF ve kayıt; geri açılan çizimde “Kol boyu”. ADR 4380'in MCP testi geçer.
- **Masaüstü** (`dxf_attribute_names_tests`): devir-blok çiziminde Rögar'a “Kapak tipi” tanımı ve değeri komutlarla
  yazılır. Dışa aktarma kaybı denetimi'nin DXF gidiş-dönüşünde öznitelik ve blok kaybı yok. Aktarma denetimi
  testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/oznitelik-adi-4800/`.

## Sonra

- DXF'te TABLE (ACAD_TABLE) yazımı: doğrulama yolu (AutoCAD ya da bağımsız okuyucu) bekler.
- Kurum profilleri: ad dönüşümünün biçimi (TUCBS lowerCamelCase, BÖHHBÜY BÜYÜK_HARF) profilin seçimi olarak.
- Web: dxf-wasm yeniden derlenince `dxf.wasm.test.ts`'in yeni baytları (TODOS §0.2).
