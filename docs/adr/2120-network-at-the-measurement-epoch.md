# 2120 — GNSS ağı ölçme epokunda: bilinen noktaların hız ızgarasıyla taşınması

Tarih: 2026-11-08. plan/map-cad.md MAP-03 (ve MAP-05), üçüncü tur. Durum: taslak. Plan satırı: “GNSS/RTK/PPK/PPP …
koordinat epoğu … işlem kaydına bağlansın.” İlgili:

- [ADR 1740](1740-gnss-network-from-archive.md): arşivden GNSS ağı. Bu ADR onun “Sonra”sındaki şu maddeyi kapatır:
  “bilinen noktaların hız ızgarasıyla ölçme epokuna kendiliğinden taşınması; farklı epoklu bilinen noktalar”;
- [ADR 0631](0631-geoid-velocity-and-vertical-statements.md) §2: hız ızgarası ve epok taşıma (`cad.crs.transform`);
- [ADR 1340](1340-gnss-campaign-archive.md): arazi ölçümünün epok kararı ve hız ızgarası.

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460 Mükerrer; ADR 1740'ta okunan metin), **Madde 15(1)(a):** TUTGA/TUSAGA-Aktif ve eski
  C1 koordinatları ölçme epokuna taşınır. 15(1)(ç)'ye göre ağ, bu koordinatlar değişmez alınarak dengelenir.
- **TUREF'in epoku** (ADR 1230 §Araştırma): TUREF, ITRF96'nın 2005,0 epokundaki gerçekleşmesidir; TUSAGA-Aktif'in
  yayımlanan koordinatları da o epoktadır. Türkiye'de yıllık 2–3 cm'lik levha hareketiyle 2005,0'dan 2026'ya fark yarım
  metreyi bulur. Fay kuşaklarında noktalar birbirine göre de kayar; ölçme epokunda tutarsız bilinen noktalar ağı
  bozar.
- **PROJ `deformation`** (ADR 1230): hızlar mm/yıl (doğu, kuzey, yukarı), Dünya merkezli koordinatta uygulanır. KentOS'un
  zinciri aynı adımı kendi motoruyla yürütür (ADR 0631 §2); bağımsız başvuru PROJ'un kendisidir.
- **GVX 1.0** (ADR 1740): vektörün `OBSERVATION_TIME` `START`/`END`'i yalnız GPS zamanıdır.
- **KentOS bugün:**
  - bilinen noktalar tek çerçeve ve epokta olmak zorundaydı (`known_differ`);
  - o epok ağın epoku sayılıyordu;
  - ölçme epokuna taşımak kullanıcının işiydi (ADR 1740 §3).

  2005,0'da girilen TUSAGA-Aktif koordinatları ve 2026'da ölçülen bazlar birlikte dengeleniyordu.

## Karar

1. **Ölçme epoku** `map.survey.network`'te şöyle belirlenir:
   - girdi `epoch` verildiyse odur (1900–2100, `invalid_epoch`);
   - değilse, dengelemeye giren bazların hepsinin başlangıç ve bitiş zamanı varsa, her bazın orta zamanının
     ortalamasıdır;
   - ikisi de yoksa ölçme epoku yoktur.

   Zaman ondalık yıla şöyle çevrilir: yıl + o yılın başından geçen saniye / yılın saniyesi (artık yıl 366 gün). GPS ile
   UTC arasındaki 18 s (5,7·10⁻⁷ yıl) yok sayılır: 3 cm/yıl hızda 2·10⁻⁸ mm eder.
2. **Bilinen noktalar ölçme epokuna taşınır** (Madde 15(1)(a)).
   - Ölçme epoku varsa her bilinen nokta epokunu söyler (yoksa `known_differ`) ve hepsi tek çerçevededir.
   - Epoku ölçme epokundan farklı olan bilinen nokta, arazi ölçümünün hız ızgarasıyla oraya taşınır. Taşımayı
     koordinat zincirinin kendisi yapar: aynı çerçeve, elipsoit yükseklikleri, iki epok (`cad.crs.transform`).
   - Hız ızgarası yoksa ağ reddedilir (`epoch_unsupported`). İleti bilinen noktayı, iki epoku ve maddeyi söyler, iki
     yol gösterir: ızgarayı verin ya da noktayı ölçme epokunda girin.
   - Farklı epoklu bilinen noktalar artık bir arada olabilir: hepsi aynı epoka taşınır.
