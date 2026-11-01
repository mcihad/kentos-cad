# 0819 — Kart imleci, sabit ölçü hedefi ve tarama önizlemesi

Tarih: 2026-11-01. Durum: uygulandı.

## Sorun ve karar

- **CBS bilgi kartı:** sahne işletim sistemi imlecini gizliyor; kartın boş
  yüzeyi etkileşim bildirmeyince alttaki `Hidden` değeri kazanıyordu. Yalnız
  gerçek bilgi kartının yüzeyi `Idle` bildirir. Buton/metin alanlarının
  kendi etkileşimleri korunur; pasif sahne etiketleri fareyi engellemez.
- **CAD ölçü yazısı:** özel çizim imleci metin maskesinin altında çiziliyordu.
  İmleç, önizlemenin metin/maskelerinden sonra ve gerçek giriş/kartlardan
  önce ayrı katmanda çizilir. Kartın yakaladığı fare sahneye sızmaz.
- **Tarama köşesi:** nesne kopyalama değil, her fare hareketinde eski bağlı
  nesne önizlemelerinin birikmesiydi. Tutamaç güncellemesi `followers`
  önizlemesini sıfırlar; yeni tarama dolgusu da önizlemeye dahil edilir.
  Belge yalnızca tutamaç onaylandığında değişir.
- **Duvar ölçüsüne erişim:** kenara en yakın nokta her harekette yeniden
  seçilince odanın ortasındaki rakam kayboluyordu. Görünen rakamın üzerinde
  veya kenar orta noktası ile rakam arasındaki 12 mantıksal piksel genişlik
  toleransındaki yaklaşım yolunda mevcut kenar korunur. Bu alan dışında
  diğer kenarlar eskisi gibi seçilir; uzunluk/mesafe editörleri değişmez.

## Marka ve Hakkında

`assets/images` altındaki üç özgün PNG ortak kaynaktır. Native tarafında
bir kez çözülüp önbelleğe alınır; logoların saydam dış boşluğu yalnız
gösterim dokusunda kırpılır. Kaynak dosyalar değiştirilmez. Açık/koyu tema
gerçek tema değerinden seçilir. Başlatıcı, başlangıç, ribon, uygulama
menüsü ve Hakkında ortak marka bileşenlerini kullanır. Web aynı dosyaları
paketler; dar ribonda kare simge, geniş ribonda tam logo gösterilir.

Native pencere simgesi de verilen PNG'dir. macOS Dock/Finder simgesinin
bundle kaynağından gelmesi için `scripts/desktop/bundle_macos.py` yeni bir
`.app` içine ICNS ve `CFBundleIconFile` üretir. Mevcut çıktının üzerine
yazmaz. Örnek: `python3 scripts/desktop/bundle_macos.py target/release/kentos-cad /tmp/KentOS.app --module mimari`.

Hakkında modülün kendi adını, logosunu, sürümü, platformu, açık çizimi,
koordinat sistemini ve bağlantı durumunu gösterir. Genel yetenekler ile
Python/MCP kullanımına kısa açıklama eklenir; geliştirici port sayacı
kullanıcı metninden çıkarılır.

## Doğrulama

- Yeni tarama regresyonu düzeltme öncesinde 3 yerine 203 önizleme çizgisi
  ile başarısızdı. Düzeltmeyle 200 hareket, sabit önizleme, değişmeyen
  belge, tek onay ve tek Undo kontrolü geçer.
- Yeni duvar regresyonu düzeltme öncesinde hedef kenarı yaklaşma sırasında
  değiştiriyordu. Düzeltmeyle rakama kadar yaklaşma ve editör açma geçer.
- Etkileşim: 843 geçti, 2 atlandı. Masaüstü: 1561 geçti, 256 atlandı.
  Başlatıcı: 56 geçti. Ortak UI (`--features snapshot`): 322 geçti.
  UI testleri snapshot özelliği gerektirir; bu özellik olmadan mevcut
  context-menu testlerinin importları derlenmez.
- Masaüstü ve başlatıcı derlemesi; ilgili dört pakette Clippy `-D warnings`
  geçti. Web üretim derlemesi geçti. Genel web TypeScript kontrolünde
  değiştirilmemiş dosyalarda 43 hata var; tam web tür kontrolü başarılı
  olarak raporlanmaz.
- Gerçek macOS GUI: duvar mesafesini 9.800'den 9.500'e yazıp değiştirme ve
  Undo; tarama köşesinde tekrarlanan hareket ve onaydan sonra üç nesnenin
  korunması; CBS Bilgi kartında imleç ve kapatma; native/web açık-koyu
  Hakkında ve başlatıcı logosu kontrol edildi.

Ekran görüntüleri ve loglar Git dışında
`/tmp/kentos-ui-fixes-2026-11-01/` altında. `01–04` ölçü/tarama,
`05–06` native temalar, `07` CBS kartı, `08–09` web temaları,
`10` son native derleme Hakkında, `11` başlatıcı, `12` Finder'da üç
uygulama paketinin gerçek simgesi kanıtıdır.
