# M0 — Kendi gözlemim (7 Kasım 2026, main e40e6dae0, debug derleme)

Kanıt: `./target/debug/kentos-cad kullan acceptance-{flat,house,change,renovation,curved}` (hepsi "beklentilerin hepsi
tuttu"), resimler `.run/shots/kullanim/masaustu-acceptance-*`; `kentos-cad snapshot` resimleri
`.run/gui-evidence/mimari-durum/01…05`.

## Etkileşim sayıları (iz dosyalarından)
- 2+1 daire (acceptance-flat): 11 araç, 15 odak değişimi, 28 yazma, 48 tuş, 38 tık, 17 fare hareketi ≈ 157 etkileşim.
- İki katlı konut (house): 8 araç, 19 odak, 36 yazma, 52 tuş, 15 tık.
- Ölçülü değişiklik (change): 5 araç, 13 odak, 30 yazma, 44 tuş, 33 tık.
- Seçenekler komut satırından harfle: `E` Enter `0.25` Enter (kalınlık), `U` Enter `4.5` (uzaklık), `B` Enter `between`.
  Komut satırında seçenekler tıklanabilir çip olarak da var (Boy referansı ▾ R, Ortak duvar ▾ O, Diğer, Nokta hesabı).
- İzler çizim yardımlarını kapatıyor (`snap/grid/ortho/polar/tracking: false`) ve araçları kimlikle başlatıyor
  (`run: tool.wallRectangle`): şerit hiç kullanılmıyor, aracı bulma ve kenetleme ölçülmüyor.
- `kullan` oynatıcısı modül seçemiyor: resimler tam `kentos-cad` arayüzü (14 sekme), `kentos-mimari` değil.
- `snapshot --sekme arch` "böyle bir şerit sekmesi yok" diyor: `catalog().tabs()` yalnız CBS sekmelerine bakıyor
  (apps/desktop/src/snapshot.rs:72–76, catalog.rs:1239).

## Çizimin görünüşü (1:100)
- Duvar ince çift çizgi; dolgu/tarama yok, kesilen duvar ile mobilya/yazı aynı ince beyaz çizgi. Dikdörtgen duvar
  aracının varsayılan **Stil: Yok** (05 resmi) — çizimin "mimari" görünmemesinin bir nedeni.
- 54 nesnenin hepsi tek katmanda: **Çizim**. Duvar/kapı/pencere/mahal/ölçü/çizelge katmanları yok (A-WALL vb. yok).
- Mahal etiketi, kapı/pencere etiketi ve ölçü yazıları ekranda çok küçük (kâğıt 2,5 mm × 1:100); `Etiketle (hepsi)`
  sonrası etiket kutuları ve altıgenler duvar çizgilerinin üstüne biniyor.
- Ölçü yalnız alt dış zincir (1.910 · 1.200 · 2.940 …, toplam 11.500); iç ölçü, diğer cepheler yok.
- Çizelgeler model alanında plana yapışık; mahal çizelgesi görünüm altına taşıyor (06).
- Tadilat örneğinde (renovation) yeni bölme ile mevcut duvar aynı görünüyor; evre çizgi dili (mevcut/yıkılacak/yeni)
  ekranda ayırt edilmiyor.
- Merdiven (house, curved): kırma çizgisi ve ok var; "merdiven boşluğu" ve "salon" etiketleri merdivenin üstüne biniyor.

## Arayüz
- Mimari sekmesi (03): Çiz (8), Yerleştir (8), Düzenle (9 + ▾), Belgele (6), Proje (6 + ▾); hepsi küçük düğme, büyük
  düğme yok. Duvar/Dikdörtgen duvar/Ölçülü bölme ikonları ve Mahal/Mahal ayırıcı/Toplu mahal bul/İç nokta seç/Mahal
  kaynakları ikonları birbirine çok benziyor. Mobilya/vitrifiye kataloğu şeritte yok (yalnız Mutfak).
- 1100 × 650'de (açık tema, 07) sekme çubuğu taşıyor: "Çık…" kesik ve etkin bağlam sekmesi **Duvar** ekranda yok.
- Duvar seçiliyken Öznitelikler'de önce genel CAD alanları (Tür, Katman, Renk, Kalınlık, Çizgi tipi, ölçeği,
  Saydamlık, Sembol); duvarın kalınlığı/stili/yüksekliği "Geometri" altında, kaydırmadan görünmüyor.
- Komut geçmişi ham parametre döküyor, Türkçe-İngilizce karışık ve kimlikli: "adopt: evet · arch: level zemin · frame:
  corners · Başvuru: inner · width: 0.25", "extent: between (end 2 alan…) · Ölçü: clear · Yan: left · source: edge 1,
  wall 01a1b609-7b27-…", "cooktop: 4 · sink: 2", "İşlem: distance (uzaklık 3.6, edge 0, yan right)".
- Alt panel sekmeleri CBS ağırlıklı (Koordinat listesi, Noktalar, İstatistik, Grafik, Arama, Adres, Topoloji);
  Mimari için Mahal listesi / Çizelgeler / Denetim bulguları yok (kentos-cad'de; modülde ayrıca bakılmalı).
- İş seçilmemiş çizim açılınca "Bu CAD çizimi hangi iş için?" penceresi (Mimari / Harita / Yol) — iyi.
- Etkin araç ayarları Öznitelikler panelinin içinde (Kat, Stil, Kalınlık, Boy referansı, Ortak duvar, Boylar, Açı);
  komut satırında seçenek çipleri. İmleç yanında sayı kutusu (dinamik girdi) resimlerde görünmedi.
