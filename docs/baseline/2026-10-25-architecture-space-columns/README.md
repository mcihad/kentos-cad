# Kalıcı kolon düşümü — işlevsel dilim kabulü

25 Ekim 2026; dal `codex/architecture-2d-workflows`, başlangıç `fcd4b4f0c`.
ADR 0643 kolon düşümü dilimi gerçek GUI ve bağımsız dosya hesabıyla doğrulandı;
aynı gerçek GUI denemesinde bulunan GPU konum kusuru ADR 0644'te izlenir.

Kaynak çizim önceki SDK ile hazırlanan altı gerçek nesnedir; elle çizildiği
iddia edilmez. Kaynak duvar 10 × 8 m eksen, 0,2 m kalınlık; Zemin kat S1
0,6 × 0,8 m dikdörtgen, S2 1 m çaplı daire; başka kattaki 2 × 2 m kolon ve
S1'i tutan iki aks. Dünya koordinatı 500000 / 4400000 m, dosya origin'i sıfır.
Dosya birimi mm, çizim ölçeği 1:100 bilerek korunur. Önizleme mm², mevcut
kalıcı mahal etiketi m² gösterir; bu değişiklik birim/ölçek davranışını değiştirmez.

Gerçek fare/klavye denemelerinin ilk kaydı:

- 01–03: önceki sürüm; Net/Projenin 76.440.000,01 mm² (76,44 m²), kolonlar düşülmez.
- 04: güncel ilk build'de altı kaynak ve yeni mahal yok.
- 05–06: görünür Kolon alanı seçimi, Tüm katlar 71.174.601,84 mm²;
  Zemin kat 75.174.601,84 mm². Kat kapsamı doğru, fakat daire deliğinin
  önizleme dolgusu eksik ve çizgi/sahne konumu ayrışıyor; bunlar başarı sayılmaz.
- 07–09: bir tıklama ile mahal ekleme, kaydetme, tek undo; altı kaynak aynıdır.
  08, kamera yaklaşık sabitken de kalan konum farkını gösterir.
- 10–12: kalıcı mahalın Öznitelikler'deki tercihi. Düşme → 76,44 m²,
  tek undo → Yalnız net / 75,17 m². Kalıcı kimlik ve kaynaklar korunur.
- 13–19: ikinci build; daire deliği boş, gerçek kolon ve önizleme hizalı.
  Yakınlaştırma, geri uzaklaştırma, Kaydır ile gerçek sol tuş sürüklemesi ve
  yeniden önizleme sınandı. Escape/kaydet sonrası dosya başlangıcın tamamıyla
  aynıdır; altı kaynak, birim, ölçek, origin ve proje ayarları değişmez.
- 20–24: Zemin kat / Yalnız net toplu önizleme; bir Enter ile tek mahal.
  Tekrar Tüm katlar'da da 0 yeni / 1 mevcut; boş onay geçmiş eklemez.
  Bir undo altı kaynak, bir redo aynı mahal UUID'sini getirir.
- 25–30: Belgele menüsünden gerçek etiket ve mahal çizelgesi yerleştirildi.
  A aksı Taşı / yer değiştirme ile −2000 mm kaydırıldı. S1 aynı komutta
  aksına taşındı; kısmen duvar dışına çıktığı için mahal/etiket/çizelge
  75,17 → 75,49 m², çevre 41141,593 → 38741,593 mm oldu. Bir undo/redo
  tamamını birlikte geri/ileri alır. Yerel Aç iletişimiyle kaydedilmiş dosya
  yeniden açıldı; sonraki kayıt bağımsız okuyucuda önceki modelin tamamıyla eşit.
- 31–32: gerçek pencere kenarından daraltıldı; ham görüntü 2200×1364 piksel,
  koyu tema / 18 px yazı. Üç kolon seçeneği tam okunur, fakat çizim kipinin
  yeni salt okunur açıklaması kırpılıyor. Bu görüntü kabul değildir; kısa
  açıklama yeniden derlenip aynı boyutta sınanmalıdır.

`verify.py` KentOS SDK/hazır geometri hesabı kullanmaz: bağımsız KCAD okuyucu,
Decimal shoelace ve analitik yarım daire alanıyla ilk gerçek kayıtları sınar.
Alan: 9,8 × 7,8 − 0,6 × 0,8 − π/4 = 75,1746018366… m². İlk oluşturma,
undo/redo ve tercih düzenlemesinde kaynak UUID/alanları ve proje ayarları korunur.
İkinci build'in ayrı `fixed-*.kcad` dosyaları aynı okuyucuyla ayrıca sınanır:
navigasyon/iptal tam model eşitliği, toplu undo/redo, aks/kolon kısmi düşümü,
aynı mahal/etiket/çizelge UUID'leri, tek undo/redo ve aç/kaydet tam eşitliği.

