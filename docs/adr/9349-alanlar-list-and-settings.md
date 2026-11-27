# ADR 9349: Alanlar penceresi liste ve ayarlar olarak yeniden düzenlendi

- **Durum:** kabul edildi (2026-11-27). Kaynak: sahibin sözü, 27 Kasım 2026: “Alanlar tıklanınca açılan form çok
  karmaşık ve kullanıcı deneyimi de çok düşük, tasarımı da kötü onu da iyileştir”.
- **İlgili:**
  - [ADR 0199](0199-layer-fields-and-feature-table.md) §3 (Alanlar);
  - [ADR 0419](0419-split-and-merge-policies.md) (bölünce, birleşince);
  - [ADR 0438](0438-record-rules-and-field-extras.md) (tekil, hariç uçlar, birim, değerin açıklaması);
  - [ADR 9318](9318-data-layer-columns.md) ve [ADR 9324](9324-field-rules-in-the-source.md) (veri katmanının alanı ve
    kuralı);
  - DESIGN.md §7.9 (liste ve ayrıntı), kitin `ListDetails` ve `Form`'u.

## Bağlam

Pencere 1 040 piksel genişlikte dokuz sütunlu, her hücresi düzenlenebilir bir ızgaraydı:

- Ad, Takma ad, Tür, Uzunluk / Basamak, Zorunlu, Varsayılan, En az, En çok, Değer listesi.
  - Türün almadığı hücreler gri ama yerinde duruyordu.
- Izgaranın altında sekiz düğme yan yanaydı: Alan ekle, Sil, Yukarı, Aşağı, Verilerden al, Değerleri biçime çevir,
  Kaynak şemasını yenile, Kuralları kaynağa yaz.
- Onların altında dört ayrı parça vardı: seçili alanın onay kutuları satırı, kural sözü, bölünce/birleşince seçicileri
  ve kullanıldığı yerler.
- Seçili alanın ayarlarının hangi alana ait olduğu ancak satırın rengiyle anlaşılıyordu.

## Araştırma

