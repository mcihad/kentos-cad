# ADR 0280: Tutamaç kipleri ve birden çok sıcak tutamaç

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-16`: “Birden çok tutamacı etkinleştirme, referansla
  ölçek/döndürme, base point değiştirme ve copy modu desteklensin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0068 (tutamaçlar, sıcak tutamaç), 0074 ve 0279 (tutamaç menüsü), 0037 (dönüşümler), 0160 (topolojik düzenleme).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Sıcak tutamaç yalnız kendi köşesini taşıyordu. AutoCAD kullanıcısının alışkanlığı tutamaç kipleridir: Shift ile birden çok tutamacı
sıcak yapmak, sıcak tutamaçtan seçimi taşımak, döndürmek, ölçeklemek ve aynalamak; taban noktasını değiştirmek, referansla döndürmek ve
ölçeklemek, aynı düzenlemeyle birden çok kopya bırakmak. Döndür ve Ölçekle araçlarında Referans ve Kopya vardı, tutamakta yoktu.

## Karar

1. **Birden çok sıcak tutamaç:** Shift basılıyken seçili nesnelerin tutamağına tıklamak onu sıcak tutamaçlara ekler, sıcaksa çıkarır;
   günlük kaçının sıcak olduğunu söyler. Sıcaklardan birine Shift'siz tıklamak düzenlemeyi o tutamaçla başlatır, öbürleri onunla gelir;
   başka yere tıklamak ya da Esc hepsini soğutur. Sıcak tutamaçlar büyük çizilir.
2. **Kipler:** sıcak tutamaç Esnet'le başlar; harfleri E (Esnet), T (Taşı), D (Döndür), L (Ölçekle), A (Aynala). Esnet her sıcak
   tutamağı aynı uzaklıkla taşır: bir nesnenin birden çok tutamağı arasında kenarın ortası o kenarın iki köşesidir (tek başına sürüklenen
   kenar ortası eskisi gibi köşe ekler). Öbür kipler seçimin bütününe uygulanır: Taşı tabandan imlece, Döndür tabanın çevresinde, Ölçekle
   tabandan (referans yoksa taban ile tutamaç arası uzaklık referanstır, tutamaç imleci izler; ikisi aynı yerdeyse bir çizim birimi), Aynala
   tabandan imlece giden doğruya göre yerinde. Döndür'e yazılan sayı derece, Ölçekle'ye yazılan sayı faktördür (referansla yeni
   uzunluk).
3. **Taban noktası (B):** sonraki nokta tabandır; taban tutamaçtan başka yerde olabilir. Esnet de tabandan ölçer.
4. **Referans (R, Döndür ve Ölçekle'de):** iki noktayla gösterilir ya da yazılır (açı derece, uzunluk çizim biriminde).
5. **Kopya (K):** her yerleştirme kopya ekler, tutamaç sıcak kalır; Esc bitirir. Esnet'in kopyası tutamağı taşınmış yeni nesnedir,
   özellikleriyle; kopyada topolojik komşu izlemez.
6. **Yazma:** Esnet `cad.entities.edit`'in `grip`'iyle (her nesnenin bütün yeni geometrisi, kopyada `add`; topoloji açıksa komşular
   `neighbours_of` ile aynı adımda), öbür kipler `cad.entities.transform`'la (Taşı, Döndür, Ölçekle, Aynala adımları). Önizleme kipin
   kendisidir: Esnet nesneleri ve komşuları, öbürleri seçimin hayaletlerini ve açıyı, faktörü ya da uzunluğu gösterir. Kipin harfleri
   masaüstünde komut çalışmazken de seçenektir (kısayoldan önce).

## Sonuçlar

- Tutamaç menüsünün Taşı, Döndür, Ölçekle ve Aynala'sı (ADR 0279) araçları başlatır; tutamaç kipleri aynı işi aracı başlatmadan yapar.
- Kısayol listesinde “Shift + tutamaca tık”, “E / T / D / L / A” ve “B / R / K”.

## Doğrulama

- `cargo test -p kentos-interaction --test all grip` (çoklu sıcak tutamaç, kenar ortası, kipler, taban, referans, kopya).
- `cargo test -p kentos-desktop input_scenes` (Shift, harf ve yazılan açı masaüstünde); bütün izler değişmeden geçer.
- Resim: `KENTOS_SHOTS_ONLY=tutamac-kipleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
