# 0638 — Oda konturlarının ortak duvar ağı

25 Ekim 2026 · ARCH-DRAW-03 · ADR 0637'nin devamı; kabul çalışması sürüyor.

## Geometrik karar

Mevcut `cad.walls.convert(mode=contours)` Eksen referansında, aynı hedef
katmandaki zıt oda taraflarının ortak düz/yaylı sınırını bir kez üretir.
Kısmi örtüşme kenarı analitik olarak böler. Kaynağın delikleri de aynı ağa
katılır. Birleşmeyen halkalar mevcut kapalı duvar olarak birebir korunur;
etkilenen sınırlar normal açık/kapalı duvar yollarına dönüşür. Birleşim
düğümlerinde yollar ayrılır; mevcut duvar birleşim çözümü T/köşe/çapraz
sunumunu yapar. Yeni KCAD modeli, dosya sürümü veya komut yoktur.

Aynı geometrik sınır iki odanın birden İç net/Dış ölçü yüzü olamaz. Bu
referanslarla ortak kenar atomik reddedilir ve Eksen seçimi istenir.
Asimetrik bileşenli bir duvarı iki taraftan aynı yönde uygulama isteği de
reddedilir. Aynı yanda yinelenen sınır, iç içe çakışan kenar, gerçek kesişme,
yalnız noktada temas veya belirsiz analitik eğri tek duvar diye onarılmaz.

Hesap `geometry-core::geom::wall_network` içindedir. Mevcut arrangement
bölme ve düğüm birleştirmesi kullanılır; kaynak kesişme/eğri adayları mevcut
FP64 PackedTree ile daraltılır. Mevcut `arrangement::TOL=1e-6 m` sabitidir.
Kaynak noktalar mevcut temsilci kuralıyla korunur; paylaşımın analitik
çizgi/daire taşıyıcısı ayrıca aynı toleransla kontrol edilir. Tolerans
kullanıcıya önizleme ve komut uyarısında yazılır. Genel tolerans, render
çizgileştirmesi, import veya çizim ölçeği değiştirilmez.

Arrangement'in kaynak başına yoğun ownership tablosunun başlangıç hesabı
8 milyon f64 (64 MB'den az) ile sınırlandırılır; aşım atomik ret verir.
Bu bütün süreç belleğinin ölçülmüş üst sınırı değildir: sonradan kesilen
parçalar ve birleşim hesabı ayrıca bellek kullanır. Büyük ağ performans ve
bellek kabulü açık kalır; bu sınırla sonsuz ölçek iddia edilmez.

## Kimlik, veri ve görünüm

Her çıktı mevcut duvar nesnesidir; kapı/pencere host'u olabilir. Tekilleşen
parçanın öznitelikleri ve görünümü seçim sırasındaki ilk kaynaktandır.
Bu seçim, paylaşım uzunluğu, parça adedi ve eşleme toleransı
`wall_shared_boundary` uyarısıyla açıkça raporlanır. Diğer normal parçalar
kendi kaynaklarının alanlarını alır. Koru/Sil bütün kümede tek işlem ve
undo/redo'dur. Farklı hedef katmanlar birbirine birleştirilmez; semantik
kat/level entegrasyonu ayrı açık iştir.

Tekrar dönüşüm, önceki ağ açık duvarlara ayrılmış olsa da, aynı katmandaki
pozitif analitik örtüşmeyi tanır ve hiçbir kısmi duvar yazmaz. Sadece tek
nokta teması tekrar üretim sayılmaz. Önizleme ortak ağda sahnenin mevcut
duvar birleşim çözümünü kullanır; eski Each/Pairs ve tekil kontur önizlemesi
kendi mevcut davranışını korur. Python/MCP aynı plan/execute yolundadır.

## Bağımsız kabul

Önceki 69 convert vakası JSON olarak aynı önek; iki ölçülü 4×3 odanın bilinen
U / ortak bölme / U yollarından bağımsız 24 vaka eklenir. Birim, uzak
koordinat, ters yön ve kaynak Koru/Sil. Beklenen tekil toplam eksen uzunluğu
25 m; 0.2 m kalınlıkta birleşmiş gövde 4.96 m². İki T düğümünün solver
kaydında inline ve tee olayları ayrıdır; olay adedi düğüm adedi sanılmaz.

Uzak koordinatta gövde alanının depolama hata sınırı, 64 m'den kısa çevre
çarpı mutlak FP64 koordinat ULP'si ve 1e-12 m² yerel hesap payıdır. Bu yeni
sayısal doğrulama ölçütüdür; mevcut geometrik tolerans büyütülmez. Yaylı
ortak çeyrek dairede toplam uzunluk bağımsız `12+5π m` beklentisidir.
Açık yaylı duvarın son sıfır bulgesi normal KCAD kayıt düzenine eklenir.

Gerçek GUI, bağımsız kayıt kontrolü ve çalıştırılmış testler
[kabul kaydında](../baseline/2026-10-25-architecture-shared-wall-contours/README.md)
raporlanır. Tam boyut/font/tema matrisi, kat semantiği, karışık kaynak ve büyük
ağ kapıları kapanmadan ARCH-DRAW-03 tamamlandı işaretlenmez.
