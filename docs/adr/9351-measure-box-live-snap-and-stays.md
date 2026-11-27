# ADR 9351: Ölçme araçları: ölçü kutusu ölçerken canlı, araç bitince kalır; ölçerken veri katmanlarına yakalama

- **Durum:** kabul edildi (2026-11-27). Kaynak: sahibin sözü, 27 Kasım 2026: “Ölçme (Ribon sekmesi) araçlarına odaklan
  … mesela mesafe ölçme aracı sonunda mesafe kutusu çıkmıyor alanda çıkıyor, aslında burada kopyalama olayı falan çok
  hoş ama burada snap çalışmalı ve hepsinden kentos ui içerisinde bunun için bir demo var ona benzer olursa çok hoş
  olur”.
- **İlgili:**
  - [ADR 9328](9328-quick-measure-box.md) (Ölçü kutusu);
  - [ADR 9103](9103-edit-session.md) §5 (düzenlenen veri katmanına yakalama);
  - kitin vitrinindeki Ölçüm bölmesi (`apps/ui-showcase/src/view/panes.rs` `measure_pane`).

## Bağlam

- **Kutu görünmüyor:** ölçü kutusu yalnız ölçen araç çalışırken görünüyordu.
  - Mesafe ölç'ü Enter'dan sonra Esc ya da Seç ile bırakınca kutu kayboluyordu.
  - Alan hesapla “Alan olarak çiz” için açık kaldığından onun kutusu görünüyordu.
- **Canlı ölçü yok:** kutu ancak ölçü bitince geliyordu; ölçerken toplam ve parçalar görünmüyordu.
- **Yakalama yok:** veri katmanlarının satırlarına yalnız katman düzenlenirken yakalanılıyordu (ADR 9103 §5). CBS
  projesinde katmanlar çoğunlukla veri katmanı ve düzenlemede değil; Mesafe ölç noktaları köşelere ve uçlara oturmuyordu.
- **Demo:** kitin vitrinindeki Ölçüm bölmesi büyük bir toplam, parçaların listesi, Kopyala, Geri al ve Temizle ile
  ölçümü canlı gösteriyor.

## Araştırma

- **QGIS, Ölç aracı:** ölçüm penceresi tıkladıkça parçaları ve toplamı canlı gösterir; imleçle süren parça da toplamda
  yer alır. Ölçüm yakalama ayarlarına uyar; pencere ölçüm bitince açık kalır, Yeni ile temizlenir
  ([Measuring](https://docs.qgis.org/latest/en/docs/user_manual/introduction/general_tools.html#measuring)).
- **KentOS kararı:**
  - Ölçü kutusu ölçerken canlıdır ve ölçü bitince açık kalır.
  - Ölçme araçları çizim araçları gibi yakalar, görünen bütün veri katmanlarının satırlarına da.

## Karar

### 1. Ölçü kutusu

- **Ölçerken (canlı):** Mesafe ölç'te bir nokta verilince, Alan hesapla'da üç köşe olunca kutu ölçüyü o anki hâliyle
  gösterir: verilen noktalar ve imlecin yakalanmış noktası.
  - Değerler aracın önizlemesinden aynı hesapla (`kentos_interaction::measured`) alınır; kutu ile çizim aynı şeyi
    söyler.
  - Başlığın yanında “sürüyor” çipi durur.
  - Notlarda: “Bitir: Enter bitirir, Esc vazgeçer. Kenet açıkken nokta yakalanan yere konur.”
- **Ölçü bitince:** son ölçü, aracı bitince (Esc, Seç) de kutuda kalır. Başka bir araç başlayınca kalkar; aynı araç
  yeniden açılınca olduğu gibi döner; × kaldırır.
- **Görünüş** (vitrinin Ölçüm bölmesi gibi):
  - büyük toplam: mesafede “Toplam 70.000 m”, alanda alan;
  - mesafenin parçaları “# | Parça” tablosunda;
  - öbür satırlar ad–değer çiftleriyle;
  - Kopyala ve × başlıkta simge düğmeleri;
  - birim, hesap ve sistem notları değişmedi.

### 2. Yakalama

- **Ölçme araçları:** Mesafe ölç, Alan hesapla, Açı ölç, Koordinat oku ve Kot oku.
- **Davranış:** bu araçlar çalışırken görünen bütün veri katmanları, düzenlenen bir katman gibi okunur. Okuma bütün
  ve sadeleştirilmemiş olur, satırlar yakalama için tutulur (en çok 20 000).
- **Yakalama noktası:** çizimin nesnelerine yakalamayla veri satırlarına yakalamanın yakını alınır (önceki kural).
- **Araç bitince:** veri katmanları yine yalnız düzenlenirken tutulur.

### 3. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Canlı ölçü kutusu, araç bitince kalan kutu | var: Ölçme sekmesi ortaktır | var | var | var |
| Ölçerken veri katmanlarına yakalama | yok: Mimari'de veri katmanı yoktur | var: CBS tabloları altlık olabilir | var: aynı | var |

### 4. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Ölçme araçları çizimi değiştirmeyen etkileşimli araçlardır; ekranda imleçle ölçülür, kanallar için komut karşılığı yoktur. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Yürüyen toplam:** parça tablosunda her parçanın yanında yürüyen toplam yok (vitrinde var); bitmiş ölçünün satırları
  biçimlenmiş metindir.
- **Geri al ve Temizle:** kutuda düğme olarak yok; aracın Geri seçeneği ve Esc onların işini görür.
- **Büyük tablo:** 20 000 satırdan büyük görünümde yakalama ilk 20 000 satırla sınırlıdır.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:**
  - `measure_box_tests::the_box_follows_the_measurement_and_stays_after_its_tool_ends`:
    - A ve C verilince kutu “1. parça 30 m” ile canlı;
    - B ve Enter'dan sonra canlı ölçü bitiyor;
    - Esc ile araç Seç'e dönünce kutu “Toplam 70 m” ile kalıyor;
    - Alan hesapla'da üçgen canlı 600 m².
  - Önceki `measure_box_tests` sınamaları geçti: başka araçla kutu kalkıyor, aynı araçla dönüyor.
  - `data_edits_tests::a_measuring_tool_keeps_every_shown_layer_for_the_snap`: düzenlenmeyen, etiketsiz katman ölçerken
    40 satırını tutuyor; 22. parselin köşesine yakalanıyor.
  - `-- data_edits_tests measure query_tests transparent_tests tools_scenes module_tests ribbon_tests history::text
    catalog settings`: 198 geçti. Kırmızı tek sınama bilinen bağımlılık tabanıdır (render birleşmesinin bağları).
  - `cargo test -p kentos-cad` (izler): 37 geçti.
- **Resim** (`.run/gui-evidence/kullanim-9351/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-mesafe-suruyor`: “Mesafe ölçüsü · sürüyor”, Toplam 70.000 m, parçalar 30 ve 40;
  - `02-mesafe-bitti`: araç bitince kutu kalıyor.
