# Stil alışverişinin profili

Bu belge `catalog/v1/style-exchange.json`'dan `scripts/style/exchange.py` ile üretilir; elle düzenlemeyin. Her satırın örneği kendi biçiminin testinde yazılıp geri okunur, profil kodla aynı değilse test düşer ([ADR 0635](../adr/0635-style-fields-and-exchange.md) §8). Taşınmayan her şey içe ve dışa aktarmanın raporunda adıyla söylenir.

## İşleyiciler

| İşleyici | Örnek | SLD 1.1 | QML (QGIS) | MapLibre |
|---|---|---|---|---|
| Tek sembol (`single`) | alan | İki yönde | İki yönde | İki yönde |
| Kategorili (`categorized`) | alan | İki yönde | İki yönde | İki yönde |
| Aralıklı (`graduated`) | alan | İki yönde | İki yönde (QGIS'in aralıkları alt sınırı dışarıda bırakır: sınırdaki değer bir alttaki sınıfa düşer.) | İki yönde |
| Kurallı (`rules`) | alan | İki yönde | İki yönde | İki yönde |
| Sürekli renk (`unclassed`) | alan | Taşınmaz | Taşınmaz | Taşınmaz |
| Orantılı sembol (`proportional`) | nokta | Taşınmaz | Taşınmaz | Taşınmaz |
| İki değişkenli renk (`bivariate`) | alan | Taşınmaz | Taşınmaz | Taşınmaz |
| Nokta yoğunluğu (`dotDensity`) | alan | Taşınmaz | Taşınmaz | Taşınmaz |
| Grafik (`chart`) | alan | Taşınmaz | Taşınmaz | Taşınmaz |
| Isı haritası (`heatmap`) | nokta | Taşınmaz | İki yönde (Saydamlık QGIS'te katmanın saydamlığıdır; yazılmaz.) | İki yönde |
| Kümeleme (`cluster`) | nokta | Taşınmaz | İki yönde (Kümenin sayısı ve sayıyla büyüyen simge QML'e yazılmaz.) | Taşınmaz |
| Yayma (`displacement`) | nokta | Taşınmaz | İki yönde | Taşınmaz |
| Ters alan (`inverted`) | alan | Taşınmaz | İki yönde | Taşınmaz |

## Sembol katmanları

| Sembol katmanı | Simge | SLD 1.1 | QML (QGIS) | MapLibre |
|---|---|---|---|---|
| Dolgu (`simpleFill`) | dolgu | İki yönde | İki yönde | İki yönde |
| Çizgi (`simpleLine`) | çizgi | İki yönde | İki yönde | İki yönde |
| Merkez işareti (`centroidMarker`) | dolgu | Yazılır, başka türle geri gelir | İki yönde | Taşınmaz |
| Tarama dolgusu (`hatchFill`) | dolgu | Taşınmaz | İki yönde | Taşınmaz |
| Degrade dolgu (`gradientFill`) | dolgu | Taşınmaz | İki yönde | Taşınmaz |
| Nokta deseni (`patternFill`) | dolgu | İki yönde (Hücre aralığı yazılmaz: SE'nin GraphicFill'i işareti kendi boyunda döşer.) | İki yönde (Dizilim QGIS'te nesnenin kutusundan başlar; işaretler bir hücre kayabilir.) | Taşınmaz |
| Resim dolgusu (`imageFill`) | dolgu | Taşınmaz | Taşınmaz | Taşınmaz |
| İşaretli çizgi (`markerLine`) | çizgi | Taşınmaz | İki yönde | Taşınmaz |
| Geometri üretici (`geometryGenerator`) | dolgu | Taşınmaz | Taşınmaz | Taşınmaz |
| Şekil işareti (`shape`) | işaret | İki yönde | İki yönde | İki yönde |
| SVG işareti (`svg`) | işaret | İki yönde | İki yönde | Taşınmaz |
| Yazı işareti (`text`) | işaret | İki yönde | İki yönde | İki yönde |
| Resim işareti (`raster`) | işaret | Taşınmaz | Taşınmaz | Taşınmaz |