- **QGIS, katman özellikleri › Öznitelik formu:** solda alanların listesi, sağda seçili alanın ayarları durur
  (düzenleyici, kısıtlar, varsayılan); seçim değişince sağ taraf o alanın olur
  ([Configure the field behavior](https://docs.qgis.org/latest/en/docs/user_manual/working_with_vector/vector_properties.html#configure-the-field-behavior)).
- **KentOS'ta liste ve ayrıntı:** koleksiyon pencereleri kitin `ListDetails`'idir. Sol listede öğeler, sağda seçili
  öğenin formu, listenin eylemi listenin altında durur (DESIGN.md §7.9; sahibin form kuralı).
- **KentOS kararı:** Alanlar bir koleksiyon penceresidir. Izgara yerine alan listesi ve seçili alanın tek formu;
  seyrek kullanılan eylemler bir menüde.

## Karar

### 1. Düzen

```text
Alanlar
bakim                                                                     Diğer ▾
GeoPackage tablosu “bakim”: alanlar tablonun sütunlarıdır.
┌ alanlar ─────────────────┐│┌ seçili alanın ayarları ──────────────────────┐
│ kod            Metin     ││ │ Genel ───────────────────────────            │
│ Bakım kodu               ││ │  Ad *          [mahalle              ]       │
│ bedel     Ondalık sayı   ││ │  Takma ad      [Mahalle              ]       │
│ mahalle        Metin     ││ │  Tür           [Metin           ▾]           │
│ Mahalle   ≣ zorunlu      ││ │  Uzunluk       [4     ]                      │
│                          ││ │ Değerler ────────────────────────            │
│                          ││ │  [x] Zorunlu  [ ] Tekil                      │
│                          ││ │  Varsayılan    [                ]            │
│                          ││ │  Değer listesi 3 değer [Düzenle…]            │
├──────────────────────────┤│ │ Nesne işlemleri ─────────────────            │
│ [+ Alan ekle]     ˄ ˅ ⌫  ││ │  Nesne bölününce [Kopyala ▾] …               │
└──────────────────────────┘│└──────────────────────────────────────────────┘
✓ 3 alan.                                                    Vazgeç  Kaydet
```

- **Başlık:** katmanın yolu ve alanların nerede durduğu: “GeoPackage tablosu “bakim”: alanlar tablonun
  sütunlarıdır.”, veritabanında “Veritabanı tablosu …”, çizim katmanında “Alanlar katmanın nesnelerinin
  öznitelikleridir.”
- **Liste** (260 piksel, kendi içinde kayar):
  - her alan adı ve altında takma adıyla durur; sağda türü;
  - işaretler: “yeni”, “ad değişiyor”, değer listesi simgesi, “zorunlu”;
  - seçili alan vurgulanır.
- **Listenin altı:** Alan ekle; seçili alanda Yukarı, Aşağı ve Sil (simgeli, adları ipucunda).
- **Diğer ▾:** Verilerden al, Değerleri biçime çevir; veri katmanında Kaynak şemasını yenile ve Kuralları kaynağa
  yaz. Her satır ne yaptığını altında söyler.
- **Ayarlar** (kalan genişlik, kendi içinde kayar): kitin `Form`'u, tek etiket sütunu.
  - **Genel:** Ad (zorunlu; ad değişince önceki adı uyarıyla söylenir), Takma ad (“Tablonun başlığında, formda ve
    Öznitelikler'de görünür.”), Tür, metinde Uzunluk, ondalıkta Ondalık basamak, sayıda Birim.
  - **Değerler:** Zorunlu ve Tekil; Varsayılan; sayı ve tarihte En az ve En çok, yanlarında “hariç”; kod tutan
    türlerde Değer listesi (kaç değer, ilk dördü “Çınar (M1)” diye, Düzenle… ya da Liste ekle…).
  - **Nesne işlemleri:** Nesne bölününce, Nesneler birleşince, Dışarı verilmez.
  - **Kaynak:** tablonun sütununun sözü, tabloda tekil kısıtı, kuralın nerede durduğu.
  - **Kullanıldığı yerler** ve bağlı tablonun kısıtları en altta.
- **Alt çubuk:** solda pencerenin sözü (yaptığı, yoksa ilk sorun, yoksa “3 alan.” ya da “4 alan; değişiklikler
  Kaydet ile yazılır.”), sağda Vazgeç ve Kaydet.
- **Türün almadığı ayar gösterilmez:** önceden gri hücre olarak yerinde duruyordu.

### 2. Değişmeyenler

- **Olaylar ve iz kontrolleri:** pencerenin olayları (`Type`, `Kind`, `Required`, `Choose`, `Add` …) ve izlerin
  denetim sözcükleri (Ad, Takma ad, Uzunluk / Basamak, Varsayılan, En az, En çok, Zorunlu, düğme adları) aynıdır.
- **Kutuların kimliği:** ayarlar formundaki kutular seçili alanın kimliğini (`cell_id`) taşır. Kutuya tıklamak ya da
  Tab ile girmek o alanı seçer (GIS-003).
- **Değer listesi penceresi:** aynı içerikle ayrı pencere olarak açılır.
- **Kaydet:** alanları ve yeniden adlandırılan anahtarları tek geri alma adımıyla yazar; veri katmanında önce yeni
  sütunu ekler (ADR 9318).

### 3. Dosyalar

Görünüm `gis/layer_fields_view.rs`'ye ayrıldı; `layer_fields.rs` 1 937 satırdan 1 348'e indi.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Alanlar'ın liste ve ayarlar düzeni | var: katman alanları aynı penceredir | var | var | var |
| Diğer ▾'de Kaynak şemasını yenile, Kuralları kaynağa yaz | yok: veri katmanı yoktur | var: veri katmanında | var: aynı | var |

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Pencerenin düzenidir. Alanları `cad.layers.edit` (`fields`) ve `cad.layers.columns` değiştirir; komutlar değişmedi. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Alan arama:** çok alanlı katmanda listede arama kutusu yok.
- **Sürükleyerek sıralama:** sıra Yukarı ve Aşağı ile değişir.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `layer_fields_tests::the_chosen_fields_settings_are_its_own_and_editing_them_changes_it`: mahalle seçiliyken takma
    ad onun olarak yazılıyor, kod'unki değişmiyor.
  - `layer_fields::tests::a_click_into_a_rows_box_chooses_that_row` (GIS-003, yeni düzene uyarlandı): listedeki ikinci
    alana basmak onu seçiyor; Tekil onun, ad kutusu ayarlarda.
  - `-- layer_fields recipe_layer form_scenes forms:: module_tests ribbon_tests history::text catalog settings`: 292
    geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır.
  - `cargo test -p kentos-cad` (izler): 37 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9349/`, açık ve koyu tema; büyütülerek bakıldı; önceki düzenin resmi
  karşılaştırma için çekildi):
  - `01-alanlar`: liste ve mahalle'nin ayarları (Genel, Değerler, Nesne işlemleri);
  - `02-deger-listesi`: Değer listesi · mahalle;
  - `03-yeni-alan`: “Alan 4” yeni işaretiyle, “Liste ekle…”, alt çubukta “değişiklikler Kaydet ile yazılır”.
