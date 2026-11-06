# 0979 — Ove klotoid PI'lardan: A çözülür, verilmez

Tarih: 2026-11-06. Durum: kabul (Y72). Kapsam ROAD-ALIGN-15 (ove klotoid; önerili düzeltme taslağı açık).

## Sorun

ROAD-ALIGN-15 şunu ister: ove spiral, ters kurp ve kesişen kurpların çakışma denetimi ayrı eleman ve kural olsun.
Netpro'nun AASHTO hız–yarıçap ile kurp çakışmasını kaldırması gibi bir öneri ise ayrı bir taslaktır; kaynak aksı sessiz
değiştirmez.

ADR 0922 ters, kırık sırtlı ve bitişik kurp çiftlerini sınıfladı. `element.rs` iki ucunda da eğriliği olan klotoidi
taşıyabiliyordu. Eksik olan, aynı yöne dönen iki kurbun bir ove klotoidiyle PI zincirinden kurulmasıydı.

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı ve uygulaması

- **KGM 2019 Kesin ve Ön Projeler Teknik Şartnamesi:**
  - "Bütün yollarda geçiş elemanı olarak KLOTOİD kullanılacaktır." (s. 5/64)
  - Kurp some bilgileriyle verilir: R, sapma açısı (grad), T, L, dever, genişleme, Lr/Lt/La, koordinatlar ve "klotoid
    ile ilgili değerler".
  - Paftada "klotoid spiral sabiti" yazılır (s. 16–17).
  - Başvurular KTEK, HCM ve AASHTO'dur (s. 18).
  - Ove, bileşik kurp ya da A sınırı hakkında **hüküm yok**.
- **KGM Karayolu Tasarım El Kitabı (KTEK 2005) 3. bölüm:** Klotoid parametresinin güvenlik, konfor ve estetik
  sınırlarını verir (denk. 3.27/3.28, Şekil 3.6). Yalnız arama özetlerinde görüldü; metin okunamadı. KTEK 2022'ye de
  ulaşılamadı.
- **Karayolları Kenarında Yapılacak Tesisler Yönetmeliği** (RG 15.05.1997/22990, değişiklikler RG 06.01.1998/23222 ve
  13.08.2022/31922): Geometri içermez.
- **Türkiye'de ove için resmî bir kural bulunamadı.** Ne R1/R2 sınırı ne en kısa ove var. KentOS bu yüzden kural
  sayısı vermez (ROAD-DEC-01).
- **Koç, Gümüş, Selbesoğlu 2015** (YTÜ; *Tehnički vjesnik* 22(2) 495–501): Girdiler R1, R2 ve daireler arası D'dir.
  Yaklaşık formül A = ⁴√(24·D·R³), R = R1R2/(R1−R2). Kesin geometriyle karşılaştırıldığında açıklığı %0,48 fazla verir.
  Başlangıç değeri olarak kullanılabilir, kesin çözüm yerine geçmez.
- **Pırtı 2009** (hkm 2009/2 s. 35–39): "Yumurta eğrisi"ni dördüncü derece parabolle kurar.
- **Netcad NETPRO:**
  - "Obje Spiral Obje" komutu seçilen iki yay arasına "klotoid ve ove türünde" geçiş koyar.
  - **Yalnız eleman zinciri yöntemindedir**; PI tablosunda (Someler) ove girişi yoktur.
  - "Kurplardaki Çakışmaları Engelle" seçeneği çakışan kurpları küçültür. ADR 0922'deki iddianın birincil kaynağı budur.

### Yabancı yazılımlar

- **Civil 3D, *Free Spiral (Between Two Curves)*:** "only one geometric solution, you cannot specify length or A
  value." Uzunluk da A da girilemez. *Floating Curve With Spiral* ile ikinci daire yüzer.
- **OpenRoads, *Spiral between Elements*:** "Two arcs where one arc is contained entirely within the circle of the other".
- **tcpMDT:** "the curve having the smaller radius must be defined within the curve having a greater radius."
- **Ortak nokta:** Hiçbiri PI tablosunda ardışık iki PI arasına ove kurmayı belgelemez.

### Matematik

**Yayımlanmış:**

