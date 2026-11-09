# 4380 — DXF'te boşluklu öznitelik adının etiketi ve geri okunması

Tarih: 2026-11-10. plan/map-cad.md MAP-17, sekizinci tur. Durum: taslak. ADR 3980'in “Sonra”sındaki “boşluklu adın
etiketi (“_” ile) ve geri okunduğunda adının KentOS verisinden düzeltilmesi: iki kaynağın çakışma kuralı gerekir”.

İlgili:
- [ADR 3980](3980-dxf-attributes-without-definition.md): tanımı olmayan öznitelik görünmez ATTRIB olarak;
- [ADR 0144](0144-blocks.md) §7: blok öznitelikleri; tanımın etiketinde boşluk “_”, ATTRIB KentOS verisinden üstün.

## Dayanak

- **AutoCAD DXF başvurusu, ATTRIB** (ADR 3980'in okuması): etiket (grup 2) boşluk içeremez; bayrak 70'in 1. biti
  görünmez öznitelik. DXF etiketleri büyük küçük harf ayırmaz.
- **KentOS bugün:**
  - Tanımlı özniteliğin etiketinde boşluk ve denetim karakteri “_” olur, önce alınmış etiket “_2”, “_3” … alır;
    değişiklik söylenir (ADR 0144 §7).
  - Tanımı olmayan öznitelik yalnız adı DXF etiketi olarak aynen yazılabiliyorsa görünmez ATTRIB olur. “Bakım notu”
    gibi boşluklu ad yalnız KentOS verisinde kalıyordu; başka program onu görmüyordu.
  - Geri okumada ATTRIB'in değeri aynı etiketli KentOS verisinden üstündür (ADR 0144 §7).

## Karar

1. **Etiket:** tanımı olmayan, değeri tek satır her öznitelik görünmez ATTRIB olarak yazılır.
   - Adı DXF etiketi olarak aynen yazılabiliyorsa etiketi adıdır, eskisi gibi.
   - Değilse tanımın kuralıyla: boşluk ve denetim karakterleri “_”; eklemenin başka bir etiketiyle büyük küçük harf
     dışında aynıysa “_2”, “_3” …
   - Değeri birden çok satır olan yalnız KentOS verisinde kalır, söylenir (ADR 3980 gibi).
2. **Ad KentOS verisinde:** etiketi adından başka olan her ATTRIB için eklemenin KentOS verisine (XDATA) bir
   `tagname` kaydı yazılır: etiket ve ad. Rapor söyler: “tanımı olmayan “Bakım notu” özniteliği “Bakım_notu” etiketiyle
   yazıldı (…); adı KentOS verisinde, KentOS geri okurken düzeltir”.
3. **Geri okuma, iki kaynağın kuralı:**
   - Dosya kazanır: ATTRIB'in etiketi KentOS verisindeki bir kayda uyuyorsa (büyük küçük harf ayırmadan) öznitelik
     kaydın adını alır, değeri ATTRIB'inkidir; başka programda değiştirilen değer geçer.
   - Başka programda silinen ATTRIB'in kaydı bir şey getirmez: öznitelik yoktur.
   - Kaydın adı dosyada başka bir ATTRIB'in etiketi olarak zaten varsa (KentOS verisi elle değişmiş), o ATTRIB adını
     tutar, bu etiket kendi adıyla kalır; iki değer de korunur.
4. Tanımlı özniteliklerin yazımı değişmez: tanımın etiketi “_” ile yazılır ve öyle geri gelir (Sınırlar).
5. **Kapsam işareti:** ADR 3980'deki gibi **CAD + CBS** (DXF dışa aktarımı her modülde).

## Sınırlar

- Tanımlı özniteliğin boşluklu etiketi (ör. “Kol boyu”) “Kol_boyu” olarak geri gelir. Düzeltmesi tanımın ATTDEF'ine
  de ad kaydı ister; işlenmiş `blocks.dxf` baytları ve bağımsız denetçi (`dxf_write_reference.py`) değişir (Sonra).
- Birden çok satırlı değer DXF'in tek satırlık ATTRIB'ine sığmaz; yalnız KentOS verisinde kalır.

## Kanıt

- **Yazıcı testleri** (`crates/shared/formats/tests/dxf_write.rs`):
  - `attributes_without_a_definition_go_out_as_invisible_attribs`: görünmez ATTRIB'ler Bakım_notu, Malzeme, NOT ve
    Not_2 (“Not” “NOT” ile çakışır); iki satırlı değer söylenir, iki ad değişikliği söylenir; geri okunan ekin bütün
    öznitelikleri aynı;
  - yeni `a_tag_written_for_a_name_comes_back_by_kentos_data_and_the_file_wins`:
    - etiket ve KentOS verisindeki kayıt;
    - geri okumada ad;
    - başka programda değişen değer;
    - büyük harfe çevrilen etiket;
    - silinen ATTRIB (öznitelik yok);
    - kaydın adı başka ATTRIB'deyse dosyanınki ve etiket kalır;
  - `an_attrib_edited_elsewhere_wins` iki satırlı değerle yeniden kuruldu;
  - işlenmiş baytlar (blocks.dxf ve öbürleri) değişmedi. Biçim kitaplığının bütün testleri geçer.
- **Bağımsız okuyucu** (`python/tests/test_dxf_extra_attributes.py`, ezdxf): Bakım_notu, Malzeme ve No görünmez ATTRIB
  olarak okunur; KentOS dosyayı geri açınca “Bakım notu” adıyla, bütün öznitelikler aynı.
- **MCP** (`crates/native/mcp/tests/dxf_extra_attributes.rs`): raporda ad değişikliği; dosyada Bakım_notu'nun `70 = 1`
  ATTRIB'i, boşluklu etiket yok; geri açılan çizimde öznitelikler aynı.
- **Paylaşılan durumlar ve masaüstü:** dışa aktarma ve paket durumları, masaüstünün DXF ve dışa aktarma testleri ve
  çapraz denetimler geçer. Arayüz değişmedi; resim çekilmedi.

## Sonra

- Tanımlı özniteliğin boşluklu etiketinin geri adı (ATTDEF'in KentOS verisi; işlenmiş baytlar ve bağımsız denetçi).
- DXF'te TABLE (ACAD_TABLE) yazımı (ADR 3580 Sonra): doğrulama yolu bekler.
- Web (TODOS §0.2).
