# ADR 0150: Python editörü, REPL ve gerçek kod tamamlama

- **Durum:** kabul edildi (2026-10-01).
- **Tarih:** 2026-10-01
- **Bağlam:** Iced/wgpu bileşen kütüphanesinde JetBrains Mono, renkli Python
  editörü, REPL ve import bağlamını anlayan IntelliSense; UI showcase'te canlı örnek.

## Karar

`kentos-ui::widget::python` yerel Iced 0.14 bileşenlerini ve host'tan bağımsız
durum modellerini sağlar. Kod renklendiricisi artımlıdır; çok satırlı dizeler,
f-string, dekoratör ve tanımları işler. Yeni bağımlılık eklenmez. Gömülü JetBrains
Mono kullanılır; renkler temadan, köşeler mevcut biçim ayarından gelir.

Editörün girinti/geri alma/yinelemesi `EditorState`'tedir. REPL, yürütmeyi
uygulamaya döndürür; benzersiz istek kimliği geç çıktıların yeni çalıştırmaya
karışmasını engeller. Tamamlama da uygulamaya `CompletionRequest` olarak döner.
Kaynak, imleç ve sürüm değiştiğinde öneri sonucu uygulanmaz. Kabul edilen öneri
geçerli sözcük aralığını tek geri alma adımıyla değiştirir.

IntelliSense, imlece bağlı bir katmandır: ayrı semantik vektör ikonları,
tür etiketi, imza, kısa doküman, klavye/fare seçimi ve soluk metin önizlemesi.
Ctrl+Space açar, ↑↓ seçer, Tab/Enter tamamlar, Esc kapatır. Yalnız öneri gerektiğinde
showcase zamanlayıcısı çalışır; dekore edilmiş yüzeyin hareketi yerel karelerdedir.

## Showcase çalıştırıcısı

`python3 -u` ilk yürütme veya tamamlama sorgusunda ayrı süreçte başlar.
Editör ve REPL aynı namespace'i paylaşır. Betik çıktısı protokolden önce yakalanır
ve sınırlandırılır. Durdur süreci sonlandırır, çıkış süreci temizler. `input()`
bu örnekte açık hata döndürür. Masaüstünün SDK konsolu bu değişikliğin dışındadır.

Tamamlama sağlayıcısı standart Python kütüphanesini kullanır: AST, importlib,
pkgutil ve statik üye incelemesi. `import`, `from … import`, modül takma adları,
kurulu paketler, kaynak tanımları, literal türler, sınıflar, metotlar, parametreler
ve canlı REPL nesneleri çözümlenir. Property getter'ları ve kullanıcı ifadeleri
tamamlama için yürütülmez. Karmaşık dinamik dönüş türleri ve proje göreli
importları bu sağlayıcının kapsamı dışındadır; aynı API dil sunucusuna açıktır.

## Doğrulama

- Ekransız gerçek klavye/fare: Unicode girinti, F5, uzun satır ve dikey
  kaydırmada imleç, Tab/Enter/fareyle tamamlama ve tek adımda geri alma.
- Gerçek Python: kalıcı değişken/son ifade/`_`, eksik blok, hata, çıktı sınırı,
  sonsuz betiğin durdurulması ve yeni oturum.
- Gerçek tamamlama: modül/paket importları, from-import sınıfı, takma adın
  fonksiyonu, string metodu, isimli parametre, yerel sınıf ve tipli parametre.
  Canlı listede tamamlanan `append` yürütülerek sonucun değiştiği denetlenir.
  Property önerisi sorgulandıktan sonra getter sayacının sıfır kaldığı doğrulanır.

API/kısayollar: [`crates/ui/README.md`](../../crates/ui/README.md#python-editörü-ve-repl).
