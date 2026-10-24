# 0630 — Mimari şablonu farkını görerek uygulama

24 Ekim 2026 · ARCH-UX-03 · Durum: ARCH-UX-03 kabulü tamam.

## Kaynakta bulunan davranış

ADR 0346'nın `cad.templates.apply` v1 çağrısı boş belgeyi paket birimine
(cm), her belgeyi 1:50'ye geçiriyor. `arch.template` doğrudan yazıyor.
Bu, ADR 0626'nın m/1:100 başlangıcında beklenmeyen sonuç veriyor. Eski
sözleşme ve bağımsız fixture'lar sessizce yeniden tanımlanmayacak.

## Uygulama kararı

- `cad.templates.apply` girdisine isteğe bağlı `settingsMode` ekle:
  `keep` birim ve çizim ölçeğini korur; `template` paket ayarını açıkça
  uygular. Alan yoksa ADR 0346 v1 davranışı sürer; eski otomasyon bozulmaz.
  GUI alanı her zaman gönderir ve `keep` ile açılır. Çıktı bu iki açık kipte
  önce/sonra ayarlarını ve korunacak parçaları içeren `review` döndürür.
- `keep` ile ayrıca `unit` gönderilmesi çelişkidir, atomik reddedilir.
  Açık `template` kipinde kullanıcı birimi gösterim ayarıdır; modeldeki SI
  koordinatları yeniden ölçeklenmez. Var olan mimari başlangıç değişmez.
- Koordinat sistemli CAD metreyle çalışır; açık paket kipinde başka birim
  `unit_needs_local` ile yazmasız reddedilir. GUI ofis seçeneği bu projede m/1:50
  gösterir ve `unit=m` gönderir. Eski dosyadaki gereksiz gösterim birimi, çözülmüş
  koordinat birimini önizlemede değiştirmez. Yerel projede cm/1:50 alternatifi sürer.
- Eklenecek katman/stil/blok/duvar stili/kat/pafta adlarını native `plan`
  belirler. Aynı kimlik/ad nedeniyle korunacak mevcut parçalar gösterilir;
  özellikle kat ve duvar stili çakışması sessizce yeni tanımla değiştirilmez.
- Önizleme hiçbir belgeyi yazmaz. Yeni kiplerde bütün paket tek açık grup
  içindedir: katmanlar, stiller, bloklar, ayarlar ve paftalar tek undo/redo
  adımıdır. Bir parça reddedilirse tüm grup geri alınır. Eski çağrıların
  tarihsel undo sınırı korunur.
- GUI penceresi farkı ve eski ofis paketinin cm/1:50 olduğunu gösterir;
  birim/ölçek koruma varsayılandır. İptal hiçbir şey yazmaz. Uygula, planın
  belge kimliği ve revision'ına bağlı girdiyi kullanır; proje değişmişse
  yeniden incelenecek farkı gösterir, eski onayla başka belgeye yazmaz.
- Python tipleri, katalog, MCP ve headless aynı iki açık kipi sunar. GUI'nin
  çağrısı Python çıktısında bu kip ve revision ile yeniden üretilebilir.
  Web arayüzü ADR 0228 gereği ertelenir; ortak sözleşme üretilir.

## Uygulama ve kabul

Genel proje tercihleri undo'ya alınmaz. `Op::TemplateSettings` yalnız paketin
etkilediği ayar alanlarını taşır; değişmeyen birim/ölçek ve ilgisiz tercihler
korunur. `set_sheets` mevcut açık grubun `Op::Sheets` kaydını kullanır.
Standart katmanın `plot` bayrağı ekleme kaydına taşındı: ilk uygulama aynı,
redo da artık aynı yazdırma durumunu üretir.

34 bağımsız yeni durum + değiştirilmemiş 23 eski şablon durumu; beş native
hedef; tam domain/native/headless/MCP/Python ve masaüstü regresyonları geçti.
Gerçek GUI'de yerel 48 m² odada önizleme/iptal/uygula/tekrar/undo/redo ve
kaydet/aç; projeksiyonlu odada m korunarak 1:100→1:50→undo 1:100 görüldü.
19 gerçek ekran, 16 GPU matrisi, son test sayıları, derleme kimlikleri ve
sınırlar [kabul kaydındadır](../baseline/2026-10-24-architecture-template/README.md).
