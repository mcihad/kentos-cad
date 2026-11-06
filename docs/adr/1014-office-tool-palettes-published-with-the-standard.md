# ADR 1014: Ofis standardıyla yayımlanan araç paletleri

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` açığı “kişisel palet dışında
  yayımlanan ofis araç paleti”; ADR 1100 §4'ün sabitlediği model.
- **Bağlam belgesi:** ADR 1100 §4, docs/specs/drawing-standard-package.md (sürüm 2 paket), ADR 0013 (komut kataloğu).

## 1. Kaynak

AutoCAD araç paletleri paylaşılan bir klasörden ya da XTP/XPG dışa aktarımıyla dağıtılır; Civil 3D araç kataloğu
araçları katalogdan sürükler ve bağlı tutar (ADR 1100 §1'in kaynakları). KentOS kararı: palet ofis standardının
paketinde yayımlanır ve bağlı olduğu yerde okunur; çizime kurulmaz. Her araç simge değil, komut kaydına bağlı bir
tariftir.

## 2. Karar

- Sürüm 2 paketin `resources.toolPalettes`'i: `{id, name, description?, tools: [{id, title, description?, category?,
  command, commandVersion, input}]}`. Biçim kuralı (`tool_palettes_problem`): en çok 50 palet, kimlik ve ad 1–128
  bayt ve kenar boşluksuz, palet kimliği büyük/küçük harf ayırmadan bir kez; palette 1–200 araç, araç kimliği palette
  bir kez; sürüm ≥ 1; girdi en çok 64 KiB'lık bir nesne; bilinmeyen alan reddedilir. Biçimi bozuk paket bağlanmaz
  (`cad.settings.set` okur).
- Birden çok standartta aynı palet kimliği ilk standardındır (öbür kaynakların kuralı); kökeni `provenance.toolPalettes`.
- `cad.standards.palettes` v1 (salt okunur): paletleri standartların sırasıyla verir. Her araç için kişisel favori
  anahtarı `standart:<paket>:<palet>:<araç>`, tarifin özeti (komut, sürüm ve girdinin anahtarları sıralı, boşluksuz
  JSON'unun SHA-256'sı) ve `ready`: komut bu konakta o sürümle çalışıyor ve girdiyi bu çizimde kabul ediyor (komutun
  kendi `validate`'i, çizimin kopyasında). Değilse `problem`: komutun kodu, sözü ve yeri (`input.<alan>`);
  çalışmayan komut ya da başka sürüm `unknown_command`. Bağlı standart yoksa palet yok; dosyada kalmış okunmayan paket
  `invalid_standard`.
- Masaüstü Şablonlar paneli **Ofis paletleri** bölümünü panel açılınca ve bir araç çalıştıktan sonra okur (her
  düzenlemede değil: araç her durumda komutun kendi denetiminden geçer). Hazır araç basılınca komutu Python ve MCP'nin
  çağırdığı kayıttan çalışır, tek geri alma adımıdır, sonucu söylenir; hazır olmayan sönük ve nedeniyle görünür.

## 3. Kanıt

`scripts/fixtures/standards_palettes_cases.py` (KentOS kodu yok; özeti aynı kanonik kuralla Python hesaplar) 23 vaka
yazar: palet yok, hazır araçlar, plan/doğrulama, eksik katman (`layer_not_found`), kilitli katman (`layer_locked`),
bilinmeyen komut, başka sürüm, okunmayan girdi (`invalid_input`), karışık palet, iki standartta öncelik ve sıra,
revizyonda değişen aracın aynı anahtarla başka özeti, okumanın çizimi değiştirmemesi, biçimi bozuk paketlerin
bağlanmaması. Hepsi geçti. `review_take::palettes_need_a_readable_bound_standard` sorgunun kendi retini sınar.
Masaüstü testi ve resmi `ofis-paleti`: hazır araç çizgiyi çizer ve geri alınabilir, eksik katmanlı araç basılamaz.

## 4. Sınırlar

- Kişisel favori listesine ofis aracı eklemek ve özeti değişen favoriyi “değişti” diye göstermek masaüstünde yok;
  anahtar ve özet sözleşmededir.
- Eksik girdi alanını kullanıcıdan isteyen yarım tarif yok: tarif tam girdi taşır.
- Web dururken (ADR 0228) web paneli yok.
