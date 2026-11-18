# ADR 8092: Bodruma inen asansör — korunmuş koridor ya da yangın güvenlik holü (BYKHY 62/4)

- **Durum:** uygulandı (17 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-13; ADR
  [8087](8087-shafts-elevators-roofs.md) §6 ("62/4'ün öbür cümleleri: … bodruma inen asansörün korunmuş koridor ya da
  yangın güvenlik holünden ulaşılması").
- **Bağlam:** ADR [8087](8087-shafts-elevators-roofs.md) (asansör kapısı, geçit, 62/4'ün ilk cümlesi),
  [8081](8081-fire-compartments.md) (ayrımlar), [8082](8082-fire-resistance-of-elements.md) (Ek-3/C).
- **Dayanak:** Binaların Yangından Korunması Hakkında Yönetmelik 4/dd (korunumlu koridor veya hol), 36/1-a, 62/4, Ek-3/C.
- **Web:** dokunulmadı. **`.kcad`:** değişmedi. **Sözleşme:** değişmedi (olgular yeterli).

## §1. Araştırma (17 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| BYKHY 62/4 (ikinci cümle) | Aynı anda bodrum katlara da hizmet veren asansörlere, bodrum katlarda korunmuş bir koridordan veya bir yangın güvenlik holünden ulaşılması gerekir | Bodrum katındaki asansör kapısının önü yangın güvenlik holü (adı) ya da korunmuş koridor olmalı |
| BYKHY 4/dd | Korunumlu koridor veya hol: bitişik olduğu mekânlardan yangına karşı dayanıklı yapı elemanlarıyla ayrılarak yangın etkilerinden korunmuş koridor veya hol | Koridorun bitişik mahallerle ayrımları: açık yer yok, her duvar ve açıklık EI |
| BYKHY 36/1-a | Kaçış yolu olan korunumlu koridor ve hollerin yangına dayanım süreleri Ek-3/B ve Ek-3/C'deki sürelere uygun olmalı | Sürenin kaynağı Ek-3/C'nin bodrum katları sütunu (binanın sınıfı, yağmurlaması, bodrum derinliği); okunmuyorsa eksik veri |
| BYKHY 4/tt (ADR 8081) | Kompartımanı ayıran eleman en az 60 dakika | Korunmuş koridor için 60 değil Ek-3/C'nin süresi kullanılır (36/1-a doğrudan ona gönderir) |

## §2. Karar

- **K124 — Bodrumda asansörün önü:** bodrum katı (zemin katın altındaki kat) üzerindeki, bir yanı asansör kuyusu, öbür
  yanı geçit olan kapı: yanı yangın güvenlik holüyse tutar; değilse geçidin ayrımları okunur.
- **K125 — Korunmuş koridor:** geçidin bitişik mahallerle bütün ayrımları açık yersiz ve her elemanı en az Ek-3/C'nin
  bodrum süresince E ve I ile (`yangin_sinifi.ayirir`). Açık yer ya da az süre kaldı, beyansız eleman eksik veri; binanın
  sınıfı okunmuyorsa ya da tablo bodruma süre vermiyorsa eksik veri. Önü geçit olmayan kapı 62/4'ün ilk cümlesinin
  bulgusudur (ikinci bulgu yazılmaz).
- **K126 — Satır:** iki cümle 62/4'ün tek satırında; kod `bykhy_elevator_basement_lobby`. Ek-3/C'nin süresi
  `dayanim.bodrum_suresi`'nden (tek tablo, tek okuma).

## §3. Uygulama

- **Paket** (`tr.kentos.mimari-mevzuat` 0.16.0): `kuyular.bodrum_holu`, `_bodrum_katlari`; `onu` başvuru gününü alır;
  `dayanim.bodrum_suresi`.

## §4. Sınamalar

- **Paket** (`fixtures/standards/v1/kuyular.json`, 7 yeni, 31 durum, metinden elle): güvenlik holü geçer; EI 60 duvarlı
  koridor geçer (apartman, bodrum 10 m'den az: 60); EI 30 kalır; 1,20 m açık yer kalır; beyansız duvar eksik veri;
  kullanımı okunmayan binada süre okunmaz, eksik veri; zemin kattaki asansör ikinci cümleye girmez.
- **Masaüstü** (`shaft_tests::a_lift_serving_the_basement_is_reached_through_a_protected_corridor`, dört daireli binanın
  bodrumu): koridoru depodan ayıran EI 30 duvarla 62/4 kalır, EI 60 ile geçer. **Resimler:**
  `.run/gui-evidence/bodrum-asansoru-8092/`.

## §5. Arayüz

- Yeni pencere yok. "Bodrum kat: asansör kapısı korunmamış koridora açılıyor (“Koridor”, ayrımı en az EI 60 değil); 62/4
  …".

## §6. Sıradaki adımlar

- **62/4'ün ilk cümlesi:** kuyunun havalandırma bacası (en az 0,1 m², kuyu alanının 0,025'i) ya da basınçlandırma
  ([ADR 8135](8135-lift-shaft-vent.md)'te yapıldı: kuyunun mahalindeki beyandan).
- **Korunmuş koridorun kapıları (36/1-b-1):** kaçış merdiveni kapısına eşdeğer dayanım ve kendiliğinden kapanma (47/3'ün
  okuması) koridorun çıkış kapılarına da.
