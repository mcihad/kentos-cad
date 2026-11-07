# 1820 — Pafta eş yükselti eğrileri: BÖHHBÜY m. 78(1)(c)–(g) ve Ek-3 (`road.terrains.sheetcontours`)

Tarih: 2026-11-07. plan/map-cad.md MAP-14, ikinci dilim. Durum: taslak. İlgili:
- [ADR 1520](1520-map-project-terrain-surfaces.md): Harita projesinde arazi yüzeyi. Bu ADR oradaki “Eğrinin detay
  kesmemesi (m. 78(1)(g)), yardımcı eğri (e), kalın eğride yükseklik yazısı (f)” maddesini açar.
- ADR 0900 §13: `road.terrains.contours` (yüzeyin eğrileri; kurallarına dokunulmadı).

## Dayanak (7 Kasım 2026'da okundu)

- **BÖHHBÜY 2018** (RG 26.06.2018/30460 Mük.). Metin, Çanakkale Onsekiz Mart Üniversitesi'nin yayımladığı PDF'ten
  okundu ([PDF](https://cdn.comu.edu.tr/cms/muhendislik.harita/files/458-51-buyuk-olcekli-harita-ve-harita-bilgileri-uretim.pdf)).
  Ek-3'ün işaret sayfaları resimdir; 88. sayfa (resmî sayfa 7048) yüksek çözünürlükte okundu.
- **m. 78(1) “Pafta çizimi”:**
  - (c) aralık 1/5000'de 5 m, 1/2000'de 2 m, 1/1000 ve 1/500'de 1 m;
  - (ç) en yakın noktaların yükseklikleri esas alınır;
  - (d) eğriler 0,13 mm, her beşinci 0,25 mm;
  - (e) eğri arası 2 mm'den azsa yalnız kalın eğriler, 20 mm'den fazlaysa araya kesik çizgiyle bir yardımcı eğri;
  - (f) kalın eğrilere her 200 mm'de bir bırakılan boşluğa, artan eğim doğrultusunda, eğrinin yüksekliği yazılır;
  - (g) eğriler yol, nehir, kanal, ark gibi çift çizgili detayları, şev sınırlarını, bina ve benzeri kapalı detayları
    kesmez.
- **Ek-3, sınıf 0000:**
  - 0001 ana eğri 0,25 mm; 0002 ara eğri 0,13 mm;
  - 0003 yardımcı eğri 0,13 mm, 5 mm çizgi / 1 mm boşluk;
  - 0005 eğri kotu 1,4 mm, çizgiye 1 mm boşluk, eğriye paralel.
- **Plan (MAP-14):** yumuşatma yalnız sunum türevi olsun; görsel gizleme kararı analizden veri çıkarma kararıyla
  karıştırılmasın.

## Karar

1. **Kural paketi veridir:** `catalog/v1/rules/esyukselti-bohhbuy-2018.json` (`bohhbuy-2018-m78`). Paket şunları
   taşır:
   - ölçeğe göre aralık, her kaçıncı eğrinin kalın olduğu;
   - üç kalınlık;
   - 2 ve 20 mm eşikleri;
   - 200 mm yazı aralığı ve 1,4 mm yazı boyu.

   Komut başka bir paket de alır. Kodda sabit yoktur.
2. **`road.terrains.sheetcontours` v1** (Harita + Yol, yazar): yüzeyin eğrileri 3B çoklu çizgi, kotlar yazı olarak tek
   geri alma adımı “Pafta eş yükselti eğrileri”. Ölçek verilmezse çizim ölçeği, aralık verilmezse paketin o ölçek için
   aralığı kullanılır.
3. **(e) üçgen başına:**
   - Bir üçgenin paftadaki eğri arası, aralık bölü üçgenin eğimidir (kâğıt mm, ölçekte).
   - `steepBelow`'un altındaki üçgen yalnız kalın eğri verir.
   - `auxiliaryAbove`'un üstündeki üçgen yarım aralıkta yardımcı eğri verir.
   - Çekirdeğin seviye izlemesine üçgen süzgeci eklendi (`Surface::level_lines`). Eğri kural değişen yerde kesilir.
     Bütün üçgenler alınınca sonuç `road.terrains.contours`'unkiyle bit bit aynıdır; 23 ortak durumu aynen geçer.
4. **(g) detaylar:**
   - Detay, detay katmanlarındaki kapalı çoklu çizginin ya da alanın halkalarıdır. Yay kenarı kirişiyle okunur.
   - Çift çizgili yol ve dere alan olarak çizilmiş olmalıdır; iki ayrı kenar çizgisinin arası bu dilimde bilinmez.
   - Eğrinin parçası detay kenarını kestiği yerde bölünür. Ortası bir detayın içinde kalan (halkalarda tek-çift) parça
     çizilmez; uzunluğu `cut`'a eklenir. Kesilen kapalı eğri açılır.
   - Eğriler sunum türevidir; yüzey ve hesap kotu değişmez.
5. **(f) yazı:**
   - Her kalın parçada, başından itibaren her `labelEvery` mm'de bir yazı konur; ilki yarım aralıkta.
   - Yazı bulunduğu doğru parçası boyunca döner, üstü yokuşa (parçanın ortasındaki üçgenin düşme yönünün tersine)
     bakar.
   - Kutusu maskelidir: m. 78(1)(f)'nin boşluğu maskeyle verilir, eğri geometrisi bölünmez (plan: görsel gizleme veri
     çıkarmaz).
   - Ek-3 0005'in 1 mm kenar boşluğu maskenin kendi kenarıdır; ayrıca ölçülmez.
6. **Yazılanlar:**
   - Kalın, ara ve yardımcı eğriler paketin kalınlıklarıyla yazılır. Yardımcı eğri yerleşik kesik çizgi tipiyle
     çizilir; Ek-3 0003'ün 5/1 mm deseni bu dilimde proje çizgi tipi olarak tanımlanmadı.
   - Yazılar 1,4 mm × ölçek yüksekliğinde, ortadan hizalı, maskelidir.
   - Cevap eğrileri (tür, kot, kapalı mı, yerler), yazıları, dik ve yardımcılı üçgen sayılarını, kesilen uzunluğu ve
     notları verir.
7. **Retler:**
   - `not_a_road_project`;
   - `invalid_contours`: ölçek 0, aralık yok ya da 0'dan büyük değil, taban sonlu değil, paket eksik, 10 000'den çok
     düzey;
   - `layer_not_found`, `not_a_layer`, `layer_locked`; detay katmanı yoksa `layer_not_found`;
   - `invalid_revision`, `revision_conflict`, `unknown_terrain`, `terrain_unbuildable`, `no_contours`.

   Gizli katmana yazıda `layer_hidden` uyarısı verilir.
8. **Masaüstü: Pafta eğrileri…** (`terrain.sheetContours`):
   - **Simge:** `sheetContours`, kesik yardımcı, yazı boşluklu kalın ve ince ara eğri. Seçenek sayfası
     `.run/gui-evidence/pafta-egrileri-1820/secenekler.png`.
   - **Yer:** Arazi › Yüzey, Buluttan'ın yanı.
   - **Form:** yüzey ve katman; ölçek ve aralık (boşsa paketin aralığı, ipucunda m. 78(1)(c)); (e) ve (f) için onay
     kutuları; (g) için detay katmanları onay kutusuyla. Katman adlarından bina, yol, şev tahmin edilir.
   - **Özet ve yazma:** özet planın sayılarıdır; Çiz tek adımda yazar.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/road_sheet_contour_cases.py` (uv ile `triangle`), 27 durum; KentOS
  kodu yok.
  - Yüzey Triangle'ındır: 20 m'lik ızgarada 5 m'ye kadar kaydırılmış 336 nokta. Düzlükte %2 eğim, 15 m'lik sırt
    (4 m'lik lojistik) ve 30 m'lik tepe vardır. Bina ve yol alan, şev sınırı kapalı çoklu çizgi olarak çizilir.
  - Eğri arası kuralı (eğim kesirle), seviye izleme, kırpma ve yazılar elle hesaplanır.
  - Durumlar:
    - 1/1000 ilk değerlerle: 18 kalın, 101 ara, 21 yardımcı eğri, 15 yazı; 17 dik ve 209 yardımcılı üçgen; detaylarda
      1216,23 m kesilir;
    - plan ve doğrulama yazmaz; detaysız;
    - 1/2000, 1/500; kurallar kapalı; elle 0,5 m; taban 0,5; 1/1500'de elle aralık; başka paket;
    - gizli katman uyarısı;
    - 15 ret.

  Rust ve Python geçer. `road.terrains.contours`'un 23 durumu çekirdek değişikliğinden sonra aynen geçer.
- **Masaüstü** (`sheet_contours_window_tests`, 2 test):
  - pencere katmanı ve detayları tahmin eder, planı ortak durumla aynıdır;
  - yardımcı eğri kapatılınca plandan çıkar;
  - Çiz eğri ve yazıları tek adımda yazar (geri alınır);
  - Harita sunar.

  Resimler `.run/gui-evidence/pafta-egrileri-1820/` altındadır. Ortak masaüstü denetimleri 137 testle geçer.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("harita.kcad")
r = cad.road.terrains.sheetcontours(doc, surface="Mevcut zemin", layer_id="esyukselti", scale=1000,
                                     details=["bina", "yol"])
print(len(r.lines), len(r.labels), r.cut)
```

MCP: `{"tool": "road.terrains.sheetcontours", ...}`.

## Sonra

- Yardımcı eğrinin Ek-3 0003 deseninin (5/1 mm) proje çizgi tipi olarak tanımlanması.
- Çift çizgili detayın iki ayrı kenar çizgisinden tanınması.
- Yazının yazı tipiyle ölçülen genişliğine göre eğride gerçek boşluk (bugün maske).
- Eğrilerin `road.project.rebuild`'ın tuttuğu çizimler arasına girmesi (yüzey değişince yeniden çizim).
- m. 78(1)(ğ) karakteristik nokta kotları.
