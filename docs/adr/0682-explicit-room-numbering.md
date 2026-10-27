# 0682 — Kimliği koruyan açık mahal numaralandırma

26 Ekim 2026. **Numaralandırma dilimi kabul edildi.** ARCH-SPACE-02 [~].

Mahal numarası UUID değildir. Yeni `cad.spaces.number` mevcut mahallerin
yalnız `number` alanını, etiket/çizelge takipçileriyle tek adımda değiştirir.
Yeni dosya alanı, geometri, birim veya render değişikliği gerekmez.

- Girdi açık UUID listesi ve tam ilk numaradır (`Z-001`). ASCII son rakam
  dizisi mevcut keyfî uzunluklu Artır yordamıyla artar; sayıya/f64'e çevrilmez.
  1–10000 farklı mahal, numara başına 256 karakter sınırı vardır.
- `order=input` listesi tıklama sırasıdır. `direction` açık iki SI nokta
  ister; mevcut mahal etiket ankrajlarının yön üzerine izdüşümünü, eşitlikte
  sol normal izdüşümünü, sonra UUID'yi sıralar. Yönü ters vermek farklı ankrajların sırasını
  tersine çevirir; eş ankrajda UUID son kararı sabit kalır. Satır/sütun veya en yakın komşu rotası tahmin edilmez.
- Seçim dışındaki mahalın numarası, gizli veya başka katta olsa da, ayrılmıştır.
  Tam metin çakışması bütün yazmayı reddeder; otomatik atlama yoktur. Seçimdeki
  eski yinelenen numaralar tek işlemde düzeltilebilir. Açıklık numaraları ayrı
  alandır. Kilitli kaynak, yinelenen UUID ve eski revision atomik rettir.
- Plan yazmaz. Araç görünür ayar ve numara önizlemesini aynı komut planından
  alır. Tıklama/önseçim ve iki noktayla yön verme, geri alma, Escape, tek
  Enter/sağ tık onayı ve yeniden açma gerçek arayüzde doğrulandı; kapsam
  [kabul kaydında](../baseline/2026-10-26-architecture-space-number/README.md).
- Katalog, bağımsız 68 ortak durum, Python SDK ve gerçek MCP, manifest ve
  kayıt/tekrar aynı komutu kullanır. Web aracı ADR0228 nedeniyle ertelenir.

Bu dilim bütün ARCH-SPACE-02 kabulü değildir: boş numara denetimi, kat/ad/kullanım
listeleri, etiket şablonları ve farklı pafta oranları ayrıca doğrulanacaktır.

## Birincil kaynak karşılaştırması

- [Revit — ardışık mahal numarası](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-DocumentPresent/files/GUID-3F821258-0E67-4483-90F5-34059E56E476.htm):
  sıra mahalın `Number` özelliğinden gelir; kata göre ilk numarayı kullanıcı verir.
  KentOS'un tam ilk numara ve kalıcı UUID ayrımı bu ilkeyle uyumludur. Bu
  dilimde alfabe dizisi yoktur; ASCII sayısal son ek açık sözleşmedir.
- [Revit — mahal/etiket/çizelge ilişkisi](https://help.autodesk.com/cloudhelp/2020/ENU/Revit-Model/files/GUID-AA99D45A-D6BD-4230-836C-9324582F851F.htm):
  özellik değişikliği mahal, etiket ve çizelgede paylaşılır. KentOS bu bağları
  numaradan ayrı UUID ile korur; önizleme yalnız komutun sonucunu gösterir.
- [AutoCAD Architecture — mahal stilleri](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Architecture/files/GUID-84D8FD5B-9DC2-46DA-9F8C-620805271E76.htm):
  adların liste tanımından seçilebilmesini ve kullanım odaklı stilleri önerir.
  Kat/ad/kullanım listeleri ARCH-SPACE-02'nin açık sonraki kısmıdır; bu
  numaralandırma dilimi bu listeleri bitmiş saymaz.

Kaynaklar 26 Ekim 2026'da kontrol edildi. Yön izdüşümü, tüm çizimde tam numara
çakışmasının reddi ve tek onay KentOS'un açık ürün kararlarıdır; evrensel
mimari numara standardı veya diğer uygulamalarla aynı çakışma politikası iddiası yoktur.

## Gerçek pencerede yakalanan seçim açığı

İlk paketli GUI'de mahalın boş iç kısmına üç tıklama seçimi artırmadı; yalnız
sınır tıklaması çalıştı (kabul klasörü 03/04). İlk bench testleri oda yazısının
üzerine tıklamıştı ve bu açığı maskelemişti. Bu görüntüler tanısal kayıttır.

Düzeltme yalnız yeni numaralandırma aracındadır: olağan kenar/yazı hit'i önce
korunur; yoksa görünür `Store::near` adaylarının kök mahalı ortak analitik
`inside_area` hesabıyla sınanır. Delikler dışarıda, en küçük alan önce,
eşit alanlarda belge sırası kullanılır. Etkin katman/Otomatik seçim ve tür
süzgeci iki yolda da aynıdır. Genel seçim, katmanın iç seçim tercihi ve
render çekirdeği değiştirilmez.

Yeni regresyon, 8×8 odanın yazıdan/kenardan uzak (2,2) noktasında olağan
hit'in bulunmadığını önce doğrular; Mahal numarala içten seçer, araçtan
çıkınca olağan Seç aynı noktada kenar politikasını korur. Delik, gizlilik,
etkin katman, tür filtresi ve konsol N isteminden görünür alana geçiş
ayrıca sınanır. Son araç takımı altı testtir; doğrulanmış ikinci GUI paketi
ayrı ikili/hash ile kaydedilir.

## Son görsel kabul

Büyük arayüz yazısı ve uzak sahnede eski→yeni önizlemesi komşu odalara
değmiştir (33/35, tanısal kayıt). Son paket yalnız atanacak `→ V-099`
numarasını gösterir; kaynak numara sahnede kalır. Son altı etkileşim testi,
clippy/derleme, koyu/açık 18 px geniş/dar gerçek fare testi ve tek Enter /
Diğer→Uygula / Escape / undo ile bağımsız KCAD kontrolü geçmiştir (43–52).
Genel etiket yerleştirme, kamera, ölçek ve renderer davranışı değişmemiştir.

## 27 Ekim 2026 eki: önerilen numaranın okunuşu

Daire yürüyüşünde (ADR 0344'ün 27 Ekim eki) dar odanın önerilen numarası (“→ Z-001”) etiket noktasının sağına
yazıldığından bölme duvarına taşıyor, duvarın çizgisi yazının içinden geçiyordu. Karar: önerilen numara etiketin üstünde
ortalanır (yazının yaklaşık genişliğiyle) ve önizlemenin bütün kısa yazıları (`Preview::labels`) haritanın işaretleri gibi
zeminin rengindeki haleyle çizilir: altından geçen çizgi yazıyı kesmez. Komut ve önizlemenin planı değişmedi. Doğrulama:
mahal numaralama testleri (7) ve resim `KENTOS_SHOTS_ONLY=daire-numara`.
