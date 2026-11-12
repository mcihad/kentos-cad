# KentOS DWG adaptörü

Codec kaynakları bu depoya kopyalanmaz. `acadrust`, OpenCADStudio'nun kilidindeki revizyonla
geliştiricisinin Git deposundan (`hakanaktt/acadrust`) workspace `rev` ve `Cargo.lock` ile alınır
(12 Kasım 2026'ya kadar silinen OpenCADCodec çatalından; ADR 8053).
Son kullanıcıya ayrı OpenCADStudio kurulumu gerekmez.

Byte dizileri ↔ KentOS sözleşmeleri sınırı, birim/kayıp raporu ve yeniden okuma
denetimi burada; dosya yolları, atomik yazma ve belge/undo ev sahibindedir.
Kapsam, sürüm yükseltme ve açık üretim kabulü sınırları:
[ADR 0586](../../../docs/adr/0586-dwg-opencadcodec-git.md).

`cargo test -p kentos-dwg` adaptör örneklerini sınar.
`cargo run -p kentos-dwg --example corpus` codec ile binary örnekleri yeniden üretir;
`python3 scripts/fixtures/dwg_import_command_cases.py` bağımsız ASCII başvurusundan
komut beklentilerini ve dosya hash/boyut makbuzlarını üretir. Binary örnekler bağımsız
DWG motorunun çıktısı değildir.

Gerçek bir DWG açılmıyor ya da yavaşsa (ADR 8053):
`cargo run --release -p kentos-native-application --example open_cad -- DOSYA.dwg` masaüstünün Aç yoluyla açar,
süreyi ve raporu yazar (`KENTOS_FILE_UNIT=mm|cm|m`; `KENTOS_SAVE_KCAD=1` Kaydet'in KCAD kodlamasını da dener,
`KENTOS_SHOW=BLOK/NESNE` bir bloğun nesnesini yazar); `cargo run --release -p kentos-dwg --example diagnose -- DOSYA.dwg`
aşamaları ayrı ölçer: kodek, okunmayan nesnelerin ayıklanması, köprü DXF'in boyutu ve süresi, KentOS'un okuması
(`KENTOS_DIAG_WRITE=KLASÖR` köprüyü yazar). Kişisel proje dosyaları depoya girmez.