- Klotoidin noktası P(s) = A√π·(C, S)(s/(A√π)), τ = s²/(2A²).
- Eğrilik merkezi M(s) = P(s) + (A²/s)(−sin τ, cos τ).
- Ove s ∈ [A²/R1, A²/R2] parçasıdır, L = A²·|1/R2 − 1/R1|.
- Varlık koşulu: küçük daire büyüğün içinde, merkezler farklı (Stoer 1982, Teorem 2.4, *J. Res. NBS* 87(4)).
- Dairelerden yerleştirme: Vázquez-Méndez, Casal ve Ferreiro 2020, *J. Surv. Eng.* 146(1), Algoritma 2.

**Türetilmiş (bu ADR'nin modeli):** Ortadaki aliyman, uzatılmış ove klotoidinin başlangıç teğetidir: O noktasında
eğrilik 0'dır; Almanca'da *Hilfstangente*.

- **Dairelerin yeri:** Her daire, O'dan Λ = A²/R boyunda klotoidi olan bir kurbun dairesinin durduğu yerdedir:
  - ortadaki teğetten R + p(R, Λ) uzakta;
  - ayağı O'dan x_M(R, Λ) ötede;
  - yarıçap küçülüyorsa (kapanan ove) O iki ayaktan önce, büyüyorsa (açılan ove) iki ayaktan sonradır.
- **Tek koşul** iki O'nun bir olmasıdır. Bu yalnız A'nın fonksiyonudur:

  `g(A) = |BC| + t2 − u1 + s·(x_M2 − x_M1)`, kapanan ovede s = −1, açılanda +1.

  g(0), dairelerin ayakları arasındaki düz açıklıktır.
- **A'nın üst sınırı:** Kapanan ovede ikinci dairenin yayı, açılanda birincinin yayı biter:
  - kapanan: A ≤ R2·√(2(|Δ2| − τ_out));
  - açılan: A ≤ R1·√(2(|Δ1| − τ_in)).
- **Yaylar:**
  - birinci yay R1(|Δ1| − τ_in ± τ1);
  - ikinci yay R2(|Δ2| − τ_out ∓ τ2);
  - τ = A²/(2R²).
- **Kök bulma:** A, (0, üst sınır] aralığında 512 yerde örneklenir; her işaret değişimi ikiye bölmeyle (Rust) ya da
  mpmath'in kök bulucusuyla (başvuru) çözülür.
- **Tek kök:** Kabul edilir.
- **Kök yok:** Her yerde g > 0 ise `OveTooLong` (ortadaki aliyman uzun, en küçük g ile), her yerde g < 0 ise
  `CurvesOverlap`.
- **İki ya da daha çok kök:** `OveAmbiguous` (A'lar yazılır; hangisinin istendiği PI'lar oynatılarak seçilir).
  Gerçek bir durumdur: R 1000 ile 900, Δ 0,3 ile 0,3 olan PI'lar A = 120 ve 399,1 iki oveyi de taşır.

## Karar

### Girdi modeli

**Aks noktasının `ove` bayrağı:** "Bu PI'nın kurbu sonrakine ove ile bağlanır." Kaynak PI'lar ve iki yarıçaptır. A ve
L çıktıdır (Civil 3D'nin *free* spiraliyle aynı belirlilik).

Retler (`BadOve`):

- sonraki PI'nın kurbu yok ya da sonraki bitiş;
- ters kurp;
- eşit yarıçap;
- aradaki yanda klotoid var (ove onların yerine geçer);
- art arda iki ove.

### Çekirdek (`chain.rs`)

- **Çözüm:** `solve_ove` A'yı çözer; dairelerin merkezlerini ve O'ya teğet boylarını günceller.
- **Elemanlar:** Zincir sırası birinci yay, `p1/ove` (Spiral, k0 = σ/R1, k1 = σ/R2) ve ikinci yaydır. Dikişler G2'dir:
  eğrilik sıçramaz.
- **Kurp raporu:** Birinci kurbun ST'si ovenin sonu, ikincinin TS'si ovenin başıdır. Birinci kurbun kendi bitiş artığı
  denetlenmez; ikinci kurbun ST'si bütün yapıyı PI geometrisine karşı denetler.
- **Bağ:** `Join.ove` A'yı, boyu, başladığı km'yi ve daireler arası açıklığı (|R1 − R2| − merkezler arası) taşır.

### Komut, kayıt, Python, ekran

- **Komut:** `road.alignments.create`/`edit`/`query` bağın `ove`'sini yazar (`AlignmentOveRow`).
- **Ret iletileri:**
  - "p1–p2 arasına ove sığmıyor: … ortadaki aliyman X m uzun kalıyor";
  - "birden çok ove var (A = …)";
  - "p1 noktasında ove olamaz: …".
- **KCAD şema 188:** `ove`; 187 ve öncesi `unknown_field` der. Örnekler `road-ove.kcad` ve
  `broken/road-ove-in-schema-187.kcad`; gelecek şema dosyası 189 oldu.
- **Python:** `kentos.cad` yeniden üretildi.
- **Güzergâh penceresi:**
  - PI tablosuna "Ove" sütunu geldi: başlangıçta, bitişte ve bitişten önceki PI'da "—".
  - Durum satırı "p1–p2 ovesi A 250,000 m, L 187,500 m, daireler arası 4,363 m" der.

**Pencerede hassasiyet (bulgu):** Pencere PI'ları üç ondalıkla gösteriyor ve yazılanı okuyordu. Ove A'yı PI'lardan
çözdüğü için koşullanması zayıftır: mikrometrelik yuvarlama A'yı milimetre düzeyinde oynattı ve hiçbir şey
değişmemişken "km'ler kayar" uyarısı çıktı. Artık yazı değişmedikçe projenin tam sayısı kullanılır (`PiRow.kept`).

## Bağımsız başvuru

`scripts/fixtures/road_alignment_cases.py` oveyi ders kitabının teğet boyu formülüyle kurar:
`((R + p_giriş) − (R + p_çıkış)·cos Δ) / sin Δ`, mpmath ile 40 hanede. Rust'ın iki doğruyu kesiştiren çözümüne
dokunmaz.

Durumlar egg klotoidinin kendisinden kurulur (`ove_pis`):

- O, ortadaki teğetin üstündedir;
- daireler A²/R'deki salınım daireleridir;
- PI'lar teğetlerin kesiştiği yerlerdir.

Başvuru A'yı bunlardan bağımsız olarak geri çözer.

## Ölçüm

**Çekirdek derlemi (`fixtures/road/v1/alignments.json`):** 45 durum, 19'u ove:

- kapanan ve açılan;
- sola ve sağa;
- giriş ve çıkış klotoidli;
- TM koordinatlarında 37° dönük;
- yakın yarıçaplar (R 1000 ve 900);
- çok farklı yarıçaplar (R 2000 ve 150);
- uzun ove (ikinci yay neredeyse biter);
- ardından ayrı bir kurp;
- retlerin hepsi.

Hepsinde elemanlar, km'ler, noktalar, ayaklar ve ötelemeler 10⁻⁸ m içinde aynı; A 10⁻⁶ içinde.

Başvuru kurulumdaki A'yı geri buldu: 250, 400, 120, 200, 480, 150. R 500 → 200, A 250 için daireler arası açıklık
4,36284862… m'dir. Bu, araştırmanın Fresnel integralleriyle 40 hanede ayrıca hesapladığı değerle aynıdır.

**Komut:** `road.alignments.create` 39 durum (yeni: iki ove, üç ret). Bütün yol durumları Rust'ta ve Python tipli
katmanda geçer.

**KCAD:** 1624 dosya; Rust ve Python okuyucuları aynı.

**Pencere testi:** Kutu işaretliyken A 250 bulunur. Kaldırılınca ove gider ve km'lerin kayması gösterilir. Kaydedilir,
tek geri alma adımıyla ove geri gelir. Resimler `.run/gui-evidence/yol-y72/` altındadır.

## Açık

- **Önerili düzeltme taslağı:** "Ove yap", "teğeti kısalt", "PI'yı kaydır", hıza göre yarıçap. Ayrı bir taslak aks olarak
  gelecek (ROAD-ALIGN-12'nin alternatif aks yapısıyla).
- **Standart profilinin ove satırları:** `oveLeastGap`, A/R sınırları, `mostRadiusRatio`'nun ovede geçerliliği. Kural
  sayıları kurumun profilinden gelir; KTEK okunamadı.
- **Eleman zinciriyle ove:** İki sabit daire arasında, Netcad'in ve Civil 3D'nin yolu (ADR 0901 §Sonra).
- **Dever:** Ovenin iki ucundaki dever oranları arasında geçiş, dever çizelgesinin "kurplardan" yönteminde ayrıca
  denetlenmeli. İki kurp ovede bitişik sayılır.
