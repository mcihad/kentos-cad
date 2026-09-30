# DXF yazıcısı fixture'ları (`formats/v1/dxf-write`)

DXF yazıcısının (`crates/shared/formats/src/dxf/writer`) çıktısını KentOS kodu dışında da denetlemek için. `<ad>.input.json` yazıcının girdisidir (`DxfWriteInput`, elle yazılmıştır), `<ad>.dxf` yazıcının ondan yazdığıdır, baytlarıyla depodadır.

- Rust yazıcısı girdiyi aynı baytlara yazar (`crates/shared/formats/tests/dxf_write.rs`); web'in WASM modülü de (`apps/web/src/io/dxf.wasm.test.ts`). Yazıcı değişince `KENTOS_WRITE_DXF=1 cargo test -p kentos-formats --test dxf_write` dosyayı yeniden yazar; farkı okuyun.
- [`scripts/fixtures/dxf_write_reference.py`](../../../../scripts/fixtures/dxf_write_reference.py) dosyayı girdiye karşı, yalnız Python'un standart kitaplığıyla ve kurallardan denetler: `python3 scripts/fixtures/dxf_write_reference.py --check`.

| Dosya | Sınadığı |
|---|---|
| `blocks.input.json` | Bloklar ([ADR 0144](../../../../docs/adr/0144-blocks.md) §5). “Direk” (taban noktası 1, 2; iki satırlı açıklama): kendi katmanı olmayan bir çizgi, delikli bir alan ve dörtte bir dönüşle “Lamba”nın yerleştirmesi. “Lamba”: kendi rengi olan bir daire ve Yapı katmanında kendi kalınlığı olan bir çizgi. Hiçbir şeyin yerleştirmediği “Kullanılmayan”. DXF'in kabul etmediği karakterli “Ağaç/Çınar”. Çizimde: Direk'in aynalı, 2,5 ölçekli, dereceye tam geçmeyen 0,1 radyan dönüşlü, renkli, öznitelikli yerleştirmesi; Direk'in düz yerleştirmesi; Ağaç/Çınar'ın 270°'lik yerleştirmesi; girdide olmayan bir bloğun yerleştirmesi; bir çizgi. |

`blocks.dxf`'te beklenenler:
- BLOCK ve blok kayıtları Lamba, Direk ve Ağaç_Çınar sırasıyla yazılır; blok, içerdiği bloklardan sonra gelir. Kullanılmayan yazılmaz.
- Bloğun nesneleri bloğun kaydına aittir. Kendi katmanı olmayan nesne 0 katmanındadır, kendi katmanı olan kendi katmanının DXF adındadır. Kendi rengi ya da kalınlığı olmayan nesne BYBLOCK yazılır (62 0, 370 −2).
- Delik, alanına KentOS verisiyle bağlıdır. Açıklama tek satır olur.
- Aynalı yerleştirmenin Y ölçeği eksidir. Dönüşü derecedir; derece tam radyanı geri vermediğinde KentOS verisi (`turn`) radyanı tam taşır.
- Girdide olmayan bloğun yerleştirmesi yazılmaz ve söylenir.
