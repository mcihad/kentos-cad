# Dinamik bloğu kaydetmeden denemek

1. Bloğu seçip **Düzenle** ile blok düzenleyicisini açın.
2. Geometriyi düzenleyin; alt çubuktaki **Test et** düğmesine basın.
3. Seçili örneğin **Blok parametreleri** bölümünde boyut/görünürlük/katalog
   değerlerini değiştirin veya tutamaçları deneyin. Sınır ve adım bilgisi
   aynı proje birimindedir; formüllü değerler salt okunur.
4. **Düzenleyiciye dön** ile taslağa dönün. Testteki değerler atılır;
   düzenleyicideki kaydedilmemiş geometri ve geri alma geçmişi kalır.
5. **Kaydet** tanımı asıl çizime uygular; **Vazgeç** asıl çizimi değiştirmez.

Test bir dışa aktarım veya bileşen sürümü oluşturma işlemi değildir. Python
ve AI da asıl belge üzerinde deneme yapmak yerine bağımsız belge kullanmalıdır:

```python
from kentos import cad

original = cad.Document.open("fixtures/kcad/v2/dynamic-blocks.kcad")
before = original.to_bytes()
test = cad.Document.from_bytes(before)
insert = next(test.entities())
cad.blocks.parameters(test, operation="set", uids=[insert.uid],
                      values={"Genişlik": 1.05, "Görünüş": "Sade"})
assert original.to_bytes() == before
# API fiziksel blokta metre/radyan, birimsiz blokta yerel tanım sayıları kullanır.
# Paneldeki 1050 mm, bu fiziksel kapı için API'de 1.05'tir.
test.undo()
assert test.to_bytes() == before
```

MCP'de ayrı çizim oturumu açılır; parametre denemesi aynı
`cad.blocks.parameters` girdisidir. `plan`/`validate` yazmaz. Güncel taslak
geometrisini sınamak için ayrı belgede önce `cad.blocks.edit` / `update`,
sonra `cad.blocks.insert`, ardından `cad.blocks.parameters` kullanılır.
Kaydetmek istenmeyen test belgesi kaynak dosyanın üzerine yazılmaz.
