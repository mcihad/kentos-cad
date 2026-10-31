# ADR 0794: Görünümün alanını kesen haritanın ön denetimi

- **Durum:** kabul edildi (31 Ekim 2026); `plan/architecture-advanced.md` ADV-06'nın altıncı teslimi **[~]**.
- **Bağlam:** ADR 0789'un 1:20 PDF incelemesi bir açık buldu: Banyo detay 1:20'de 190 × 265 mm tutuyor, A3 paftanın harita
  çerçevesi daha alçak; çerçeve görünümün üst ve alt duvarlarını kesiyor ve hiçbir yer bunu söylemiyor.
- **Web:** ertelendi (ADR 0228); satırı TODOS.md §0.2'de.
- **Şema:** değişmez; yeni komut yok.

## Araştırma

- Revit, [Crop a View](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Model/files/GUID-43D2D8F0-1BEB-4E0B-B7B1-1D1CEB9F5D7A.htm)
  ve [Place Views on Sheets](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Model/files/GUID-36B39C71-6FA3-4D97-A31B-6C47B6E8C5E2.htm):
  görünüş paftaya kırpma bölgesi kadar konur; görünüş ölçeği kırpma bölgesinin kâğıttaki boyunu belirler, pafta küçük
  gelirse görünüş paftadan taşar ve kullanıcı ya ölçeği ya kırpmayı değiştirir. Revit bunu bir uyarıyla söylemez, kâğıtta
  görülür.
- KentOS'ta harita çerçevesi sabittir ve tarifin ölçeği haritanındır (ADR 0774 §3): taşan alan sessizce kesilir. Bu yüzden
  ön denetim söyler.

## Karar

- Pafta ön denetiminin (ortak çekirdek `kentos_sheet::preflight`) yeni uyarısı `map_view_cut`: bağlı görünümün tarifi ölçeği
  veriyorsa (`recipe.scale`), görünümün alanı (`width` × `height`, görünümün kendi dönüşünde; harita onu izler) çerçevenin
  içeriğinin o ölçekte tuttuğu zeminden büyükse: ““Harita” “Banyo detay” görünümünün alanını 1:20 ölçeğinde kesiyor: alan
  çerçeveden boyuna 1.14 m büyük.” (yalnız taşan yön yazılır). Çözüm sözü: çerçeveyi büyütün, alanı küçültün (Kırpma çerçevesi, ADR 0789) ya da
  ölçeği küçültün. Kendiliğinden düzeltme yok: üç yoldan hangisinin doğru olduğu tasarımcının kararıdır.
- Ölçeği olmayan tarif görünümün kutusunu çerçeveye sığdırır (ADR 0599): kesilmez, uyarı yok. Tolerans alanın milyonda biri.
- Hesap `view_overrun(width, height, rect, denominator)`; ön denetim panelinde hemen (yerli denetimi beklemeden) görünür.

## Doğrulama

- `cargo test -p kentos-plot --test all map_view_cut`: 400 × 300 m'lik alan 1:20'de uyarı (ad, ölçek, büyüklük), 0,5 × 0,4 m
  ve ölçeksiz tarif uyarı vermez.
- Resim: `KENTOS_SHOTS_ONLY=kirpma-kesik-denetim cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored
  --nocapture` (`.run/shots/arac-kirpma-kesik-denetim-*`): Banyo detay 1:20, Ön denetim sekmesinde uyarı.

## Sınırlar

- Dönük çerçevede (haritanın kendi dönüşü görünümünkinden farklıysa) karşılaştırma görünümün ekseninde yapılır.
- Gerçek pencerede elle denenmedi.
