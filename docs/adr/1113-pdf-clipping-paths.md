# 1113 — PDF okuyucusu kırpma yollarını uygular

Tarih: 2026-11-20. gis-usage-report.md GIS-053. Durum: kabul edildi.

İlgili: ADR 0352 (PDF içe aktarma), ISO 32000-1 §8.5.4 (kırpma yolu).

## Sorun

KentOS'un iki sayfalı park atlasının ikinci sayfası içe alınınca harita çerçevesinin kırptığı yürüyüş yolu çerçevenin
dışına (263 m batıya) uzandı: içerik akışında dört `W` vardı, okuyucunun `W` ve `W*` dalları boştu.

## Karar

1. Grafik durumu kırpma yollarını taşır: sayfanın görünen kutusu (crop box, döndürmeyle) ilk kırpmadır; `W` (sıfırdan
   farklı sarım) ve `W*` (tek-çift) bir sonraki boyama işlecinden (`n` dahil) sonra duruma eklenir, `q`/`Q` ile
   saklanıp geri gelir. Yeni kırpma öncekilerle kesişir (bir nokta hepsinin içindeyse içeride).
2. Boyanan her parça boyandığı andaki kırpmalarla kesilir (`pdf::clip::cut`): çizgiler içeride kalan parçalarına
   bölünür; dolgulu alanlar dışbükey tek halkalı kırpmada Sutherland–Hodgman ile, diğerlerinde düzlemin alan
   kesişimiyle (`region::intersect_areas`) tam kesilir. Kesilen parçanın Bézier bilgisi bırakılır (noktaları düzlük
   toleransında kalır), kesilmeyen parça onu korur. Başlangıç noktası kırpmanın dışındaki yazı alınmaz.
3. Kırpma yolunun kendisi de boyanıyorsa (çerçeve çizgisi) yeni kırpma onu kesmez, dıştakiler keser.

## Sınır

Yazı kırpma kipleri (`Tr` 4–7) kırpma olarak uygulanmaz; bu yazılar zaten alınmıyor (3 ve 7 görünmez). Sıfırdan farklı
sarımlı ve kendini kesen bir kırpma halkası dolgu kesişiminde alan bindirmesinin kuralıyla okunur.

## Kanıt

- `crates/shared/formats/src/pdf/clip.rs` testi: çerçeveyi kesen çizgi, iki pencere, tek-çift delik, iç içe kırpma,
  çerçeveyi örten dolgu (100 birim²), içeride ve dışarıda kalanlar.
- `crates/shared/formats/src/pdf/mod.rs` `a_map_frame_clips_what_is_drawn_past_it`: elle yazılmış PDF'te harita
  çerçevesi, `Q` sonrası tüm sayfa, iki çerçeve, `W*` deliği, iç içe kırpma, sayfa kutusu ve çerçeve dışındaki yazı.
- `kentos-formats` testleri (217) ve paylaşılan örnekler (bilinen 41 dışında) değişmeden geçer. Bu iş, `walls.rs`
  testindeki önceden kırık `ArchInfo` başlatıcısını da düzeltti.
