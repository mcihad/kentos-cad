# ADR 0810: Katları tekrarla, hedefte aynısı olan modülsüz nesneyi yeniden kopyalamaz

- **Durum:** kabul edildi (2026-11-01).
- **İş:** plan/architecture-advanced.md §3 `ADV-05`'in “Açık” satırındaki modülsüz kopyaların yinelenme denetimi; ADR
  0796 §2'nin üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** değişmez. **Sözleşme:** `ModulesOperation::Repeat.again`, `ModuleReport.matched`, uyarı
  `module_repeat_matched`.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0796 §2 Katları tekrarla'ya `loose`'u getirdi: kaynak katın modülsüz duvar, açıklık, mahal, donatı, ölçü ve
etiketleri her hedefe düz kopya olarak gider. Modül örnekleri ikinci kez yerleşemez (`module_repeat_taken`), ama düz
kopyalar her tekrarda yeniden yapılıyordu: aynı tekrarı iki kez çalıştıran kullanıcı katı iki kat dolduruyordu, üst üste
duvarlar birleşmez, mahaller ikişer sayılır, Metraj ikiye katlanır. ADR 0796 bunu açık bıraktı.

Revit'in Paste Aligned to Selected Levels'ı da aynı yerdeki eşi denetlemez; yapıştırma sonrası “Identical instances in
the same place” uyarısı verir, kullanıcı elle siler. Archicad'in Hotlink'i kopyalamaz, bağlar; modülsüz nesne için
karşılığı yoktur. KentOS uyarıdan önce yazmamayı seçer: eşi olan kopya hiç yapılmaz, plan bunu yazmadan söyler.

## Karar

1. **Aynısı.** Kaynak katın modülsüz bir nesnesi, hedef katta aynı türden, hiçbir modül örneğinde ve tasarım
   seçeneğinde olmayan, yapacağı kopyaya (hedef kata yazılmış, numarası hedefin kuralıyla) yuvası ve belgenin kendi
   yazdıkları (açıklığın pervazları, kaynağa bağlı mahalin halkaları, etiketin yazısı ve bağ noktası, kopukluğun nedeni;
   ADR 0779 §3'ün `same_object`'i, sayılar 1e-6 içinde) dışında eşit bir nesne varsa kopyalanmaz. Kopyanın bağları
   aynısı bulunan kaynakların aynısına gider (kapının kopyası, aynısı olan duvara), bu yüzden aynısı, yeni bulunan
   kalmayana dek yeniden aranır. Hedefteki her nesne en çok bir kopyanın aynısıdır, çizimin sırasıyla ilki.
2. **Cevap.** Kopyalanmayanlar `report.matched`'ta (kaynak, hedefteki aynısı, kat), uyarı `module_repeat_matched`
   sayısıyla. `again` hepsini yine kopyalar (ADR 0796'nın davranışı). Hiçbir örnek yerleşmez ve hiçbir nesne
   kopyalanmazsa tekrar `module_repeat_taken` ile reddedilir (yol `operation.to`).
3. **Masaüstü.** Oda modülleri'nin Katları tekrarla'sında Modülsüz nesneler de açıkken **Hedefte aynısı olanı da
   kopyala**; plan “n nesnenin aynısı hedef katta zaten var; yeniden kopyalanmaz” der, yazınca durum satırı da.
   Pencerenin gövdesi kayar: 1100×650'de eylem düğmeleri pencerenin dışına düşüyordu.

## Doğrulama

- Bağımsız başvuru KentOS kodu olmadan: `scripts/fixtures/module_repeat_matched_cases.py` (module_storeys_cases.py'nin
  çizimi ve kat kuralıyla) `cad.modules.edit.matched.json`'a 23 durum yazar: ikinci tekrarın reddi ve planı, `again`,
  silinen ya da taşınan kopyanın (donatı, duvar, kapı, mahal, etiket, ölçü) yeniden yapılması ve bağlarının var olan
  aynılara gitmesi, iki hedeften birinin dolu olması, numara önekinin değişmesi, 1. katta elle çizilmiş aynı duvarlar
  (zeminin duvar kopyaları onlarla eşleşir, kapının kopyası onlara konur), taşınan ve tasarım seçeneğindeki duvar, geri
  alma, yazmayan plan. Masaüstünde ortak durumlar (bir beklentiyi bozarak koştuğu da sınandı), yetenek testi,
  Python'un ortak durumları; ADR 0775, 0779, 0796'nın modül durumları değişmeden geçer.
- Masaüstü: `module_storey_scenes::tests::a_second_repeat_copies_only_what_the_target_lacks`; resimler (iki tema,
  1440×900 ve 1100×650) `KENTOS_SHOTS_ONLY=dubleks-yinelenen cargo test -p kentos-desktop --lib tools_screens::screens --
  --ignored --nocapture`.
- Gerçek pencerede elle denenmedi.

## Sonuçlar

- Aynısı tam eşitliktir: bir milimetre kaydırılmış ya da adı değiştirilmiş kopya aynısı sayılmaz ve yeniden yapılır.
- Açık: tek katın yerleşmesi (Single Story; ADR 0811 kapattı), seçenekler arası eşleme ve birleştirme.
- Web'e gerekenler (TODOS.md §0.2): `again`, `report.matched` ve penceredeki seçenek.
