# 0648 — Kolon, merdiven ve rampa özelliklerinde uzunluk ekleri

25 Ekim 2026. **Dar dilim uygulandı/doğrulandı.** ADR0647 sonrasında gerçek kolon En
hücresinde `40 cm` → `40 mm` kusuru yeniden üretildi; eski dosya/görüntü
korundu, tek Undo ile geri dönüldü. Kolon düşümü follower'ı yanlış ölçüyü doğru
hesapladığından mahal da yanlış büyüyordu; düzeltme okuma katmanındadır.

Kapsam: yalnız CAD `ArchField` Width/Depth/RiserHeight/Tread/Landing/Rise/Base
fiziksel uzunlukları mevcut ortak okuyucuyla çözülür. Birimsiz değer çizim
biriminde, açık ek fiziksel birimdedir. Opsiyonel boşaltma ve gerekli alanın
boş ret davranışı korunur. Balon çapı/etiket boyu kağıt mm, dönüş derece,
rıht sayısı adet, eğim yüzde; metin ve CBS legacy anlamı korunur. Model komutu,
follower, şema, API, render ve ölçek değiştirilmez.

- [x] Gerçek eski kusur ve kırmızı App regresyonu.
- [x] Beş birimde metrik/imperial, boyutsuz/kağıt değer korunumu, boşaltma,
  geçersiz ek, alan sınırı ve tek Undo.
- [x] Son derlemeyle gerçek kolon/follower etkileşimi, ret/Escape/undo/redo,
  kaydet ve bağımsız model/alan hesabı.
- [x] Özellikler testleri, derleme/clippy ve kanıt kaydı.

[Kabul](../baseline/2026-10-25-architecture-element-units/README.md):
41 özellik testi başarılı/4 atlanan; derleme/clippy başarılı. 10 bağımsız GUI
kaydı, Decimal kolon deliği/mahal hesabı. 20 ham görüntü içinde önceki pencereye
bağlı 04–08 ve odak dışı 10 denemesi kabul sayılmadı; doğru pencereyle tekrar
yapıldı. Merdiven/rampa alanlarının bu dilimdeki kontrolü App düzeyindedir.

Bu dilim tam merdiven/rampa tasarımcısı veya tüm mimari GUI matrisi kabulü değildir.
