# 1093 — Nokta bulutu alan ve profil pencerelerinde bulut, sahneden seçim ve birim

Tarih: 2026-11-20. gis-usage-report.md GIS-036. Durum: kabul edildi.

İlgili: [ADR 0552](0552-point-cloud-areas-meshes-and-virtual-clouds.md) (pencereler), [ADR 8006](8006-editors-ask-and-pick-from-drawing.md)
(Çizimden seç), CLAUDE.md §5 (birim, noktalı ondalık).

## Sorun

Seçim boşken iki pencere “pencereyi kapatın, bulutla birlikte Shift ile seçin, sekmeden yeniden açın” diyordu; oysa
komut bulut seçimi istemez (`uids` boş: çizimin bütün bulutları) ve geometri seçimi yeterliydi. Kullanıcı hangi
bulutun okunacağını görmüyordu. Metre tabanlı projede özet “Alan 1088” ve “6.807 nokta/birim²” yazıyordu: kopyalanan
metinde birim yoktu.

## Karar

1. **Bulut satırı.** Pencere açılırken çizimin bulutları sayılır: hiç yoksa uyarı (“Çizimde nokta bulutu yok: önce
   Ekle › Nokta bulutu ile…”) ve Çalıştır kapalı; bir taneyse adı (etiketi, yoksa ilk dosyasının adı) ve nokta sayısı
   yazılır, o okunur; birkaç taneyse seçim kutusu: önce “Tüm bulutlar (n)”, sonra her biri. Seçilen bulut komuta
   `uids` olarak gider; Tüm bulutlar boş `uids`'dir (komutun eski davranışı).
2. **Sahneden seç.** Alan/Yol satırının altında “Sahneden seç” (ve eski “Seçimden al”): pencere Çizimden seç
   düzeniyle (`pick_for_window`) çekilir, alan ya da çizgi çizimde tıklanır, Enter pencereyi aynı değerlerle (koridor
   genişliği, adım, seçili bulut) geri getirir ve şekil seçimden yeniden okunur (`area_after_pick`). İleti bunu söyler:
   “Seçimde alan yok: Sahneden seç ile çizimde alanı … tıklayıp Enter'a basın; pencere aynı ayarlarla döner.”
3. **Geçersiz alan.** Üçten az köşeli ya da alanı sıfır halka komuta gitmeden “Seçilen alan geçersiz: en az üç ayrı
   köşesi ve sıfırdan büyük alanı olmalı” ile reddedilir.
4. **Birim.** Alan projenin alan birimiyle (`Format::area`: m², dönüm, ha; yerel projede mm²…), yoğunluk nokta/m²
   olarak (çizim metre) yazılır; birden küçük yoğunluk üç anlamlı basamakla (0.00237), büyüğü üç ondalıkla. Profilin
   uzunluğu, istasyon sınırları ve yükseklikleri `Format::length` ile birimli; giriş alanları “m” der. Sayımlar binlik
   noktalı (2.822). Ondalık ayıracı nokta kalır (CLAUDE.md §5): raporun “6,807” önerisi bu kurala göre “6.807”dir.

## Sınır

- Coğrafi (derece) sistemdeki projede alanın birimi GIS-040'ta ele alınır; burada çizim birimi metre varsayılır, çünkü
  komutun alanı çizimin düzlem birimidir.
- “Yoğunluk: en az … en çok …” satırı LAS yansıma şiddetidir (intensity); adı ayrı işte gözden geçirilebilir.

## Kanıt

`apps/desktop/src/shell/exchange/area_cloud_tests.rs`: tek bulut adıyla alınır, birkaç bulutta biri seçilince komut
o kimlikle aynı cevabı verir; bulutsuz çizimde Çalıştır bir şey yapmaz; Sahneden seç ile alan ve yol alınır, genişlik
korunur; düz halka reddedilir; cevaplar m², nokta/m² ve m taşır; resimler `.run/gui-evidence/cbs-kullanim/gis-036-*`.
