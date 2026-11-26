# ADR 9328: Ölçme › Hızlı ölçüm ve ölçü kutusu

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  10, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 0140](0140-new-drawing-and-editing-tools.md) (Mesafe ölç, Alan hesapla, Açı ölç, Koordinat oku);
  - [ADR 1097](1097-geographic-project-measures-on-the-ellipsoid.md) (coğrafi projede elipsoit üstünde ölçü);
  - [ADR 0167](0167-second-crs-and-converter.md) (ikinci koordinat sistemi);
  - [ADR 0622](0622-interface-arrangement.md) (şerit) ve [ADR 0802](0802-strict-module-separation.md) (modülün
    Ölçme sekmesi).

## Bağlam

- **Gözlem:** boş CBS projesinde Ölçme sekmesinde mesafe, açı, alan ve koordinat okuma yoktu; araçlar Harita › Sorgu
  grubundaydı.
  - Mesafe ve alan sonucu yalnız durum satırında ve Komut geçmişi'nde söyleniyordu.
  - Düzlemsel ya da elipsoidal kip, açı biriminin açık adı ve sonucun koordinat sistemi birlikte görünmüyordu.
- **İncelemenin kabulü:**
  - Ölçme › Hızlı ölçüm grubu aynı dört komutu taşısın; Harita'daki yerleri kalsın.
  - Sabit bir sonuç kutusu parçaları ve toplamı, çevreyi ve alanı, birimleri, hesap kipini, sistemi ve Kopyala'yı
    göstersin; Y ve X doğu ve kuzey adlarıyla yazılsın.
  - 30–40–50 m'lik üçgenle 50 m, 70 m, 600 m² ve 100 gon/90° sınansın.
- **Kod:** araçlar sonucu yalnız iletiyle (`cx.say`) söylüyordu; kutunun okuyacağı bir sonuç yoktu.

## Araştırma