3. **Ağın epoku ölçme epokudur.**
   - Cevapta ayrıca `epochFrom` (`given`, `baselines`, `known`) yazar.
   - Taşınan her bilinen nokta için `moved`: epoku, kaç yıl taşındığı ve verilen yerinde doğu, kuzey, yukarı kayması.
   - Yeni noktalar ölçme epokundan arazi ölçümünün epok kararına aynı ızgarayla döner (ADR 1340 §3'ün zinciri).
   - Kabul (`network`) dengeleme kaydına ölçme epokunu yazar; kabul işlemi de `epoch` alır.
   - Aynı ağın yeniden kabulü epoku da karşılaştırır.
4. **Ölçme epoku yoksa davranış eskisidir:** bilinen noktalar tek epokta olur, o epok ağınkidir; ileti artık “ya da
   bazların gözlem zamanlarını verin” der. Şema değişmez: hız ızgarası arazi ölçümünde, epok dengeleme kaydında zaten
   vardı.
5. **Masaüstü** (Arşivden ağ dengelemesi, yalnız Harita):
   - **Ölçme epoku** kutusu: boşsa bazlardan alınır;
   - özetin ilk satırı epoku, nereden geldiğini, kaç bilinen noktanın ne kadar taşındığını ve maddeyi söyler;
   - Bilinen noktalar tablosunda **Taşınma D/K/Y (mm)** sütunu.
6. **Kapsam işareti:** **Harita + CBS** (`map.survey.network` ikisinde salt okunur; pencere ve kabul Harita'nın).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_survey_network_cases.py`. Ölçme epoku ve ondalık yıl ADR'den elle
  yazılır; taşıma PROJ'un deformation adımıyla, ızgara `fixtures/geoid/v1/hiz.tif`. Yeni durumlar:
  - bazların zamanından ölçme epoku (2 Kasım 2026); iki bilinen nokta 21,84 yıl taşınır (≈ 543 mm yatay, 43 mm düşey);
  - verilen epok (2026,5);
  - farklı epoklu bilinen noktalar: BN2'nin 2026,84 koordinatı PROJ'la taşınmış; ağ uyuşur;
  - ızgarasız arazi ölçümünde ret, epoksuz bilinen nokta, aralık dışı epok;
  - ölçme epokunda kabul.

  `map.survey.network` 36, `map.survey.edit`'e 34 ağ durumu. Rust ve Python geçer; yetenek listesi denetimi geçer.
- **Bulunan:** `.kcad` okuyucusu bazın zamanını `Z`'li an olarak ister. Python koşucusu durumu dosyadan geçirdiği için
  bunu yakaladı; durumların zamanları öyle yazıldı.
- **MCP** (`crates/native/mcp/tests/survey_epoch.rs`): ızgara cihaz kütüphanesinden yüklenir, durumun çizimi `.kcad`
  olarak açılır. Ölçme epoku, kaynağı, taşınan noktalar ve kaymaları durumunkilerle 10⁻⁶ içinde aynıdır.
- **Masaüstü:**
  - `survey_epoch_tests.rs`: bazlardan epok, yazılan epok, sayı olmayan kutu, kabulde kayıt epoku ve noktaların karara
    dönmesi;
  - `survey_network_window_tests.rs`: GVX'li arşivde 2005,0'daki istasyon için madde iletisi; ızgara verilince kabul
    edilebilir.

  Resimler `.run/gui-evidence/ag-epoku-2120/`.

## Sonra

- Bilinen noktanın kendi hızı (TUSAGA-Aktif istasyonlarının yayımlanan hızları), ızgarasız.
- Oturum başına ölçme epoku: uzun kampanyada her oturumun kendi epokunda bazları.
- GVX oturum çapraz korelasyonları; arşiv panelinde bazlar; 15(1)(c) benzerlik dönüşümü.
- Resmî TUREF hız alanı (sahibin kaynağı bekleniyor; ADR 1230).
