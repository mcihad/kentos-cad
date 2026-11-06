# 1540 — Noktanın görünüşü ve düzenlenişi: dolu nokta, ad ve kod görünürlüğü, ayıklananlar katmanı

Tarih: 2026-11-06. Sahibinin istekleri (6 Kasım 2026 akşamı):
- varsayılan nokta sembolü gerçek nokta olsun, içinde nokta olan daire olmasın;
- Nokta şeritte Çizgi gibi büyük düğme olsun;
- nokta listesinde ad ve kod kolayca değişsin;
- nokta birleştirmede birleştirilenler ayrı katmana atılabilsin;
- üstteki arama “komut ara” değil “menülerde ara” olsun;
- Görünüm'de taramalar gibi nokta adları ve kodları açılıp kapansın, ayar olarak da bulunsun.

Durum: taslak. İlgili ADR'ler:
- [ADR 0153](0153-point-editor.md): Nokta editörü, Çift noktaları ayıkla;
- [ADR 0212](0212-label-engine.md): etiket motoru;
- [ADR 0077](0077-desktop-command-search.md): şerit araması.

## Dayanak

- **AutoCAD:** PDMODE'un ilk değeri 0'dır, yani noktayı küçük bir nokta olarak çizer. 32 halka, 33 içi noktalı
  halkadır ([PDMODE](https://help.autodesk.com/cloudhelp/2018/ENU/AutoCAD-LT/files/GUID-82F9BB52-D026-4D6A-ABA6-BF29641F459B.htm);
  [BricsCAD PDMODE](https://help.bricsys.com/pl-pl/document/system-variable-reference/p/pdmode-system-variable)).
  KentOS'un simgesiz katmanı bugüne dek içi noktalı halka çiziyordu.
- **Civil 3D:** noktanın sembolü (point style) ile etiketi (point label style) ayrıdır. Etiket numarayı, kotu ve
  açıklamayı gösterir; “Point Number Only” gibi bir seçim veriyi silmez, yalnız gösterimi değiştirir
  ([Edit Feature Settings – Point](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-CC37A096-19E6-4F8B-AB92-278FB5B69ECD.htm);
  [About Points](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-3481A6B7-B16E-4636-8FC0-F28119CEC0FD.htm)).
  Netcad'in nokta gösterimi de nokta adı, kodu ve kotunu ayrı ayrı açıp kapatır (sahibinin kullanımı).
- **Office:** şerit araması bir komut listesi değil, menülerdeki işi yeriyle bulur. KentOS'un araması zaten her
  satırda şeritteki yeri söylüyor (ADR 0077); yalnız adı bunu söylemiyordu.
- **Türk mevzuatı:** BÖHHBÜY ek-1 Detay ve Öznitelik Kataloğu noktayı kodu ve numarasıyla tanımlar; bu ad ve kodu
  çizimde göstermek ölçmecinin olağan iş akışıdır. Mevzuat bir ekran sembolü dayatmaz.

## Karar

1. **Simgesiz katmanın noktası dolu noktadır.**
   - Katmanın rengiyle, ekranda 4 px çaplı dolu bir daire çizilir (`simple::DOT_SIZE`; çizim hattı `circle`, pafta
     hattı `marker_shape::DOT`).
   - Katmana seçilmiş sembol aynen kalır: Kot noktalarının artısı, Poligon noktalarının üçgeni, halka.
   - Bir dosya alanı değişmez; `.kcad` şeması aynı kalır.
2. **Şerit.**
   - Giriş › Çizim'de Nokta büyük düğmedir: Çizgi, Çoklu çizgi, Nokta, Daire.
   - Yay küçük sütuna iner; Giriş sekmesi 1100 ve 1440 px'e sığar.
   - Aracın ikonu dolu noktadır (`pointDot`; seçenekleri sahibine gönderildi, B seçildi).
3. **Nokta listesi.**
   - Tek seçili satırın Ad ve Kod hücresine tek tıklama, dosya listesinde seçili adın yeniden adlandırılması gibi
     düzenlemeyi açar.
   - Öbür hücreler ve öbür satırlar çift tıklamayla düzenlenir (ADR 0153).
4. **Çift noktaları ayıkla › Ayıklananlar: Sil | Katmana taşı.**
   - Katmana taşı'da fazla noktalar silinmez; verilen adlı katmana taşınır (ilk değer “Ayıklanan noktalar”).
   - O adda bir katman yoksa ilk ayıklanan noktanın yanında açılır.
   - Katman açma ve taşıma tek geri alma adımıdır (“Çift noktaları ayıkla”).
   - Kilitli hedef katman reddedilir, iletisi nedenini ve çözümü söyler.
   - Seçim, uygulama açık kaldıkça bir sonraki pencereye taşınır.
5. **Menülerde ara.** Üstteki kutunun adı, ipucu ve boş sonucu “Menülerde ara” der. `view.commandSearch`'in masaüstü
   başlığı da budur (`DESKTOP_TITLES`).
6. **Nokta adları ve Nokta kodları.**
   - **Ayarlar:** `graphics.pointNames` (açık) ve `graphics.pointCodes` (kapalı). Uygulama ayarları › Görünüm'de ve
     Görünüm › Görünüş'teki iki düğmede bulunurlar (`view.pointNames`, `view.pointCodes`; Harita ve Yol).
   - **Etiketler:** etiket motoru noktanın metnini ad, kod ya da “ad␣␣kod” olarak yapar.
     - Katman etiketlemiyorsa (Yok) bir şey eklenmez.
     - Varsayılan durumda (adlar açık, kodlar kapalı) katmanın kendi etiket şablonu aynen kalır.
   - **Belge:** değişmez.
   - **Kapsam dışı:** paftanın kendi görünümleri bugünkü (çizimin) etiketlerini kullanır.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/point_dedupe_cases.py` 6 yeni “Ayıklananlar” durumuyla 26 durum üretir: yeni
  katman, var olan katman (büyük küçük harf ayrımı yok), kilitli katman, boş ad, aynı yer, ortalama. Desktop geçer.
- **Masaüstü** (`points/display_tests.rs`) şunları sınar:
  - dolu nokta sembolü;
  - ayarlar ve etiket metinleri: ad, ad+kod, yalnız kod, hiçbiri; belge revizyonu değişmez;
  - tek tıkla Ad/Kod düzenleme (gerçek tıklama);
  - ayıklananların katmana taşınması ve tek adımda geri alınması.

  `ribbon_tests`, `module_tests`, `history::text`, `catalog`, `settings` ve `labels` geçer.
- **Lejant:** `fixtures/style/v1/legend.json`'un simgesiz katman satırı yeni noktayla güncellendi. Web dururken
  (ADR 0228) web'in `style/fromLayer.ts`'i aynısını yapmalı (TODOS §0.2).
- **Resimler:** `.run/gui-evidence/nokta-gorunum-1540/`; ikon seçenekleri, şerit, Görünüm, liste, ayıklama ve arama.

## Sonra

- Nokta kotunun da açılıp kapanması (Netcad'in üçüncü gösterimi) ve ad/kod/kot için ayrı yer (sağ üst, sağ alt).
- Katman stilinde nokta sembolü seçimi (Nokta, Halka, Artı, Üçgen) ve boyutu.
- Paftanın görünümünde ad ve kod tercihleri.
