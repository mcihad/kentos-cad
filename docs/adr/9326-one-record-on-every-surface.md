# ADR 9326: Tabloda seçilen veri satırı Öznitelikler'in ve Koordinat listesinin de kaydıdır

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  5, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:** [ADR 9102](9102-data-table.md) §5 (veri satırının seçimi haritada),
  [ADR 9129](9129-data-layers-in-source-status-and-table-picker.md) (tablonun katmanı ve çizim hedefi),
  [ADR 9319](9319-a-records-form-for-reading.md) (kaydın formu okumak için).

## Bağlam

- **Kusur:** projeye iki kaynak katmanı eklendi; biri Katmanlar'da etkin bırakıldı, tablonun Katman seçicisinden öbürü
  açıldı ve bir satırı seçildi.
  - Tablo yeni katmanın satırlarını gösterirken araç çubuğunda başka bir katman “Çizim hedefi” olarak görünüyordu.
  - Tablodaki seçim haritada mavi halka çizdi; sağdaki Öznitelikler “Seçili nesne yok” dedi.
  - Koordinat listesi çizimde bir nesne seçilmesini istedi.
  - Kaynak noktası ile proje nesnesi arasındaki ayrım panelde açıklanmıyordu.
- **Kod:**
  - Veri seçimi (`data_selection.rs`) seçilen satırları yalnız haritadaki çizgileri için, değerleri olmadan
    okuyordu.
  - Öznitelikler ve Koordinat listesi yalnız çizimin seçimine bakıyordu.

## Karar

### 1. Seçimin kimliği katman ve anahtardır

- Tabloda seçilen satırlar katmanın kimliği ve kaynak anahtarıyla tutulur (önceden de öyleydi).
- Okuma artık satırın değerlerini de alır (`SourceRow`: anahtar, değerler, katmanın nesnesi olarak geometri). Satır
  çizime alınmaz.
- Aynı GeoPackage'ın iki tablosunda aynı anahtar (fid 1) iki ayrı kayıttır; her yüzey seçilen tablonunkini gösterir.

### 2. Öznitelikler

- Çizimde seçim yokken tabloda veri satırı seçiliyse panelin başlığı “Veri satırı (N)” olur, ilk bölümü de aynı adı
  taşır.
  - **Bölümün satırları:** katman, anahtar (ör. `fid 2`) ve katmanın alanlarının değerleri. Kodlu alanda etiket
    gösterilir.
  - **Altında:** “Kaynaktan okundu; çizime alınmadı.” Birden çok satır seçiliyse ilki gösterilir ve bu söylenir.
- Değer burada değişmez. Tablonun hücresi ya da Formu aç › Düzenle değiştirir (ADR 9319).

### 3. Koordinat listesi

- Çizimde seçim yokken tablonun seçili satırlarının koordinatları listelenir.
  - Nokta: anahtarıyla adlandırılır, örneğin “fid 1 (kaynakta)”.
  - Çizgi ve alan: köşeleriyle; altında “fid 3 (kaynakta, çizime alınmadı)” yazar.
- Bu satırlar yerinde düzenlenmez.

### 4. Tablo ve çizim hedefi

Tablonun çubuğundaki “Çizim hedefi: …” sözü kalır. İpucu ikisinin ne için olduğunu söyler: tablo seçilen katmanın
satırlarını gösterir, Öznitelikler ve Koordinat listesi onları okur; çizim araçları etkin katmana çizer.

### 5. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Veri satırı Öznitelikler'de ve Koordinat listesinde | yok: veri katmanı CBS'nindir | yok: aynı gerekçe | yok: aynı gerekçe | var |

### 6. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Seçim ve paneller masaüstünün sunumudur. Satır `cad.layers.rows` ile anahtarıyla okunur. |
| MCP | yok | Aynı gerekçe; okuma aynı komutla. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Haritadan seçim:** haritada bir veri satırına tıklamak (Seç aracı) satırı alıp çizimin seçimine koyar
  (ADR 9103 §5). Tablonun seçimiyle birleşik tek bir seçim, Seç'in alınmadan seçmesini isteyen ayrı bir iştir.
- İki PostGIS şemasında aynı adlı tablo bu dilimde gerçek veritabanıyla sınanmadı. Kimlik katman ve anahtar olduğu için
  aynı kurala bağlıdır.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- properties bottom:: data_selection source_selection data_bar
  coords features::`: 125 geçti.
- **`source_selection_tests`:**
  - `bakim`'in 1. satırı seçilince Öznitelikler “Veri satırı (1)”, katman `bakim`, `fid 1`, `kod` B01 gösteriyor.
  - Koordinat listesi “fid 1 (kaynakta)” noktasını düzenlenemez olarak listeliyor.
  - `mahalleler`'in 1. satırı (aynı anahtar, başka tablo) seçilince `mahalle` M1, `ad` Çınar; `kod` yok.
  - Hiçbir satır çizime alınmıyor.
- **Resim** (`.run/gui-evidence/kullanim-9326/01-veri-satiri-oznitelikler`, açık ve koyu tema): tabloda `bakim`'in 2.
  satırı, Öznitelikler'de “Veri satırı (1)”, çubukta “Çizim hedefi: Parsel”.
