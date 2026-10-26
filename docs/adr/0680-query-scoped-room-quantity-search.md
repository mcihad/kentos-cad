# 0680 — Metraj sorgusunda geçici ortak mahal araması

25–26 Ekim 2026. **Bu sorgu ve tablo yerleşimi dilimi kabul edildi.** ARCH-SPACE-01 / ARCH-PERF-01.

ADR0679'nin 200 yapı/400 mahal/20.000 ilgisiz çizgi denemesi, metrajın her
mahalın üç yöntemi için kaynak ağını baştan kurduğunu gösterdi. Her çağrıda
bütün belge taranması ve aynı sınırın tekrar kurulması sorgu gecikmesidir.

- Bir `cad.spaces.quantities` sorgusu tek immutable `SpaceSearch` kullanır.
  Kapsam/kat/açık sıralı UUID kümesine göre kaynaklar belge sırasında ve
  analitik kutularıyla bir kez alınır; yakın kutu (+2, sonra +1 metre) ve
  tam arama geri dönüşü korunur. Slot sırası belge sırası yerine geçmez.
- Seçilmiş kaynakların **tamamı**, yakındaki kutulara indirilmeden önce
  doğrulanır; uzaktaki kayıp/uygunsuz kaynak sessizce yok sayılamaz.
- Tam sıralı kaynak slot kümesi ve serbest oda politikası aynıysa üç yöntem
  aynı ağdan hesaplanır. Kolon düşümü aynı kaynak/kat/kapsam anahtarını korur.
  Sorgu ömrü biterken ağlar ve listeler bırakılır; kalıcı/global önbellek ve
  revizyon geçersizliği tasarlanmaz. Rust immutable belge ödüncü yazmayı engeller.
- Kolon tercihi/kapsam/UUID/ayırıcı kullanmayan eski mahaller önceki
  `Document::space_found` yolunda kalır. Dosya, birim, tolerans, alan hesabı,
  uyarı veya komut sözleşmesi bu optimizasyon için değiştirilmez.

Kabul: eski tam rapor çıktılarıyla yeni çıktılar aynı belgelerde
karşılaştırıldı; karışık kaynak anahtarları, beş birim, yakın/uzak konum,
üç yöntem, kayıp kaynak, farklı katlar, çizilmiş ve eski mahaller birlikte
sınandı. Bağımsız geometri durumları, readonly/undo, Python/MCP ve gerçek
Metraj penceresi doğrulandı. Debug süreleri release/GPU/UI bütçesi sayılmaz;
uçtan uca süre/bellek ve bütün mimari GUI matrisi ana planda açık kalır.


400 mahal/21.000 nesneli aynı debug belgede sıcak tam sorgu ortancası
8646,943→83,765 ms; yeni sürecin ilk sorgusu ayrıca 815,623 ms. Süreç soğukluğu
eşlenik değildir; release/p95/bellek/GPU kabulü değildir. 100 dondurulmuş tam
rapor eşit, bağımsız 105 ortak durum, 300 Python, 268 application/103 domain/6 MCP
başarılı; son UI için all-target clippy4 ve build4 başarılı.

Gerçek Metraj'da önce sağda boş alan varken kesilen sütun genişlik doldurmayla
düzeldi. İki yönlü Iced kaydırıcısı çubuk spacing'ini layout'ta kullanmadığından
sağ/alt içerik boşluğu çubuk eniyle birlikte verilir. Dar 1100 mantıksal en ve
18px yazıda sağ/alt uçlara fareyle gidilerek son rakam/satır doğrulandı. Başarısız
ara adaylar da 22 ham görüntü kaydında ayrıdır. CSV ve KCAD bağımsız okundu.
CSV bildirimindeki 600/gerçek400 satır ayrılığı ARCH-DOC-02 takip işidir.

[Ham log, donanım/derleme SHA, süre, gerçek GUI ve bağımsız dosya kabulü](../baseline/2026-10-25-architecture-quantity-cache/README.md).
