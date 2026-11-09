# 1420 — İmar planı tipli nesne olarak: `imar-plani` profili ve `cad.zoning.check`

Tarih: 2026-11-06. plan/map-cad.md MAP-09 (alt maddeler: tipli plan nesneleri; birimli değerler ve “boş ≠ 0 ≠
verilmeyen”; plan notunun bağı; karar çelişkisi, eksik not, üst/alt plan tutarsızlığı sahnede). Durum: taslak.
İlgili: [ADR 0541](0541-data-profiles.md) (veri profilleri), [ADR 0202](0202-topology-rules.md) (topoloji kuralları ve
parça genişliği), [ADR 1360](1360-field-codes-and-kodla-ciz.md) (yerleşik ikinci profil).

## Dayanak

- **Bakanlık:** CBS Genel Müdürlüğü'nün sayfası şunları söyler:
  - çevre düzeni, nazım ve uygulama imar planlarının gösterimi Mekânsal Planlar Yapım Yönetmeliği'nin eklerinde
    standarttır (renk, sınır türü, obje sınıfı, öznitelik, geometri);
  - “Plan GML Doğrulama Yazılımı” ve “veri yapısı ve semboloji dosyaları” projeleri sürmektedir
    ([Plan hazırlama standartları](https://cbs.csb.gov.tr/en/determination-of-standards-for-preparation-of-plans-by-geographic-information-systems-4816),
    6 Kasım 2026'da okundu).

  Bakanlığın XSD'si ve kod listeleri bu makineden edinilemedi.
- **Açık kaynak karşılık (ikincil):** QGIS'in planx_mpyy_studio eklentisi MPYY 1.1.7 UİP/NİP/ÇDP şemalarıyla çalışır
  ([eklenti](https://plugins.qgis.org/plugins/planx_mpyy_studio/), [depo](https://github.com/YusufEminoglu/planx_mpyy_studio)):
  - alan formunda fonksiyon kataloğu, yapı nizamı (A, B, BL), TAKS, KAKS/emsal, yükseklik ve çekme mesafeleri vardır;
  - GML aktarımında şemada olmayan öğe ve kod listesi dışı değer bildirilir, tahmin edilmez;
  - plan kademesi, topoloji ve plan notu denetimleri vardır.
- **Netcad Planet:** plan kademelerine göre çizim araçları sunar ve lejantı plan verisinden üretir
  ([Planet](https://en.netcad.com/en/products/planet)). Plan notu ve değer denetiminin belgesi bulunamadı.
- **Planlı Alanlar İmar Yönetmeliği:** KAKS (emsal), yapı inşaat alanının imar parseli alanına oranıdır. Ayrık ya da
  blok nizamda TAKS planda açıkça verilmemişse %40'ı geçemez (2017 değişikliği; arama dökümünden, resmî metinle
  karşılaştırılmadı). Bu dilim bu kuralı denetlemez: değer planda verildiğinde plan geçerlidir.
- **KentOS bugün:** MPYY sembol kitaplığı vardır (K20). `tucbs-temel`'in tek bir “Planlı alan kullanımı” sınıfı
  vardır (kullanım, emsal, Hmax). Plan, kademe, plan notu ve denetim yoktur.

## Karar

1. **Yerleşik profil `imar-plani`, sürüm 1** (`catalog/v1/profiles/imar-plani.json`; Harita + CBS). KentOS çalışma
   profilidir; temeli (basis) MPYY/PlanGML şeması olmadığını ve eşlenip doğrulanmadan uygunluk iddia edilmeyeceğini
   söyler. Sınıfları:
   - **Plan sınırı:** PlanKimlik, PlanAdi, Seviye (msp, cdp, nip, uip), Durum (taslak, askıda, onaylı, yürürlükte,
     iptal), Sürüm, OnayTarihi, Ölçek, UstPlan, Dayanak.
   - **Karar alanı:**
     - KararKimlik, Plan, Kullanım (17 kod), Nizam (A, İ, B, BL);
     - birimli ve sınırlı değerler: TAKS (0–1), KAKS (0–20), KatAdedi, Hmax (m), Ön/Yan/Arka çekme (m);
     - **Belirtilmeyen** (çoklu: TAKS, KAKS, KatAdedi, Hmax, Cekme) ve PlanNotlari (not numaraları).
   - **Plan notu** (nokta): NotNo, Plan, Metin, Kapsam (genel / alan).
   - **Yol aksı:** YolKimlik, Tür, Genişlik (m).

   Sembol ve renk karar değildir: karar alanın tipli değerlerindedir.
2. **`cad.zoning.check` v1, salt okunur.** Katmanları ve değer listelerini profilden okur. Bulgular, listedeki
   sırasıyla:

   | Bulgu | Ne zaman |
   |---|---|
   | `missingLayer` | Profilin katmanı çizimde yok. |
   | `noLevel` | Plan sınırının kademesi yok ya da listede değil. |
   | `upperMissing` | Üst plan plan sınırları arasında yok. |
   | `levelOrder` | Üst plan bu plandan yukarı bir kademede değil (msp > cdp > nip > uip). |
   | `noPlan` | Karar alanının planı yok, ya da iki plan varken yazılmamış. |
   | `noUse` | Karar alanının kullanımı yok. |
   | `unknownUse` | Kullanım listede değil. |
   | `missingValue` | Üstünde yapılaşan kullanımda TAKS, KAKS ya da yükseklik (kat adedi ya da Hmax) ne verilmiş ne Belirtilmeyen'de. |
   | `declaredValued` | Değer hem verilmiş hem Belirtilmeyen'de. |
   | `kaksReview` | KAKS > TAKS × kat adedi. Hata değil, gözden geçirme: kapalı çıkmalar emsale girebilir. |
   | `overlap` | İki karar aynı yerde (karar çelişkisi). |
   | `uncovered` | Plan sınırının kararsız kısmı. |
   | `outside` | Karar alanı plan dışına taşıyor. |
   | `noteMissing` | Anılan not numarası yok. |
   | `noteUnused` | Kapsamı alan olan notu hiçbir karar anmıyor. |

   - **Boş, 0 ve verilmeyen:** boş değer 0 değildir; 0 bir değerdir; “verilmeyen” ayrı bildirimdir.
   - **Açık kullanımlar:** park, yeşil alan, meydan ve yol değer istemez.
   - **Alan bulguları plan plan hesaplanır:** topoloji çekirdeği (Çakışmamalı, … tarafından kapsanmalı) her planın
     sınırı ve o planı anan kararlar üzerinde çalışır. Nazım planın kendi kararları uygulama planının boşluğu sayılmaz.
     Kararı olmayan plan sınırı yalnız üst plan olarak çizilmiş sayılır. Toleranstan dar parça sayılmaz (ADR 0202 §2).
3. **Pencere İmar planı denetimi…** (`zoning.check`, ikon `zoningCheck`):
   - **Yeri:** Harita'da Plan sekmesinde yeni İmar planı paneli, CBS'de Veri › Standart. Yol ve Mimari sunmaz.
   - **Form ve özet:** tolerans ve profilin adı bir satırda, özet kutusu.
   - **Süzgeç:** Tümü, Plan ve kademe, Karar ve değer, Alan, Plan notu.
   - **Tablo:** Bulgu, Nesne (Plan/Karar kimliği, Not no), Alan, Açıklama.
   - **Seç ve göster:** gösterilen bulguların nesnelerini seçer ve onlara yakınlaştırır (sahnede bulma).
   - **Profili uygula:** katman eksikse görünür; `cad.profiles.apply` ile tek adım yazar.
4. **Yerleşik profiller:** üç oldu (`tucbs-temel`, `harita-varliklari`, `imar-plani`). Profil listesi ve “böyle profil
   yok” iletisi bunu söyler; etkilenen ortak durumlar yeniden üretildi.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/zoning_check_cases.py` 28 durum üretir. Kurallar elle uygulanır; alanlar
  eksene paralel dikdörtgendir, parçalar koordinat sıkıştırmasıyla tam hesaplanır. Durumlar şunları kapsar:
  - temiz plan ve karışık plan (16 bulgu, türün sırası);
  - geniş toleransın dar parçaları saymaması, katman yokluğu;
  - her bulgu türü tek başına, “0 değerdir”, “Hmax yeterlidir”, “park değer istemez”, eşit KAKS;
  - tek planda planı yazılmayan karar, tolerans retleri.

  Rust ve Python geçer. Profil, profil dosyası ve Kodla çiz durumları da geçer.
- **Masaüstü:** `zoning_window_tests.rs` karışık planı, Alan grubunda Seç ve göster'i, temiz planı, Profili uygula'yı
  ve tek adımda geri almayı, modül işaretlerini sınar. Resimler `.run/gui-evidence/imar-denetimi-1420/`.

## Sonra

- **Resmî şema:** Bakanlığın MPYY/PlanGML şemasını ve kod listesini edinip sürümünü sabitlemek. `imar-plani`'yi ona
  ADR 0558 eşlemesiyle bağlamak; şema doğrulaması ile plan denetimini ayrı sonuç olarak vermek.
- **Plan değişikliği:** kaynak revizyona bağlı taslak; yürürlükteki veri doğrudan değişmesin.
- **Ek denetimler:** üst planın sınırının alt planı kapsaması, yol devamlılığı, donatı standartları.
- ~~**Sembol:** MPYY sembolünün karar kodundan seçilmesi (sembol değişikliği kararı değiştirmez).~~ ADR 4240.