- **ArcGIS Pro:** Measure aracının penceresi sonuçları tutar ve Copy Results ile verir.
  - Planar ölçü 2B Kartezyen hesaptır ve yalnız projeksiyonlu sistemde vardır; Geodesic ölçü elipsoit üstündeki en kısa
    yoldur.
  - Sistemi bilinmeyen haritada sonuç bilinmeyen birimde, düzlemseldir.
  - Kaynak: [Measure](https://pro.arcgis.com/en/pro-app/latest/help/mapping/navigation/measure.htm).
- **QGIS:** ölçü penceresi toplamı ve parçaları birimiyle birlikte gösterir, Kartezyen ya da elipsoidal hesap orada
  seçilir ([haritada ölçüm](https://docs.qgis.org/4.4/en/docs/user_manual/map_views/map_view.html#measuring)). Ölçü
  sınıfı kaynak sistemi ve elipsoidi ayrı tutar
  ([QgsDistanceArea](https://api.qgis.org/api/classQgsDistanceArea.html)).
- **KentOS kararı:**
  - Dört komut Ölçme'de de durur.
  - Sonuç, aracın yanında sabit bir kutuda gösterilir. Hesap yolu seçilmez, söylenir: projenin sistemi onu belirler
    (ADR 1097).

## Karar

### 1. Ölçme › Hızlı ölçüm

- CBS'nin Ölçme sekmesinin ilk grubu Hızlı ölçüm'dür: Mesafe ölç (büyük), Alan hesapla, Açı ölç, Koordinat oku.
- Komutlar aynıdır (`tool.measure`, `tool.area`, `tool.measureAngle`, `crs.query`). Harita › Sorgu'daki yerleri ve
  kısayolları kalır.

### 2. Ölçü kutusu

- **Yer:** çizim alanının sol alt köşesi. Kutu, ölçünün aracı çalıştıkça görünür; başka bir araç başlayınca gider, o
  araç yeniden seçilince son ölçüyle döner.
- **Başlık:** ölçünün adı (“Mesafe ölçüsü”, “Alan ölçüsü”, “Açı ölçüsü”, “Okunan koordinat”). Yanında Kopyala ve ×.
- **Satırlar:**
  - **Mesafe ölç:** her parça (“1. parça”) ve Toplam. Sabit ilk nokta açıkken her ışın uzunluğu ve yönüyle.
  - **Alan hesapla:** Alan ve Çevre. İçine tıkla'da delikler alandan düşülür.
  - **Açı ölç:** Açı ve Dış açı.
  - **Koordinat oku:** Y (doğu), X (kuzey), kotu varsa Z (kot); UCS'de ve ikinci sistemdeki değerleri.
  - Zemin yüksekliği olan projeksiyonlu projede elipsoit üstündeki ve zemindeki ölçüler, ikinci sistemi olan projede
    o sistemin düzlemindeki ölçüler de birer satırdır.
- **Satırların altında:**
  - **Birim:** adıyla; örneğin “metre, metrekare”, “gon (bir tur 400 g)” ya da “derece (bir tur 360°)”.
  - **Hesap:**
    - projeksiyonlu sistemde “Düzlemde: projeksiyon koordinatlarıyla, ölçek düzeltmesi yok”;
    - coğrafi sistemde “Elipsoit üstünde (jeodezik), GRS 1980 elipsoidi” (datumun elipsoidi);
    - yerel sistemde “Düzlemde: yerel sistemin koordinatlarıyla”;
    - sistemsiz çizimde “Düzlemde: çizimin kendi koordinatlarıyla”.
  - **Sistem:** projenin sistemi, örneğin “TUREF / TM36 (EPSG:5256)”.
- **Uzun yol:** satırlar bir yükseklikten sonra kayar.
- **Kopyala:** başlığı, her satırı (ad, sekme, değer), birimi, hesabı ve sistemi metin olarak panoya yazar; “Ölçü
  panoya kopyalandı.” der.
- **×:** kutuyu sonraki ölçüye kadar kapatır.
- **Değerler:** araçların kendi hesabıdır (`kentos_interaction::measured`); araçlar sonucu iletiyle söylemeyi sürdürür.
  Araç bağlamı (`Context.measured`) son ölçüyü taşır.

### 3. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Ölçü kutusu | var: Görünüm › Sorgu'nun ölçüleri | var: Ölçme › Sorgu | var: Ölçme › Sorgu | var |
| Ölçme › Hızlı ölçüm grubu | yok: Mimari'nin Ölçme sekmesi yoktur (ADR 0802), ölçüler Görünüm › Sorgu'dadır | yok: Ölçme'nin Sorgu grubu aynı dört komutu taşır | yok: aynı gerekçe | var |

### 4. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Kutu, etkileşimli araçların masaüstündeki sunumudur. Betik bir nesnenin uzunluğunu ve alanını `measure(uid, method)` ile ölçer. |
| MCP | yok | Aynı gerekçe. |
| Yapay zekâ ajanı | yok | Aynı gerekçe. |
| Eklenti | yok | Aynı gerekçe. |

## Açık

- **Hesap yolu seçimi:** Kartezyen ile elipsoidal arasında elle seçim yok; projenin sistemi belirler (ADR 1097).
- **Birim seçimi:** kutuda birim değiştirilmez; projenin ayarındaki birim kullanılır.
- **Canlı satırlar:** kutu ölçü bitince dolar; tıklarken imlecin yanındaki etiket ara değerleri söyler.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Çekirdek:** `cargo test -p kentos-interaction -- measured measure angle coordinate`: kitaplıkta 6, bütünleşik
  sınamalarda 95 geçti. `measured::tests`:
  - üçgende 30, 40 ve toplam 70 m, AB 50 m, alan 600 m² ve çevre 120 m;
  - dik açı 100 gon, birim adıyla; noktanın Y (doğu) ve X (kuzey) adları;
  - coğrafi projede ekvatorda 1″ elipsoit üstünde, bağımsız başvuruyla: 2π·6 378 137 / 1 296 000 = 30,922 m.
- **Masaüstü:** `measure_box_tests` (3 sınama), gerçek komutlarla ve yazılan noktalarla:
  - Mesafe ölç A, C, B: 30, 40, 70 m; A, B: 50 m;
  - Alan hesapla: 600 m², 120 m;
  - Açı ölç: 100,0000 g ve 300,0000 g; derece projede 90°;
  - Koordinat oku: Y (doğu), X (kuzey);
  - Kopyala'nın metni, ×, araç değişince kutunun gitmesi;
  - şeritte Hızlı ölçüm'ün dört komutu.
- **Kesişen sınamalar:**
  - `cargo test -p kentos-desktop --lib -- module_tests ribbon keytips catalog history::text settings measure query`:
    249 geçti; tek kırmızı işten önce de olan bağımlılık tabanı.
  - `cargo test -p kentos-cad`: 37 geçti. `cargo test -p kentos-desktop-cbs`: 58 geçti.
  - `python3 scripts/ui/ribbon.py --check` yeşil.
- **Resim** (`.run/gui-evidence/kullanim-9328/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-hizli-olcum-mesafe`: Ölçme › Hızlı ölçüm ve Mesafe ölçüsü kutusu (30, 40, 70 m);
  - `02-alan`: Alan ölçüsü (600.00 m², 120.000 m);
  - `03-aci`: Açı ölçüsü (100.0000 g, “gon (bir tur 400 g)”).
