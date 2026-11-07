# 1460 — Düzenleme dağıtım taslağı: kural paketi, kesin pay ve toplam korunumu (`cad.readjustment.distribute`)

Tarih: 2026-11-06. plan/map-cad.md MAP-11 (P2; kabulün ilk adımı: sentetik veriyle alan, pay ve değer korunumu ve kesin
yuvarlama, bağımsız hesapla). Durum: taslak. İlgili: [ADR 0004](0004-numeric-policy.md) (kadastral sayısal
politika), docs/specs/numeric-policy.md §6, [ADR 1400](1400-parcel-area-check-and-identity.md).

## Dayanak

- **3194 sayılı İmar Kanunu 18. madde ve Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik** (RG 22.02.2020/31047):
  - DOP, düzenlemeden önceki yüzölçümlerden imar planındaki kullanım kararlarına göre yüzde kırk beşe kadar
    düşülebilen miktardır (2019 değişikliğinden önce %40).
  - Oran virgülden sonra yedi basamakla hesaplanır (Yönetmelik m. 4/d).

  Okunan ikincil kaynaklar: [DergiPark](https://dergipark.org.tr/tr/download/article-file/996594),
  [Lexpera](https://www.lexpera.com.tr/mevzuat/yonetmelikler/arazi-ve-arsa-duzenlemeleri-hakkinda-yonetmelik),
  [Kadim Hukuk](https://kadimhukuk.com.tr/makale/duzenleme-ortaklik-payi-dop/) (6 Kasım 2026). Resmî metinle
  karşılaştırılmadı. Bu yüzden **oran KentOS'un kodunda yoktur**: kural paketinin verisidir.
- **Netcad:** Netmap, Netkamu ve Nettop dağıtımı, hakları ve tabloları geometriyle birlikte ele alır (plan/map-cad.md
  [S03], [S06], [S07]).
- **KentOS bugün:** çekirdekte kesin ondalık, kesir pay, sürümlü sayı politikası ve artık dağıtımı (`numeric::distribute`)
  vardır (ADR 0004). Bir düzenlemenin hesabı, kural paketi ve artığın raporu yoktur.

## Karar

1. **Kural paketi** (`rules`), girdi verisidir:
   - kimlik ve sürüm, kurum, kaynak belge, yürürlük, onay;
   - kesinti oranı (ondalık yazı, [0, 1));
   - sayı politikası: basamak ≤ 6, yuvarlama yönü, artık kuralı (`reject` ya da `largest_remainder`).

   Hiçbir yasal oran ya da sınır kaynak koduna sabit olarak gömülmez.
2. **`cad.readjustment.distribute` v1, salt okunur (Harita + CBS).** Kayıtlar parsel, hak sahibi (kimlik; kişisel veri
   kendi yetkili kaydında kalır), tapu alanı (ondalık yazı, politikanın basamağından ince değil) ve paydır (`n/d`).
   Hesap:
   1. **Kesinti:** kesin toplamı Σ alan × oran bir kez yuvarlanır, parsellere alanlarıyla dağıtılır.
   2. **Net:** her parselin neti alan − kesintidir.
   3. **Haklar:** net, hak sahiplerine kesin paylarıyla dağıtılır.
   4. **Artık:** her yuvarlamanın artığı en çok eksik kalana birer birim verilir (eşitlikte kayıt sırası), ya da
      kural `reject` ise reddedilir (`needs_rule`). Artığın gittiği parsel ve hak sahibi cevapta söylenir.
   5. **Toplamlar** kesin denetlenir (`conserved`): tapu = kesinti + net, hak sahiplerinin toplamı = net.

   Sayılar ondalık yazı ve kesirdir, float değil; ekrandaki yuvarlanmış sayı yeniden hesaba girmez. **Sonuç her zaman
   taslaktır** (`draft: true`): resmî karar ya da kuruma gönderim yoktur.
3. **Retler:** girdi (`invalid_input`), farklı tapu alanı (`area_mismatch`), aynı parselde iki kez hak sahibi
   (`duplicate_holder`), toplamı 1 olmayan paylar (`shares_not_whole`), reddedilen artık (`needs_rule`). Her biri yeriyle
   (`entries[i].…`) gelir.
4. **Çekirdek:** `numeric::distribute_shares` (kesir ağırlıkla dağıtım), `multiply_exact`, `sum_exact`,
   `subtract_exact`; `Distribution.given` artığın verildiği kalemleri söyler.
5. **Pencere Düzenleme dağıtım taslağı…** (`readjustment.distribute`, ikon `readjustment`):
   - **Yeri:** Harita'da Plan › Ada ve parsel, CBS'de Veri › Tablo. Yol ve Mimari sunmaz.
   - **Form:** kural paketi kısa alanlarla, birbirine ait olanlar aynı satırda (sahibin isteği: düzgün, her zaman alt
     alta değil).
   - **Kayıtlar:** CSV'den okunur (`parsel;hak sahibi;tapu alanı;pay`).
   - **Sonuç:** taslak uyarısı, korunum ve kesinti satırları; Hak sahipleri ve Parseller görünümleri (artık → kime).
   - **Sonucu kaydet:** sonucu CSV olarak yazar.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/readjustment_cases.py` 30 durum üretir. Hesap Python `fractions`'la ve elle
  yazılmış yuvarlama kipleriyle yapılır; KentOS'un sayı kodu kullanılmaz. Durumlar şunları kapsar:
  - üçte bir ve yedide bir paylar (0,01'lik artık ilk en çok eksik kalana);
  - `reject`'in artıkta reddi ve artık yokken geçmesi;
  - yarımda çifte ve sıfırdan uzağa (−0,01'lik artık geri alınır), sıfıra doğru;
  - basamak 0, kesinti 0, büyük alan ve ince oran, parseller üzerinden toplam, sıfır alan, sadeleşen pay;
  - bütün retler.

  Rust ve Python geçer.
- **Masaüstü:** `readjustment_window_tests.rs` sınar:
  - CSV okumayı, korunumu ve artığın K1'e gittiğini;
  - `reject`'in yerini söylemesini, çizime yazılmadığını, modül işaretlerini.

  Resimler `.run/gui-evidence/duzenleme-taslagi-1460/`.

## Sonra (MAP-11'in kalanı; sahibiyle ayrı kapsam)

- Değer esaslı dağıtım, kamulaştırma ve irtifak, arazi toplulaştırması.
- Kapsam geometrisi ve öneri parselasyonu. Kapsam ve ayırma çapının sayıları:
  [ADR 2060](2060-readjustment-scope-and-separation-sheet.md). Öneri parselasyonu açık.
- İtiraz ve düzeltme, sürüm karşılaştırması, belge ve kişisel veri yetkileri.
- İkinci kabul: anonim gerçek örneğin yetkili uzman ya da kurumla karşılaştırılması. Bu yapılmadan “resmî işlem hazır”
  denmez.
