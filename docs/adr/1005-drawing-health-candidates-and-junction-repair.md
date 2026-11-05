# ADR 1005: Çizim sağlığı adayları ve T birleşimi onarımı

- **Durum:** kabul edildi (2026-11-05). Kapsam: plan/cad-advanced.md `CAD-ADV-18`'in “küçük boşluk, T birleşiminde
  taşma/kısa kalma, istenmeyen açı sapması, uzak gereksiz nesne ve yinelenen blok tanımı için aday denetim” ve
  “ölçülmüş veya kısıtlı geometriyi otomatik düzelten genel rutin olmasın; maksimum yer değişimi, alan/uzunluk farkı
  raporlansın” işleri. Maddenin tamamı değildir (§5).
- **Bağlam belgesi:** ADR 0849 (`cad.drawing.inspect`), BricsCAD OPTIMIZE/DWGHEALTH (plan [R14], [R15]).

## 1. Sağlık grubu (`checks: ["health"]`)

`cad.drawing.inspect`'e yalnız adıyla istenen `health` grubu eklendi; `checks` boşken çalışan varsayılan denetimler
aynı kalır (eski cevaplar değişmez). Bulgular çizim sırasıyla:

- `t_junction_short` / `t_junction_overshoot`: bir çizginin serbest ucu (başka doğru kenarın ucu 1e-9 içinde değil),
  kendi doğrultusunda başka bir nesnenin doğru kenarının **içine** en çok `maxGap` uzaklıkta ulaşmıyor ya da onu
  geçiyor (geçen kısım çizginin içinde). Noktalar uç ve kesişim, `distance` uzaklık.
- `angle_deviation`: yatay ya da düşeyden `maxAngle` (derece, 0–10, varsayılan 0,5; 0 kapatır) içinde ama üstünde
  olmayan kenar; `distance` uzak ucun kayması (boy × sin sapma).
- `stray_object`: en az on nesnede, merkezi (noktalarının ortalaması) 10.–90. yüzdelik merkez kutusundan, kutunun
  köşegeninin on katından (köşegen en az 0,1 m) uzak nesne. Yüzdelik, sıralı listede `round((n−1)·q)` sırası.
- `block_definition_duplicate`: parçaları (kimlikleri dışarıda, sırasız), taban noktası ve öznitelikleri aynı,
  adları farklı blok tanımları (dış başvurular hariç).

Bulgular aday ve bilgidir; hiçbiri çizimi değiştirmez. Geçersiz açı eşiği `invalid_angle_tolerance`.

## 2. `cad.drawing.repair` v1 (Onar)

İncelenmiş T birleşimini düzeltir: çizginin ucu kendi doğrultusunda hedefin en yakın doğru kenarının içine, en çok
`maxGap` kadar taşınır. Bir kısıtın ya da ilişkili ölçünün okuduğu çizgi taşınmaz (`repair_refused_measured`);
kilitli katman reddeder; çizgi dışındaki nesne ve kendi kenarı `repair_unsupported`; uzantı eşik içinde değmiyorsa
`repair_target_missing`. Hepsi ya da hiçbiri; tek Undo “Onar”. Çıktı her uç için kayma ve çizginin önceki/sonraki
boyu, en büyük kayma. Masaüstünde CAD denetimi › Çizim sağlığı seçili bulguda **Ucu kenara getir** bu komutu
denetimin eşiğiyle çalıştırır ve denetimi yeniler.

## 3. Doğrulama

`scripts/fixtures/drawing_repair_cases.py` KentOS kodu kullanmadan 21 sağlık ve 24 onarım ortak durumu yazar;
düzeltilen tek fark bulgu sırasıydı (Rust önce kimlik sırasıyla geziyordu; çizim sırası seçildi). Gerçek pencerede
denetim 7 bulgu verdi, onarımdan sonra 6; görüntüler repo dışında.

## 4. Kurtarma ve kısmi okuma

Bu ADR masaüstü kurtarma kopyasına (`apps/desktop/src/recovery.rs`) ve kısmi okunabilen dosyadan veri çıkarmaya dokunmaz.

## 5. Sınırlar (CAD-ADV-18 açık kalanı)

Açılışta isteğe bağlı salt okunur sağlık denetimi (süre/bellek bütçesi, aynı revizyon için önceki sonucun tekrar
kullanımı), onarım tarifinin sıra/eşik/kapsamla kaydı ve öbür bulgu türleri için onarım, gerçek çökme/disk dolu/kesilen
kayıt senaryolarıyla kurtarma kabulü ve kısmi okuma yolu açıktır. Polyline uçları onarılmaz (yalnız çizgi).
