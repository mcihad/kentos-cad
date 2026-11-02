# 0838 — Saklanan geometrinin salt okunur yuvarlama doğrulaması

2026-11-02. CAD-ADV-05 / ADR 0837. Durum: çekirdek, komut, geniş regresyon, Python/MCP ve gerçek Mac arayüz kabulü geçti.

## Sorun ve kaynak incelemesi

487000 / 4420000 m koordinatında, sabit uçlu dairesel path kenarına
teğet ve 2.5 m uzunluk bağlı çizgi, yarıçap 3 m yapıldığında doğru yerel
çözümü veriyordu. Dünya koordinatına dönüp f64 olarak kaydedilen çizginin
ölçülen uzunluğu 2.5000000002222236 m oluyordu. Aynı belgenin salt okunur
sorgusu bu temsil farkını uzunluk ve teğet çelişkisi sanıyordu. İlk sorgu
başarılıyken kaydedilmiş dosyanın kırmızıya dönmesi kabul edilemez.

[Autodesk'in büyük koordinat açıklaması](https://www.autodesk.com/blogs/autocad/working-large-coordinates-in-autocad/)
dünya koordinatının f64 anlamlı basamaklarını tükettiğini ve ayrı ofsetle
küçük sayılarda hesaplamanın önemini açıklar. KentOS çözücüsü zaten yerel
başlangıç kullanır; eksik olan saklama sonrası doğrulamadır.
[OCCT Precision belgesi](https://occt3d.com/dev/doc/refman/html/class_precision.html)
hesaplama hassasiyetini uygulama geometrisi toleransından ayırır.
Aşağıdaki algoritma KentOS'un kendi çözümüdür; bu kaynaklardan bir
algoritma ya da sayısal tolerans kopyalanmadı.

## Karar ve kesin sınırlar

- Mevcut normal çözüm ve artık toleransı korunur. ULP ile artık toleransı
  büyütülmez, ekranda yuvarlanan değerle eşitlik ilan edilmez.
- Yalnız bütün bilinmeyenleri tutulmuş bir doğrulamada, kopuk başvuru
  yokken ve başarısız bağıntıların tamamı uygun doğrusal olmayan türlerden
  oluşurken ikinci, özel bir doğrulama denenir. Nokta–nokta Çakışık,
  Yatay/Düşey ve Fix ihlalleri bu yoldan kabul edilmez.
- Her kaynak dünya koordinatının yarım ULP'si mevcut `MOVED=1e-9 m`
  çözünürlüğünü aşmamalıdır. Örneğin 1e15 düzeyindeki kaba temsil bu
  doğrulamaya giremez. Bu sınır artık denklemine eklenen bir tolerans değildir.
- Özel doğrulama bütün yarıçap/eğrilik/açı gibi koordinat olmayan
  bilinmeyenleri kaynakta dondurur. Kalıcı Fix ve diğer denklemler korunur;
  geçici bütün-nesne tutuşları kaldırılarak yalnız koordinatlar çözülür.
- Normal katı yerel çözücü başarılı olmalı; tekrar dünya koordinatına
  çevrilen **her koordinatın bitleri kaynakla aynı** olmalıdır. Ayrıca
  raporlanacak hareket eşiğinin altında kalanlar dahil **her model yuvası**
  yeniden yazılarak oluşan şekil kaynak şeklin tamamına eşit olmalıdır.
  Atlanan küçük yazma bu kanıt yerine geçmez.
- Bu özel tur kendini çağırmaz, takipçi kanonikleştirmesi yapmaz ve sonucu
  belgeye yazmaz. Ancak katı yerel çözüm tam olarak aynı saklama temsiline
  dönüyorsa ilk salt okunur doğrulamadaki uygun hatalar kaldırılır.
  Çözücü iterasyon sınırı ve durdurma isteği iki turda da geçerlidir.
- Query belgeyi onarmaz, dosyayı normalize etmez, kimlik/öznitelik/bulge
  değiştirmez ve Undo adımı üretmez. Geçersiz yarıçap veya kilitli nesneye
  değişiklik isteği bu yolla başarılı hale getirilemez.

## Kod ve kabul

`crates/shared/geometry-core/src/ops/constraints/solve.rs::run_checked`
özel doğrulamayı ve durdurmayı taşır. `arc_edge_tests.rs` kaynak temsile
dönüşü sınar; 1e-7 m gerçek bozma, orijine yakın 1 nm yanlış uzaklık,
1e15 koordinat ve nokta–nokta bir ULP açıklık reddedilir. Durdurma kaynak
şekli korur ve Cancelled döndürür.

`native/application/tests/all/arc_edge_constraints.rs` büyük koordinatta
kalıcı bağları kurar; yarıçap değişimi, salt okunur query, kaydet/aç ve
tek Undo kabulünü yürütür. Python aynı dosya ve komutlarla bağımsız çağrı
yolunu sınar. Gerçek arayüzde kurulan altı bağ, 2.5→3 m değişimi, dosyanın gerçek uygulamada yeniden açılması ve tek Undo kabulü ADR 0837 içinde kayıtlıdır. UI/Python geometri farkı 0 ölçüldü.

Bu karar bütün f64 koordinatlarda sınırsız doğruluk iddiası değildir.
Belirtilen çözünürlükte, katı yerel çözümü değişmeden temsil eden mevcut
saklama biçimi için sınırlı bir doğrulama sağlar.
