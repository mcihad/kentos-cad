# 3980 — DXF'te tanımı olmayan blok ekinin öznitelikleri: görünmez ATTRIB

Tarih: 2026-11-09. plan/map-cad.md MAP-17, yedinci tur. Durum: taslak. ADR 3580'in ve 3180'in “Sonra”sındaki “Tanımı
olmayan ek özniteliklerin ATTRIB'i”, ADR 2380'in “DXF'te ek özniteliklerinin ATTRIB olarak yazılması”.

İlgili:
- [ADR 0144](0144-blocks.md) §7: bloğun öznitelik tanımları ATTDEF, ekin değerleri ATTRIB;
- [ADR 2380](2380-exchange-blocks-tables-relations.md): dışa aktarma denetiminde bloklar.

## Dayanak

- **AutoCAD DXF Reference, ATTRIB**
  ([belge](https://help.autodesk.com/cloudhelp/2016/ENU/AutoCAD-DXF/files/GUID-7DD8B495-C3F8-48CD-A766-14F9D7D0DD9B.htm),
  9 Kasım 2026'da okundu):
  - grup 2 özniteliğin etiketidir ve “boşluk içeremez”;
  - grup 70 bit kodlu bayraktır: 1 “öznitelik görünmez”, 2 sabit, 4 doğrulama, 8 önayarlı;
  - 10 yazının başlangıç noktası, 40 yüksekliği, 1 değeri; 7 yazı stili isteğe bağlıdır (STANDARD).
- **KentOS bugün** (ADR 0144 §7, `writer/entities.rs`): bloğunda öznitelik tanımı olan ekin değerleri ATTRIB olarak
  yazılıyordu. Tanımı olmayan öznitelikler (ekin kendi öznitelikleri) yalnız KentOS'un XDATA'sında kalıyordu; başka
  program onları görmüyordu.
- **KentOS'un okuyucusu:** ATTRIB değeri KentOS verisine üstündür. Tanımda karşılığı olmayan görünür ATTRIB ayrıca yazı
  olarak alınır, görünmez olan yalnız öznitelik olur.

## Karar

1. **Görünmez ATTRIB:** blok ekinin, bloğunun hiçbir tanımının tutmadığı her özniteliği INSERT'in ATTRIB'i olarak yazılır
   (`66 = 1`, tanımlı olanlardan sonra, SEQEND'den önce):
   - etiket özniteliğin adıdır, değer kendi değeridir;
   - bayrak `70 = 1` (görünmez);
   - yer ekin noktası, yükseklik gerçek boyda 2,5 mm (ISO 3098'in yazı yüksekliği). Yalnız görünmez öznitelikleri
     gösteren bir programda görülür.
2. **Koşul:** yalnız gidiş-dönüşte aynen dönecekler yazılır:
   - ad DXF etiketi olarak aynen yazılabilmeli: boşluk ya da denetim karakteri yok, ekin başka bir etiketiyle büyük
     küçük harf dışında aynı değil;
   - değer tek satır olmalı.

   Öbürleri eskisi gibi yalnız KentOS verisinde kalır ve dışa aktarmanın raporunda söylenir (“Öznitelik: … yalnız
   KentOS verisinde kaldı; başka program onu görmez”).
3. **KentOS verisi:** ATTRIB olarak yazılan öznitelik KentOS verisinden çıkar (tanımlı olanlar gibi). Geri okunduğunda
   ATTRIB'den gelir. Başka bir programda değiştirilirse değişmiş hâli geçer (ADR 0144 §7'nin kuralı).
4. Tanımlı özniteliklerin yazımı ve etiket kuralı (boşluk “_”) değişmez.
5. **Kapsam işareti:** ADR 1660'taki gibi **CAD + CBS** (DXF dışa aktarımı her modülde).

## Kanıt

- **Yazıcı testleri** (`crates/shared/formats/tests/dxf_write.rs`):
  - `attributes_without_a_definition_go_out_as_invisible_attribs`: Direk'in ekine adı boşluklu, büyük küçük harfle
    çakışan ve iki satırlı öznitelikler eklendi. Görünmez ATTRIB'ler yalnız Malzeme ve NOT'tur; not söylenir; geri
    okunan ekin bütün öznitelikleri aynıdır, ATTRIB yazı olarak dönmez;
  - `an_attrib_edited_elsewhere_wins` yeni duruma göre: KentOS verisinde yalnız boşluklu ad kalır, ATTRIB yine
    üstündür;
  - `blocks.dxf` yeniden yazıldı. Fark: Malzeme XDATA'dan çıkıp görünmez ATTRIB oldu, tutamaçlar birer kaydı.
- **Bağımsız okuyucu** (`python/tests/test_dxf_extra_attributes.py`, ezdxf 1.4.5): tanımsız blokta Malzeme ve No
  görünmez ATTRIB olarak okunur (etiket, değer, `is_invisible`); boşluklu ve iki satırlı öznitelik ATTRIB değildir.
  KentOS dosyayı geri açınca ekin dört özniteliği aynıdır.
- **MCP** (`crates/native/mcp/tests/dxf_extra_attributes.rs`): `file.export.dxf` raporunda not; dosyada Malzeme'nin
  `70 = 1` ATTRIB'i, boşluklu etiket yok; geri açılan çizimde öznitelikler aynı.
- **Paylaşılan durumlar:** dışa aktarma ve devir denetimi durumları değişmeden geçer. Masaüstünün dışa aktarma testleri
  ve çapraz denetimler geçer. Arayüz değişmedi; resim çekilmedi.

## Sonra

- Boşluklu adın etiketi (“_” ile) ve geri okunduğunda adının KentOS verisinden düzeltilmesi: iki kaynağın çakışma kuralı
  gerekir.
- DXF'te TABLE (ACAD_TABLE) yazımı (ADR 3580 Sonra): doğrulama yolu bekler.
- Web (TODOS §0.2).
