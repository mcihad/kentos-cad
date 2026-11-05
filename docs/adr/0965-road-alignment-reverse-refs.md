# 0965 — Ters çevirmede km'ler ve etki raporunun bütün km adları

Tarih: 2026-11-05. Durum: kabul (Y59). Kapsam ROAD-ALIGN-10'un kalanı (eşleme ve düzenleme etkisi ADR 0963, dever ADR 0964).

## Sorun

ADR 0963'ün etki raporu iki şeyi eksik bırakıyordu:

1. **Ters çevirme sorulamıyordu.** Ters çevrilen aksın boykesitleri yerinde kalıyor ve eskiyordu. Kullanıcı PVI'ları
   elle L − s'ye taşımak zorundaydı.
2. **Bazı km adları raporda yoktu.** Rapor yalnız boykesit PVI'larını, kesit takımlarını, koridorları ve deveri
   biliyordu. Aksa km ile bağlı ama raporda olmayanlar:
   - boykesit kesişmeleri (ADR 0946);
   - kübaj katsayı ve sıyırma aralıkları;
   - Brükner alanları, uygunsuz aralıkları ve engelleri;
   - menfez, istinat duvarı ve pafta takımı km'leri.

ROAD-ALIGN-10: "Alignment edit ve reverse için segment/station mapping üret; ölçü, profil, corridor region ve stakeout
refs'e impact raporu ver."

## Araştırma

- **Civil 3D, "To Reverse Alignment Direction":** Bağlı nesneler yeniden işlenir; km eşitlikleri ve tasarım hızları
  silinir.
- **Kullanıcı bildirimleri:** Civil 3D'de boykesit aynalanmıyor. Forumda 100 aksın düşey geometrisini yeniden kurmak
  zorunda kalan bir kullanıcı var; profil görünüşünü sağdan sola çizmek geçici çözüm olarak öneriliyor.
- Kaynaklar: Autodesk Civil 3D Kullanıcı Kılavuzu (2015–2024), Autodesk forumu "Reverse alignment issue", "Reverse
  Alignment Direction with Attachments".
- **Yeni hesap yok:** Aynalama tanımdan çıkar. Kırmızı kot aynı fiziksel çizgidir, ters yönden okunur.

## Karar

### Boykesit ters çevirmede onunla döner

Yeni km t'deki kot, eski km L − t'deki kottur: z′(t) = z(L − t). Bundan şunlar çıkar:

- **PVI'lar:** km L − s olur, kotu aynı kalır, sıra tersine döner.
- **Eğimler:** −1 ile çarpılıp yer değiştirir: g₁′ = −g₂, g₂′ = −g₁. Böylece g₂′ − g₁′ = g₂ − g₁; tepe kurbu tepe,
  çukur kurbu çukur kalır.
- **Kurplar:**
  - simetrik parabol PVI'sına göre simetriktir, boyu aynı kalır;
  - daire de simetriktir, yarıçapı aynı kalır;
  - asimetrik parabolün PVI'dan önceki kolu artık sonradır: `lengthIn` ↔ `lengthOut`.
- **Kesişmeler:** km L − s olur; altta ya da üstte olması değişmez.
- **Bağlılık:** Boykesit aksın yeni revizyonuna bağlanır, revizyonu +1 olur.

Aks, boykesitleri ve dever çizelgeleri (ADR 0964) tek işlemde, tek "Aksı düzenle" adımında yazılır. Yanıt dönen
boykesitleri `profiles` alanında verir.

### Neler dönmez ve neden

Koridorlar, kesit takımları ve onlara bağlı olanlar (kübaj, Brükner, menfez, duvar) ile pafta takımları dönmez:

- **Yanlı tanımlar:** Koridor ve kesit takımı tip kesitle kurulur. Tip kesitin sol ve sağ yanı vardır; onları çevirmek
  sol yanı fiziksel sağa koyar, yani başka bir yol üretir.
- **Bunlara bağlılar:** Kübaj, Brükner, menfez ve duvar koridora bağlıdır.
- **Sonuç:** Bu km'ler eski yönde kalır ve eskir. Ters çevirmeden önce etki okunur; Güzergâh penceresi ters çevirdikten
  sonra bunları adlarıyla söyler (`alignment_reverse_stale`).

### `road.alignments.impact` genişler

- **`reverse: true`:** Yeni PI'lar yerine ters çevirme sorulur; ikisi birden reddedilir. Her km L − s'ye taşınır.
  Kayan ve yeni aralık yoktur. Tek aralığın kayması uçlarda +L ve −L'dir. Eleman eşlemesi "reversed" satırlarıdır.
- **`carried`:** Ters çevirmede boykesit ve dever km adları `carried: true` taşır; kendi kendine yazılırlar. Ötekiler
  `alignment_reverse_stale` uyarısıyla söylenir.
- **Yeni km adları** (her iki kipte):
  - boykesit kesişmesi;
  - kübaj katsayı satırı ve sıyırma aralığının başı ve sonu;
  - Brükner alanı, uygunsuz aralığın başı ve sonu, engel;
  - menfez km'si;
  - duvarın başı ve sonu;
  - pafta takımının başı ve sonu.

  Uyarıda hepsi Türkçe adlarıyla geçer ("1. katsayı satırı sonu", "Dere engeli").

### Aplikasyon ve ölçü

Bunların modelde saklanan bir km adı yoktur:

- aplikasyon her çalıştığında koridordan okunur;
- ölçü noktaları koordinatla durur.

Eşlenecek km yoktur. Maddedeki "stakeout refs" ve "ölçü" bu yüzden boş kümedir.

## Ölçüm

**Kırmızı kotun değişmezliği, bağımsız referansta.** `scripts/fixtures/road_alignment_command_cases.py` düşey
referansla (`road_vertical_cases.py`, kesirler ve mpmath) eski ve ters çevrilmiş boykesiti kurar. Boykesitte simetrik
parabol (L 120), asimetrik parabol (60/90), daire (R 3000) ve bir kesişme vardır. 401 noktada z(s) = z′(L − s) sınanır;
fark 1e-9'un altındadır, tutmazsa referans durur.

**Rust'ta.** `crates/native/application/tests/all/superelevation_reverse.rs`, çekirdeğin düşey hesabıyla 1001 noktada
aynı eşitliği 1e-9 içinde bulur. Kesişme L − s'dedir ve boykesit aksın yeni revizyonuna bağlıdır.

**Paylaşılan durumlar:**

- `road.alignments.edit`'e "Ters çevrilir: boykesit de döner" eklendi: yazma ve tek geri alma.
- `road.alignments.impact` 23 durum oldu. Yeniler: "Ters çevrilir: her km L − s'ye", "Yan aks ters çevrilir", "Hem PI
  hem ters çevirme" (ret).
- Setup'a menfez, duvar ve pafta takımı eklendi; bu yüzden eski 20 durumun km adları da arttı. Örneğin yarıçap
  150 → 180'de 8 değil 17 km adı.

Rust'ta ve Python'da geçer.

## Açık

- **Koridor ve kesit takımını ters çevirmek:** Tip kesitin aynasını gerektirir (sol ↔ sağ). Ayrı bir karar ister.
