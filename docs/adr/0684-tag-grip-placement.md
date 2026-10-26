# 0684 — Mahal etiketinin konum tutamacı

26 Ekim 2026. **Etiket konum tutamacı kabul edildi.** ARCH-SPACE-02 [~].

Mac'te seçilmiş mahal etiketinin ortasından fareyle sürükleme önizleme
oluşturdu, bırakmada “Yapılacak değişiklik verilmedi” dedi. KCAD'nin tamamı
değişmeden kaldı. Sorun kaynak izleme veya çizim ölçeği değildir: ortak
geometri tutamacı Tag.at konumunu taşıyor, fakat genel EntityGeometry bilerek
Tag temsil etmiyor. Select'in genel geometri yazım döngüsü etiketi atlayıp
boş değişiklik komutu yolluyor. Eski kaynakta gerçek oturum testi başarısızdır.

Etiket tek bir konum tutamacına sahiptir. Bu tutamaç yalnız sıcak etiket
UUID'lerini mevcut `cad.entities.transform` Move komutuna verir; seçilmiş ama
sıcak olmayan oda/öteki etiketler taşınmaz. Önizleme aynı ortak tutamaç
geometrisidir. Kopya aynı komutun copy seçeneğini kullanır; yeni etiket aynı
kaynak nesneyi izler. Kalıcı model/şema, kâğıt yazı boyu, birimler, renderer
ve diğer tutamaç aileleri değiştirilmez. Python ve MCP bu mevcut komutu
kullanır; yeni komut veya alternatif taşıma hesabı eklenmez.

Etiket ile farklı türdeki sıcak tutamaçlar birlikte taşınmaz; hiçbir nesne
değişmeden açıklama verilir, kısmi önizleme gösterilmez. Beklerken katmanı
kilitlenen sıcak etiket varsa bütün bu etiket tutamaç yerleşimi reddedilir.
Normal genel Taşı komutunun kilitli nesne politikası aynı kalır.

Kabul: 20 bağımsız hedefe gerçek pointer_down/move/up, tam model/UUID,
tek undo/redo; yalnız sıcak etiket alt kümesi, çoklu sıcak etiket, kopya,
karışık tür, kilit ve Escape. Kamuya açık Python'da plan/yazma/kayıt-aç,
20 duvar kalınlığında analitik net alan ve kaynak silme/geri alma.
Son değişmez Mac paketinde gerçek etiket sürükleme ve ardından 40 cm duvar,
20.16 m² mahal/etiket/çizelge; kaynak mahal/duvar silme, şablon değiştirme,
tek undo/redo, dar açık/18 px önizleme/Cmd-S/sağ tuş Bırak/Escape ve yeni
süreçte açma doğrulandı. 22 ham JPEG, 18 bağımsız disk karşılaştırması;
692 interaction/219 application/303 Python/bir mevcut MCP testi, clippy/build.
Atlanan testler ve başvurudaki çalışmayan mpmath karşılaştırması kayıtta ayrıdır.
Eski etiketi Koru/Yenile aracı, zengin kot şablonları, kat/pafta ve bütün
tema/yazı/pencere matrisi açıktır; parent madde tamamlandı sayılmaz.
Kanıt [kabul kaydındadır](../baseline/2026-10-26-architecture-tag-follow/README.md).
