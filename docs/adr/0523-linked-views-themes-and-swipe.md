# ADR 0523: Bağlı görünümler, görünüm teması ve kaydırmalı karşılaştırma

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-19'un ilk dilimi (G1: “mevcut iki görünüm ve named view
  güvenilirliği”; üstüne bağlı görünüm, tema ve swipe).
- **Bağlam belgesi:** ADR 0285 §3 (Görünümü böl: iki model görünümü, etkin olan), 0177 §4 (katman durumları), 0311 §3
  (durumların dondurma ve yazdırması), 0090 (stilli çizim, görünüm başına önbellek), 0212 (etiket motoru).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - zaman ve ölçek bağlamı: iki görünümün zaman sürgüsünün ayrı penceresi (bugün sürgü belgenindir, iki görünüm aynı
    zamanı gösterir) ve ölçeği kilitli bağ;
  - bağlı katmanın ya da bulut projesinin iki revizyonunu yan yana gösterme (CBS-36/37; bugün iki görünüm aynı çizimdir);
  - ikiden çok görünüm, adlı görünüme temanın yazılması;
  - öbür görünümde, kendi temasının gizlediği bir nesnenin üzerine gelme vurgusu da çizilir (seçim ve üzerine gelme iki
    görünümde ortaktır).

## Bağlam

CBS'de görünüm yalnız kamera değildir: aynı yerde ortofoto ile kadastroyu, eski ile yeni durumu yan yana ya da üst üste
görmek gerekir. KentOS'ta iki bağımsız kameralı bölünmüş görünüm (ADR 0285) ve projenin adlı katman durumları (ADR 0177)
vardı; ama iki görünüm çizimi aynı katman görünürlüğüyle gösteriyordu ve kameraları ayrıydı. Plan bunları yeniden
kurmadan, veri kopyalamadan bağlı görünüm ve swipe ister; temanın çizimi değiştirmemesini, hangi görünümün düzenleme
hedefi olduğunun belli olmasını, görünüm kapanınca kaynakların bırakılmasını.

## Karar

### 1. Görünümün teması

Her görünümün (bölünmüş alanda ikisinin de) bir **teması** olabilir: projenin katman durumlarından biri, ya da “Çizimdeki
gibi”. Tema bir **görünüştür**:

- Durumun tuttuğu görünürlük, dondurma ve stiller ağacın bir **kopyasına** konur
  (`kentos_domain::layer_states::themed`); belge, katman ağacı, revizyon ve geri alma değişmez. Kilitler temaya girmez:
  kilit bir düzenleme kuralıdır, görünüş değil.
- Etkin katman temada da dondurulmaz (durumu uygulamanın kuralı, `changes_keeping`).
- Durumun adlandırmadığı düğümler çizimdeki gibi kalır; projeden silinen durum görünümün temasından da düşer.
- Görünümün stilli çizimi kendi önbelleğinde kurulur (`StyledCache::scene_themed`); temanın gösterdiği yeni katmanlar
  kurulur, çizimdekiyle aynı stilde kalanlar yeniden kurulmaz. Veri kopyalanmaz: iki görünüm aynı belgeyi ve aynı geometri
  deposunu okur.

### 2. Düzenleme hedefi ve etiketler

- **Etkin görünüm** (çerçevesi vurgu renginde) düzenleme hedefidir: araçlar, seçme ve kenet onun teması neyi gösteriyorsa
  onu görür (`Spatial::set_theme`: geometri deposunun katman tablosu etkin görünümün temasından). Öbür görünüme basmak onu
  etkin yapar ve seçme onun temasına geçer.
- **Etiketler** her görünümde kendi temasıyla yerleşir: depo, katman tablosundan bağımsız bir görünürlük maskesiyle
  etiket sorar (`Store::labels_shown_in`; ortak çekirdekte, var olan `labels_shown` değişmeden).
- Seçim iki görünümde ortaktır: bir görünümde seçilen kayıt öbüründe de vurgulanır.

### 3. Bağla ve Kaydırmalı karşılaştır

- **Görünümleri bağla** (`view.splitLink`): öbür görünüm etkin görünümün merkezini, ölçeğini ve dönüklüğünü her mesajdan
  sonra izler; yeniden basmak bağı çözer. Alan bölünmemişse önce böler.
- **Kaydırmalı karşılaştır** (`view.swipe`): öbür görünüm etkin görünümün üstünde, sürüklenen bir ayırıcının sağında
  çizilir; aynı yer aynı pikselde kalsın diye öbür görünümün merkezi, etkin görünümün kendi ortasını gördüğü yerdir. Öbür
  görünüm **edilgendir**: boyutunu söyler, imleç ve basmalar onun altından etkin görünüme gider (araç, kenet, üzerine gelme
  etkin görünümde sürer). Etkin görünümün işaretleri (artı imleç, kenet, taslak) öbür görünümün de üstündedir.
- Her görünümün üstünde tema seçicisi: yan yanada her görünümün sol üstünde, karşılaştırmada solda etkin görünümünkü,
  sağda öbürününkü.
- Tek görünüme dönmek (Görünümü böl'e yeniden basmak) temaları bırakır; öbür görünüm, önbelleği ve etiketleriyle düşer;
  seçme çizimin ağacına döner.

### 4. Komut, Python ve MCP

Tema, bağ ve karşılaştırma oturumun **görünüş durumudur**: çizimi ya da projeyi değiştirmez, kayda girmez; ürün komutu
yoktur (CLAUDE.md §9.2'nin kuralı çizimi değiştiren araçlar içindir). Arayüz komutları `catalog/v1/additions.json`'da,
manifestte oturum düzeyindedir.

## Doğrulama

- `cargo test -p kentos-desktop split_view`: temanın öbür görünümde katmanı gizlemesi, çizimin ve revizyonun değişmemesi,
  etkin görünümün temasıyla seçme, Bağla'nın merkez ve ölçeği, tek görünüme dönüşte temanın bırakılması; karşılaştırmada
  aynı yerin aynı pikselde olması ve ayırıcının sürüklenmesi.
- Resimler: `KENTOS_SHOTS_ONLY=gorunum-tema,gorunum-karsilastir cargo test -p kentos-desktop tools_screens -- --ignored
  --nocapture` (`.run/shots/arac-gorunum-tema-*`, `arac-gorunum-karsilastir-*`): solda Kadastro teması (parseller mavi ve
  kalın, yol gizli), sağda Yol teması (yalnız yol).
