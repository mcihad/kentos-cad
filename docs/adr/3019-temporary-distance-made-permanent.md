# 3019 — Geçici ölçüden kalıcı ölçü

Tarih: 2026-11-08. Durum: kabul (M19, ADV-11; plan S11).

## Sorun

- **Geçici ölçü yarım kalıyordu.** Duvar seçilince iki yandaki net mesafe görünüyor (ADR 0710), sayıya yazılan değer duvarı
  kaydırıyor, kilit "bu ölçüyü koru" bağını kuruyor (ADR 0770 §3).
- **Kalıcı ölçüye çevirme yoktu.** ADR 0728: "Kalıcı ölçüye çevirme ve kilitleme (Revit'in kilidi) yok". Kilit sonra geldi,
  çevirme gelmedi. Mimar aynı mesafeyi paftada göstermek için Ölçülendirme ya da İç ölçü zinciri ile iki yüzü yeniden
  tıklıyordu.

## Araştırma (8 Kasım 2026)

- **Revit:** geçici ölçünün altındaki simge ("Make this temporary dimension permanent") onu kalıcı ölçü yapar; ölçü
  başvurularına bağlı kalır, eleman kayınca değeri değişir. ADV-11: "Tek tıkla kalıcı ölçü ya da 'Bu ölçüyü koru' bağı olur"
  (W2 R6; Chief Architect KB-01116).
- **KentOS'ta hazır yol:** İç ölçü zinciri (ADR 0711) kesen çizgiyle yüzleri bulur. Her ölçünün iki ucu kestiği yüze bağlıdır
  (`along` yeri, "kesen çizgi" kuralı, KCAD şema 118). Duvar kayınca değer güncellenir.

## Karar

- **Sayının kilide göre öbür yanında bir +** (`Switch::Permanent`, `wall_distance.rs`):
  - Yalnız duvar yüzüne olan net mesafede vardır. Aksa olan mesafede yoktur, çünkü aks kesen çizginin başvurusu değil.
  - Vurgu renginde çizilir. Kilit gibi 12 px içinde tıklanır (`temporary_refs::square_at`).
- **Tıklama** (`wall_distance::make_permanent`):
  - Yazan komut `cad.dimensions.interior`'dır. Kesen çizgi duvarın yüzünün 5 mm içinden komşu yüzün 5 mm içine çizilir; 5 mm
    KentOS'un en ince duvarının yarısından azdır.
  - Böylece tam iki yüz kesilir ve tek bir ölçü çıkar, zincir çıkmaz. İki ucu yüzüne bağlıdır: duvarlardan biri kayınca değer
    değişir.
  - Ölçünün çizgisi geçici ölçünün yerindedir. Tek geri alma adımıdır.
  - Katmanı Mimari'nin ölçü katmanıdır (MM-XXXX-OLCU; yoksa ilk ölçüyle açılır, ADR 3008). Katman anahtarı kapalıysa etkin
    katmana yazılır.
  - Seçim değişmez: duvar seçili kalır, öbür mesafe de çevrilebilir.
- **İleti:** "Kalıcı ölçü yazıldı (4.343 m); iki yüze bağlı: duvarlar kayınca değeri güncellenir."
- **Sayıya yazılan değer duvarı kaydırır** (ADR 0710) ve **kilit bağ kurar** (ADR 0770); bu ADR onları değiştirmez.

## Ölçüm

- **Test** (`building_context_tests::the_plus_beside_a_wall_s_distance_writes_a_permanent_dimension_that_follows_the_walls`):
  - Aksları 4 m arayla iki Dış duvar 25 duvar. Birincisi seçilince + çizilir; tıklanınca tek ölçü yazılır: 3,750 m,
    mm-xxxx-olcu katmanında.
  - İkinci duvar 0,5 m uzaklaşınca aynı ölçü 4,250 m der.
- **Gerçek pencere** (`fixtures/interaction/real/kalici-olcu.txt`, 1440×900, sahibin ayarlarının kopyası):
  - Dikdörtgen duvarla oda ve Duvar ile bölme çizildi. Bölme seçilince sayıların altında + göründü; soldaki tıklandı.
  - Kalıcı ölçü 4,343. Bölmenin sağ mesafesine 5 yazılıp Enter'a basıldı, bölme 0,920 m sola kaydı; kalıcı ölçü 5,263 oldu.
  - Resimler: `.run/gui-evidence/mimari-m19/once/`, `sonra/`, `m19-once-sonra.png`.

## Sonraki adımlar

- Açıklığın geçici ölçüsü (ADR 0701) ve kolonunki: iç ölçü zinciri açıklık kenarını ve aksı başvuru almıyor (ADR 0711'in
  "alınmayanları"). Onlar için başvuru eklenince aynı + gelir.
- Duvarın boyu (ADR 0728 §3) ve aksa olan mesafe için kalıcı ölçü.
