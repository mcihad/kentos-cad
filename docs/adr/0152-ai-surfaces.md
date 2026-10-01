# ADR 0152: Yerel AI Surface bileşenleri

- **Durum:** kabul edildi (2026-10-01).
- **Tarih:** 2026-10-01
- **Bağlam:** Iced/wgpu bileşen kütüphanesinde konuşma, prompt, karakter akışı,
  düşünme özeti, kullanıcı soruları, araç ilerlemesi ve onayları; showcase'te canlı örnek.

## Karar

`kentos-ui::widget::ai`, sağlayıcıdan bağımsız yerel Iced bileşenleri ve durum
modelleri sunar. `AiConversation`, `AiMessage`, `AiPrompt`, `AiContent`,
`AiStream`, `AiActivity`, `AiThinking`, `AiQuestion` ve `AiToolCall` birlikte
veya ayrı kullanılabilir. Yeni bağımlılık eklenmez.

Uygulama `AiAction` ile model isteği, durdurma, kaynak açma, pano, dosya eki,
soru yanıtı, araç onayı ve geri bildirim işlerini üstlenir. Bileşen kütüphanesi
modeli çağırmaz veya aracı çalıştırmaz. Çizim belgesine bağlanan host mevcut
komut/yetki/transaction sözleşmelerini kullanmalıdır.

## Akış ve etkileşim

Yanıt parçaları, düşünme özetleri, araçlar, sorular ve bitiş/hata aynı istek
kimliğini taşır. İptal edilen veya tamamlanan isteklerin geç parçaları reddedilir.
Hata/durdurma kısmi yanıtı ve sonraki prompt taslağını korur. Bekleyen sorular,
araç onayları ve çalışan araçlar bitişi engeller. Önerilen seçenek seçilmeden
ve açıkça gönderilmeden yanıt sayılmaz. Araç çalıştırma onayı açık kullanıcı
eylemidir; host gerçek yürütme sonucunu ayrıca bildirir.

Enter gönderir, Shift+Enter yeni satır açar. Çalışırken sonraki prompt
yazılabilir. Tekli/çoklu seçenek, açıklama, serbest yanıt ve isteğe bağlı
atlama desteklenir. Konuşma 100 mesaj, yanıt 512 KiB, araçlar 32 ve sorular
16 ile sınırlıdır. Sınırlar UTF-8 karakterlerini bölmez.

Düşünme alanı sağlayıcının gösterilebilir özeti veya gözlemlenen işlem
ilerlemesidir. `AiContent` başlık, vurgu, bağlantı, liste, alıntı, tablo ve
kod bloklarını çizer; Python bloklarında gömülü JetBrains Mono ve Python
renklendiricisi kullanılır. Desteklenmeyen Markdown metin olarak görünür.

## Hareket ve titreme düzeltmeleri

Parçacık yörüngesi, ışıklı karakter imleci, odak geçişi, yüzey ışık izi ve
240 ms açılır kartlar Iced'in yerel kareleriyle çizilir. Görünürlük, pencere
odağı ve `motion::reduced()` sürekli kare isteklerini sınırlar. Şekiller,
gradyanlar ve gölgeler wgpu çizicisi üzerinden gider.

Akış ve tamamlanmış metin aynı rich-text çocuğunu, tip ölçeğini ve satır
genişliğini korur. Yeni paragraf veya yanıt bitişinde widget değiştirilmez.
Karakterler ekrana geldikçe konuşma görünümü alta yapışır. Yalnız gerçek
kaydırma hareketi izlemeyi durdurur; Iced'in yeniden yerleşim bildirimleri
kullanıcı hareketi olarak işlenmez. Kullanıcı geçmişi okurken konumu sabittir.
Kart dolguları ve tablo satırları biçim ayarının yarıçapını kullanır; yüzey
ışık izi yuvarlak köşelerin düz kenar aralığında kalır.

## Showcase ve doğrulama

Galeri → AI Surface yerel, belirlenmiş örnek veri üretir. Metin akışı, soru,
araç onayı ve hata senaryoları; durdurma, yeniden deneme, ekler ve geri
bildirim etkileşimlidir. Uzak model sağlayıcısı bağlı değildir. Örnek dosya
eki ve kaynaklar showcase verisidir.

Gerçek klavye/fare testleri: dar promptta çok satırlı gönderim ve durdurma,
sonraki taslağın korunması, açık soru yanıtı, büyüyen içerikte alta izleme ve
geçmiş okuma konumunun korunması. Yerel kare testleri UTF-8 ilerlemesini,
akış bitişinde aynı paragraf yerleşimini, görünürlükte kare istemeyi ve açılır
kartın ara/final yüksekliğini denetler. İstek kimliği, iptal, hata, onay ve
yanıt bekleme kuralları durum modeli testlerindedir.

Ekransız wgpu görüntüleri:

```sh
KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-ui-showcase ai::tests::screens -- --ignored --nocapture --test-threads=1
```

API ve kullanım: [`crates/ui/README.md`](../../crates/ui/README.md#ai-surface).
