# KentOS DWG adaptörü

Codec kaynakları bu depoya kopyalanmaz. `acadrust`, OpenCADStudio'nun kullandığı
OpenCADCodec Git deposundan workspace `rev` ve `Cargo.lock` ile alınır.
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
