# ADR 0413: Öznitelik tablosunda koşullu biçim

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek. TODOS.md §16.6 `FORM-T`.
- **Bağlam belgesi:** ADR 0199 (katman alanları, Öznitelik tablosu), 0211 (katman süzgeci, koşulun dili), 0401 (formlar), 0165
  (ifadeler çizim biriminde).
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

Formla girilen kayıtlar Öznitelik tablosunda gözden geçirilir. Ağır hasarlı binayı, eksik notu, sınırı aşan değeri tabloda
bir bakışta görmek için QGIS'in öznitelik tablosunda **koşullu biçim** vardır: satır ya da alan için koşul, zemin rengi, yazı
rengi ve yazı tipi; kurallar katmanla kaydedilir. KentOS'un tablosu yalnız kuralına uymayan değeri uyarı zeminiyle gösteriyordu.

## Karar

### 1. Kural

Katmanın `tableStyles`'ı en çok 50 kuralın sıralı listesidir (`kentos_contracts::TableStyle`):

| Alan | Anlamı |
|---|---|
| `field` | boyadığı alanın adı; yoksa bütün satır |
| `condition` | kayıt üzerinde ifade (İfadeyle seç'in dili, katmanın alanları adlarıyla), en çok 2000 karakter |
| `background`, `color` | `#RRGGBB` zemin ve yazı rengi |
| `bold` | kalın yazı (yalnız `true` yazılır) |

Kural en az birini verir (zemin, yazı rengi ya da kalınlık). Bir hücrenin görünüşü (`cell_look`): önce o alanın kuralları, sonra
satırın kuralları, sırayla; koşulu tutan ilk kural zemini, ilk kural yazı rengini, ilk kural kalınlığı verir. Kuralların biçim
denetimi `table_styles_problem` (okuyucular, komut, sunucu ortak).

### 2. Komut

`cad.layers.table` v1 (“Koşullu biçim”): `layer`, `styles` (boş liste kuralları kaldırır), `expectedRevision`. Sıra: kuralların
biçimi (`invalid_table_styles`, yol `styles/i`), sürüm, katman (`layer_not_found`, `not_a_layer`, `service_layer`), her kuralın
alanı katmanda (`field_not_found`, `styles/i/field`) ve koşulu (`invalid_expression`, `styles/i/condition`). Koşul projenin okuma
biriminde derlenir (ADR 0165); `$sıra` ve `$ölçek` reddedilir: biçim tablonun sırasına ve çizimin ölçeğine göre değişmez. Biçim
çizimi değiştirmez: kilitli katmanın kuralı da yazılır. Tek geri alma adımıdır (`Op::LayerTableStyles`); aynı kurallar adım
yazmaz. Python `kentos.cad.layers.table`, MCP'de araç (yıkıcı: kuralları siler).

### 3. Dosya

`tableStyles` `.kcad` şema 72'dedir (formlarla aynı şema), katmanın haritasında opak JSON olarak; grup taşımaz. Şema 59 okuyucusu
alanı `unknown_field` ile reddeder (`broken/table-styles-in-schema-71.kcad`). Örnek `forms.json`'un Bina katmanı iki kural taşır;
bağımsız `tools/kcad/kcad.py` ve `kcad_v2_reference.py` aynı kuralla.

### 4. Masaüstünde

- **Tablo:** her kuralın koşulu bir kez, katmanın bütün nesnelerinde ifade motorunun sütun yoluyla değerlendirilir (süzgeçle aynı
  yol); okunamayan koşul hiçbir satırda tutmaz. Satırın zemini satırın kendi zeminidir (`TableLine::tint`, hücre aralarıyla
  boşluksuz; seçim ve üzerine gelme onun yerine geçer); alanın kuralı satırınkinden başka bir zemin verirse yalnız o hücre
  boyanır. Zemin verilip yazı rengi verilmemişse yazı zemine göre koyu ya da açık seçilir. Sayı sütunu sağda ve rakam
  yazısıyla kalır; kurala uymayan değerin uyarı zemini kuralın önündedir.
- **Pencere:** Veri › Tablo'da Koşullu biçim (ikon: tablo ve boya damlası). Katman seçilir; kurallar listede örnekleriyle
  (“Aa 12”), Yeni kural, Yukarı ve Aşağı, Kuralı sil; seçili kuralın Uygulandığı yer (Bütün satır ya da alan), Koşul (ε İfade
  oluşturucu, okunamayan koşulun nedeni altında), Zemin rengi, Yazı rengi (renk seçici), Kalın ve önizleme. Kaydet kuralları
  komutla tek adımda yazar ve pencereyi kapatır; reddedilen kurallar pencerede kalır.

## Denetimler

- `scripts/fixtures/table_styles_command_cases.py --check`: komutun 25 ortak durumu ve hücre görünüşünün 20 durumu
  (`fixtures/table-styles/v1/looks.json`) KentOS kodu olmadan; masaüstü, Python ve sözleşmenin testi aynı dosyaları geçer.
- `kentos-kcad`: örnek ve bozuk örnek; `kcad_v2_reference.py --check`.
- Masaüstü: `form_scenes`'in `a_tables_rules_are_written_through_the_window_in_one_step`; resimler `form-kosullu-tablo`,
  `form-kosullu-bicim`.

## Sonuçlar

- Kayıtların durumu tabloda renkle okunur; kurallar katmanla gider ve gelir.
- Açık: kuralın yazı tipi ve simgesi (QGIS'in ikon sütunu), haritada aynı kuralla vurgulama, kuralların başka katmana kopyalanması.
