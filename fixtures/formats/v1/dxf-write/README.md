# DXF yazıcısı fixture'ları (`formats/v1/dxf-write`)

DXF yazıcısının (`crates/shared/formats/src/dxf/writer`) çıktısını KentOS kodu dışında da denetlemek için. `<ad>.input.json` yazıcının girdisidir (`DxfWriteInput`, elle yazılmıştır), `<ad>.dxf` yazıcının ondan yazdığıdır, baytlarıyla depodadır.

- Rust yazıcısı girdiyi aynı baytlara yazar (`crates/shared/formats/tests/dxf_write.rs`); web'in WASM modülü de (`apps/web/src/io/dxf.wasm.test.ts`). Yazıcı değişince `KENTOS_WRITE_DXF=1 cargo test -p kentos-formats --test dxf_write` dosyayı yeniden yazar; farkı okuyun.
- [`scripts/fixtures/dxf_write_reference.py`](../../../../scripts/fixtures/dxf_write_reference.py) dosyayı girdiye karşı, yalnız Python'un standart kitaplığıyla ve kurallardan denetler: `python3 scripts/fixtures/dxf_write_reference.py --check`.

| Dosya | Sınadığı |
|---|---|
| `blocks.input.json` | Bloklar ([ADR 0144](../../../../docs/adr/0144-blocks.md) §5, §7). “Direk” (taban noktası 1, 2; iki satırlı açıklama): kendi katmanı olmayan bir çizgi, delikli bir alan, dörtte bir dönüşle “Lamba”nın yerleştirmesi ve üç öznitelik tanımı: sorulu ve varsayılanlı “No”, varsayılanı olmayan “Tür”, sorusu olmayan, 90° dönük, adında boşluk olan “Kol boyu”. “Lamba”: kendi rengi olan bir daire ve Yapı katmanında kendi kalınlığı olan bir çizgi. Hiçbir şeyin yerleştirmediği “Kullanılmayan”. DXF'in kabul etmediği karakterli “Ağaç/Çınar”. Çizimde: Direk'in aynalı, 2,5 ölçekli, dereceye tam geçmeyen 0,1 radyan dönüşlü, renkli, “No” ve “Tür” değerli, bir de “Malzeme” öznitelikli yerleştirmesi; Direk'in değersiz düz yerleştirmesi; Ağaç/Çınar'ın 270°'lik yerleştirmesi; girdide olmayan bir bloğun yerleştirmesi; bir çizgi. |
| `texts.input.json` | Yazılar ([ADR 0145](../../../../docs/adr/0145-text-extras.md) §7). “Ada 101” on iki hizada (biri hizasız), 30° dönük, 0,8 genişlik çarpanıyla; ortasının ortasına hizalı zeminli, hizasız zeminli ve düz birer yazı; öznitelik tanımı altının ortasına hizalı ve 1,2 çarpanlı “Etiket” bloğu ve onun 2 ölçekli, 0,5 radyan dönük yerleştirmesi. |

`blocks.dxf`'te beklenenler:
- BLOCK ve blok kayıtları Lamba, Direk ve Ağaç_Çınar sırasıyla yazılır; blok, içerdiği bloklardan sonra gelir. Kullanılmayan yazılmaz.
- Bloğun nesneleri bloğun kaydına aittir. Kendi katmanı olmayan nesne 0 katmanındadır, kendi katmanı olan kendi katmanının DXF adındadır. Kendi rengi ya da kalınlığı olmayan nesne BYBLOCK yazılır (62 0, 370 −2).
- Delik, alanına KentOS verisiyle bağlıdır. Açıklama tek satır olur.
- Aynalı yerleştirmenin Y ölçeği eksidir. Dönüşü derecedir; derece tam radyanı geri vermediğinde KentOS verisi (`turn`) radyanı tam taşır.
- Girdide olmayan bloğun yerleştirmesi yazılmaz ve söylenir.
- Özniteliği olan bloğun BLOCK'unun bayrağı 2'dir (70 2). Öznitelik tanımları bloğun nesnelerinden sonra, bloğun kaydına ait, 0 katmanında ATTDEF'tir: yeri, yüksekliği, varsayılanı (1), sorusu (3), etiketi (2), görünür (70 0), dönüşü derece (50; sıfırsa yazılmaz).
- DXF'in etiketi boşluk içermez ve büyük küçük harf ayırmaz: “Kol boyu” “Kol_boyu” yazılır ve söylenir; çakışan etiket “_2” alır.
- Özniteliği olan bloğun yerleştirmesinde 66 1 vardır; ardından yerleştirmeye ait, onun katmanında ve renginde, tanımın her özniteliği için bir ATTRIB gelir: yeri, yüksekliği ve dönüşü yerleştirmenin benzerliğiyle yerleşmiş, değeri yerleştirmenin gösterdiği (değeri yoksa varsayılan, o da yoksa boş). Son ATTRIB'den sonra yerleştirmeye ait bir SEQEND. Özniteliklerin değerleri yalnız ATTRIB'lerdedir, başka bir programın değiştirdiği onlardır; KentOS verisi yerleştirmenin öbür özniteliklerini (“Malzeme”) taşır.
- Okuyucu dosyayı geri okuyunca tanımlar etiketleri, soruları ve varsayılanlarıyla, yerleştirmeler gösterdikleri değerlerle gelir; hiçbir ATTRIB ayrı yazı olarak gelmez (`attributes_go_out_as_attdef_and_attrib_and_come_back`). Başka bir programda değişen ATTRIB değişmiş gelir; aynı etiket KentOS verisinde de varsa ATTRIB'inki geçer (`an_attrib_edited_elsewhere_wins`). Satır sonlu değer, varsayılan ya da soru tek satır yazılır ve söylenir.

`texts.dxf`'te beklenenler:
- Hizasız yazı noktasından başlar (10); 11, 72 ve 73 yoktur.
- Hizalı yazı hiza noktasında (11) tam durur; 72 ve 73 (öznitelikte 74) hizayı söyler, 0 olan yazılmaz. 72 ve 11 ikinci AcDbText'ten (öznitelikte AcDbAttribute ya da AcDbAttributeDefinition'dan) önce, dikey hiza sonradır.
- 10, hizayı bilmeyen programlar için başlangıç noktasıdır: 11'in, hizanın yükseklik payı kadar taban çizgisine dik altında ve genişlik payı kadar gerisinde; genişlik yazının Arimo'daki ölçüsüdür (aynı sözlerin, yüksekliğin ve çarpanın hepsinde aynı).
- Genişlik çarpanı 41'dir (1'se yazılmaz).
- TEXT'in zemini yoktur: zeminli yazının KentOS verisinde `mask` ögesi vardır, öbürlerinde yoktur; rapor bunu söyler.
- ATTRIB yerleştirmenin benzerliğiyle yerleşir: 11'i tanımın noktasının yerleşmiş hâlidir, yüksekliği ölçekle çarpılır, çarpanı ve hizası tanımınkidir.
- Okuyucu dosyayı geri okuyunca her yazı yerinde, hizası, çarpanı ve zeminiyle gelir (`texts_read_back_with_their_alignment_width_factor_and_mask`).
