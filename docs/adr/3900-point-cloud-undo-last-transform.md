# 3900 — Nokta bulutunda son dönüşümü geri alma

Tarih: 2026-11-09. plan/map-cad.md MAP-13, yedinci tur. Durum: taslak. ADR 3500'ün “Sonra”sındaki “Geçmişten bir adımı
geri alma (son matrisin tersini kendiliğinden uygulama)”.

İlgili:
- [ADR 3100](3100-apply-a-transform-to-a-point-cloud.md): bulutta dönüşüm uygulama;
- [ADR 3500](3500-point-cloud-inverse-and-history.md): ters matris ve nesnenin dönüşüm geçmişi.

## Dayanak

- **CloudCompare** (ADR 3500'ün okuması, 9 Kasım 2026): her nesne uygulanan dönüşümlerin geçmişini tutar; eski yere
  dönmek için geçmiş matrisinin tersi uygulanır. Kullanıcı tersi kendisi kurar.
- **KentOS bugün** (ADR 3500): Tersini uygula verilen matrisin tersini uygular, geçmişe tersi ekler. Son adımı geri
  almak için matris geçmişten kopyalanıp yazılıyor, Tersini uygula işaretleniyordu; geçmiş geri alınan adımla tersini
  birlikte taşıyordu.

## Karar

1. **Dönüşüm uygula'nın Son dönüşümü geri al'ı** (`undoLast`, öntanımlı kapalı):
   - açıkken matris yazılmaz: Matris ve Tersini uygula gizlenir, denetlenmez;
   - uygulanan matris bulutun **Dönüşümler**'inin son matrisinin tersidir (ADR 3500 §1'in tersi, aynı sıra);
   - yazılan bulutun geçmişinden o adım düşer. Geriye bir şey kalmazsa Dönüşümler yazılmaz; Kaynak kalır;
   - özet, çıktılar ve denetimler (ızgara, katılık, açı) uygulanan matrisindir.
2. **Ret:** geçmişi olmayan bulutta “Bulutun dönüşüm geçmişi (Dönüşümler) yok; geri alınacak adım yok.” Geçmişin son
   matrisi okunmuyorsa okunmayan matrisin sözüyle.
3. Koordinat sistemi değişmez, kaynak dosyaya dokunulmaz (ADR 3100). Noktalar kaynağın ızgarasına yuvarlandığından geri
   alınan bulut ilk dosyanın noktalarıyla bir ızgara adımı içinde aynıdır, bayt bayt değil.
4. **Kapsam işareti:** ADR 1800'deki gibi Harita + CBS.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/pointcloud_processing_cases.py`; laspy, numpy; ters aynı formülle betikte):
  - çizime koy.laz'ı gösteren iki bulut eklendi: geçmişinde tek adım (dönen matris) ve iki adım (dönen matris |
    öteleme);
  - üç yeni durum (47):
    - tek adımın geri alınması: noktalar dönen matrisin tersininki, geçmiş boş, Kaynak koy.laz;
    - iki adımlıda yalnız son ötelemenin geri alınması: ilk adım geçmişte kalır;
    - geçmişsiz bulutta ret;
  - noktaların tam sayıları bayt bayt (FNV-1a) eşleşir; eski 44 durum ve belgenin öntanımları aynı;
  - Rust ve Python (başsız bulut testleri) geçer.
- **MCP** (`crates/native/mcp/tests/cloud_transform_undo.rs`): döndürülen bulutta geri alma, Kaynak'ı kalan ve geçmişi
  boş bir bulut yazar; nokta sayısı durumunki; geçmişsiz bulutta durumun sözüyle ret. ADR 3100 ve 3500'ün MCP testleri
  değişmeden geçer.
- **Masaüstü** (`register_window_tests::son_donusumu_geri_al_takes_the_last_step_back`): kaydı Buluta uygula, sonra
  taşınan taramada Son dönüşümü geri al: geçmiş boş, Kaynak aynı; Tarama kaydı ilk kaydın başlangıç RMS'ini yeniden
  bulur. Tarama kaydının öbür testleri geçer. Resimler `.run/gui-evidence/son-donusum-geri-3900/`.
- **Disk:** derleme sırasında disk doldu (`.rcgu.o` yığını ve ADR 3740'ın sürüm derlemesi); ikisi silindi, 138 GB boşaldı.

## Sonra

- Çoklu tarama demeti (MAP-13'ün öbür maddesi).
- ~~Eksen-açı~~ (ADR 4300) ya da Euler açılarıyla matris girişi (Euler'in sırası kaynakta bekleniyor).
- Gerçek veride süre ve bellek ölçümü.
- Web (TODOS §0.2).