`gui-events.json` zaman/sırayı; `gui-*-binary*.json` gerçek programın ve
kaynaklarının hash'lerini tutar. Görüntüler CUA'dan alınan ham JPEG'dir.
`old-reader-refusal.json` önceki şema 102 okuyucunun 103 dosyasını
`file_unreadable` ile reddettiğini ve iletide sürüm sınırını söylediğini kaydeder.
`legacy-plan-equality.json`, önceki 400-seed tam planıyla yeni SDK sonucunun
aynen eşitliğidir; performans ölçümü değildir.

GPU kaydı:

- `gpu-before.log`: mevcut gerçek GPU testi Apple M5 / Metal'de −0,05 piksel
  beklerken sıfır hareketle düşer.
- `gpu-invariant.log`: konum çıktıları invariant olduktan sonra yakın/sıfır/uzak
  origin × iki eksen geçer; en büyük kapsama merkezi hatası 0,0029 piksel.
  Mevcut MSAA/HiDPI/pipeline/bileşim senaryoları da aynı gerçek GPU testinde geçer.

Sonraki gerçek GUI kayıtları:

- 33–34: Brüt/Yalnız net 83,64 m², Brüt/Her yöntemde 82,53 m²; etiket/çizelge
  aynı komutta değişir. Bağımsız alan: 10,2 × 8,2 − 0,4 × 0,8 − π/4.
- 35–36: S2 çapı 20000 mm yapıldı; bütün oda kaplandığında sınır ve son geçerli
  metraj korunur, mahal uyarı taşır. Tek undo önceki modelin tamamını getirir.
  35’te uzun Durum etiketi kırpılmıştır; başarı kabul edilmez.
- 37–38: üçüncü build’de kısaltılmış etiket hâlâ `(sabit)` son ekiyle kırpılır;
  sarılan uyarı sabit satır yüksekliğinden kesilir. İki görüntü de kusur kaydıdır.
- 39–40: dördüncü, son build. Tam en/içerik yüksekliğindeki uyarının dört satırı
  ve sonraki Ekler grubu ayrıdır. Çizilen kipte **Düşme (sabit)** tam okunur.
- 41–47: aynı son build’de uygulama ayarları gerçek fareyle değiştirilir;
  uyarı paneli açık/koyu × dar/geniş × 13/18 px sekiz birleşimde okunur.
  Koyu/dar/18 birleşimi 39’dadır. Görseller ham 2× JPEG’dir; geniş pencere
  2880×1736, dar pencere 2200×1364 piksel. Geniş testin yüksekliği 900 mantıksal
  piksel hedefini sağlamaz; bu genel §10 tam iş akışı kabulü değildir.
  Son `accepted-covered-reopened.kcad` bütün modelde `fixed-fully-covered.kcad`
  ile eşittir; tema/yazı/pencere/seçim/kayıt çizimi değiştirmez.

Uyarı panelinin sekiz görünümü bütün araçların sekiz defa işlendiği anlamına
gelmez. ARCH-SPACE-01 kalıcı sanal sınır, kaynak alt kümesi, uçtan uca performans
ve genel GUI matrisi nedeniyle [~] kalır. Bu çalışma mevzuat alan standardı
veya bütün GPU donanımlarında doğruluk iddiası değildir.

Son kaynak doğrulamaları `validation.json` içindedir. Son GUI programının
Rust/WGSL dosya hash’leri mevcut kaynakla karşılaştırıldı. Son değişikliklerden
sonra 279 UI testi, üç kolon etkileşim testi, 38 özellik paneli testi (dört
atlanan), üç katalog testi ve uyarı satırının altı ölçü birleşimi geçti.
Native geniş tekrar 198 başarılı/22 atlanan; önceki interaction/MCP/KCAD
birleşik çalışması 1012 başarılı/8 atlanan. Paylaşılan WASM kontrolü, son
masaüstü derlemesi ve `clippy --all-targets -- -D warnings` başarılı.
SDK güncellik kontrolü ve 889 satırlı CAD manifesti tutarlı. GPU ölçümü kendi
Metal kaydında; skipped testler başarılı sayılmaz. Python FFI son katalogla
tekrar derlendi; son Python kümesinde 289 test 246,084 saniyede geçti.
