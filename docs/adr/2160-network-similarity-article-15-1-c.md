# 2160 — GNSS ağında Madde 15(1)(c)'nin benzerlik dönüşümü ve ölçek testi

Tarih: 2026-11-08. plan/map-cad.md MAP-05 (ve MAP-03), üçüncü tur. Durum: taslak. Plan satırı: “Üç boyutlu belirsizlik ve
büyük ağ kalite yönetimini geliştir.” İlgili:

- [ADR 1740](1740-gnss-network-from-archive.md): arşivden GNSS ağı. Bu ADR onun “Sonra”sındaki “Madde 15(1)(c):
  TUTGA/TUSAGA-Aktif'le 3B benzerlik dönüşümü ve ölçek testi” maddesini kapatır;
- [ADR 2120](2120-network-at-the-measurement-epoch.md): ölçme epokuna taşınmış bilinen noktalar;
- [ADR 0203](0203-network-adjustment.md): ortak istatistik çekirdeği.

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460 Mükerrer). Metin Çanakkale Onsekiz Mart Üniversitesi Harita Mühendisliği'nin
  yayımladığı kopyadan okundu (s. 6967; 8 Kasım 2026, PDF'in metni):
  - **Madde 15(1)(c):** TUTGA/TUSAGA-Aktif noktalarının ölçme anındaki verilen koordinatları ile C1 ağının
    zorlamasız (serbest) dengeleme sonucu bulunan koordinatları arasında üç boyutlu benzerlik dönüşümü yapılır, ölçek
    uyuşumu istatistik yöntemle test edilir. Ölçek faktörü için |1 − λ| ≤ 3 ppm olmalıdır.
  - **Madde 15(1)(d):** istatistik güven 1 − α = 0,95'tir.
  - **Madde 15(1)(a)** (aynı sayfa) epok kaydırmayı X(T) = X(T₀) + (T − T₀)·V ile verir: T₀ TUREF'in 2005,0 epoku,
    V noktanın hızı. Bu, noktanın kendi hızıyla taşınmasının dayanağıdır (Sonra).
- **Horn (1987)**, “Closed-form solution of absolute orientation using unit quaternions”, JOSA A 4(4):
  - en küçük karelerle benzerlik dönüşümünün kapalı çözümü;
  - bağımsız başvurunun yöntemi budur. KentOS Gauss–Newton'la aynı en küçük karelere yakınsar.
- **Student t:** iki yanlı test için t’nin (1 + güven)/2 nicemi. Çekirdekte düzenlenmiş eksik beta fonksiyonu (Lentz'in
  sürekli kesri) ve ikiye bölme vardır; scipy 1.18.0'ın `t.ppf`'iyle 10⁻⁹ içinde aynıdır.
- **KentOS bugün** (ADR 1740): C1'in baz sınırları (15(1)(b)) ve nokta sınırları (15(1)(ç)) denetleniyordu.
  15(1)(c)'nin benzerlik dönüşümü yoktu.

## Karar

1. **Benzerlik dönüşümü.** `map.survey.network`, üç ya da daha çok bilinen noktada dönüşümü kurar: verilen = T + λ·R·serbest.
   - Verilen koordinatlar ölçme epokundadır (ADR 2120). Serbest koordinatlar serbest aşamanındır: tutulan nokta kendi
     verilen yerinde, öbürleri bazlardan.
   - Çözüm birim ağırlıklı en küçük karelerdir: λ = 1, R = I'dan Gauss–Newton; her adım dönüşmüş noktaların
     ortalamaya indirgenmiş koordinatlarında doğrusallaştırılır. R Rodrigues'le dönüklük olarak kalır; adımlar
     10⁻¹⁵'in altına inince durur.
   - Döndürmeler R'nin ters bakışımlı kısmıdır (konum vektörü kuralı, açı saniyesi).
   - Dönüklüğün σ0² = vᵀv / (3n − 7)'dir. Ölçeğin σ'sı, ortalamaya indirgenmiş serbest noktalarla doğrusallaştırılmış
     normallerden gelir.
   - Ölçeğin testi t = |λ − 1| / σ'dır; sınır Student t'nin (1 + güven)/2 nicemi, serbestlik 3n − 7'dir. Güven
     sınıfınkidir (C1: 0,95); sınıf vermiyorsa Madde 15(1)(d)'nin 0,95'i.
   - Cevapta `similarity` şunları taşır: noktalar, öteleme (m), döndürmeler, λ − 1 ve σ'sı (ppm), serbestlik, t, nicem,
     `agrees` ve her bilinen noktanın doğu, kuzey, yukarı artığı.
   - Üçten az bilinen noktada `similarity` yoktur.
2. **Sınıfın ölçek sınırı veridir.** Kural paketinde sınıfa `similarity` (ppm) eklenir; C1'inki 3'tür, paketin
   sürümü 2'ye çıkar. Sınıfın sonucu `rules.similarity` (|λ − 1| ≤ sınır) olur ve `passed`'a girer. Tutmazsa
   `network_rules_failed` uyarısı ölçeği söyler. Kabulü durdurmaz: teknik denetimdir, kurum onayı değildir.
3. **Masaüstü** (GNSS ağ dengelemesi; 8 Kasım 2026'ya dek Arşivden ağ dengelemesi): özette benzerlik satırı vardır:
   - ölçek ve σ'sı, t ve nicem, uyumlu ya da uyumsuz, sınıfın sınırını tutması;
   - üçten az bilinen noktada bunun koşulu.

   Sınıf satırı neyin tutmadığını söyler: bazlar, noktalar, ölçek. Bilinen noktalar tablosunda **Artık D/K/Y (mm)**
   sütunu vardır.
4. **Kapsam işareti:** **Harita + CBS** (komut ikisinde salt okunur; pencere Harita'nın).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/map_survey_network_cases.py`: Horn'un kuaterniyon çözümü, numpy, scipy'nin
  Student t'si; KentOS kodu yok. 38 durum, iki yeni:
  - **üç bilinen nokta:** ölçek −0,19 ppm (σ 0,07), t = 2,71 ≤ 4,30, C1 tutar;
  - **aynı noktalar ortalamalarına göre 5 ppm büyütülmüş:** 4,81 ppm, t = 66, C1 tutmaz; dayalı aşama da uyuşmaz.

  t, 10⁻⁷'lik ölçek farkının 10⁻⁸'lik σ'sına bölümüdür. Serbest koordinatlardaki 10⁻⁹ m'lik çözücü farkı onu 10⁻⁶
  kadar oynatır. Bu yüzden durum 1e-5 içinde karşılaştırılır; uyuşmayan ağ (vᵀPv ≈ 555) 1e-4 içinde. Rust ve Python
  geçer.
- **Çekirdek:** `survey::adjust::student` scipy'nin değerleriyle sınanır.
- **MCP** (`crates/native/mcp/tests/survey_similarity.rs`): durumun çizimi `.kcad` olarak açılır; benzerlik ve kural
  sonucu durumunkiyle aynıdır.
- **Masaüstü** (`survey_epoch_tests.rs`): üç noktada benzerlik ve C1 tutar; 5 ppm'de aşar. Resimler
  `.run/gui-evidence/ag-benzerlik-2160/`.

## Sonra

- Ağırlıklı benzerlik (bilinen noktaların ve serbest koordinatların kovaryansıyla).
- Bilinen noktanın kendi hızıyla epok kaydırma (Madde 15(1)(a)'nın formülü).
- Robust kestirim, seyrek çözüm ve büyük ağ, deformasyon (MAP-05'in kalanı).
