# ADR 9142: Kapalı oturumda hücre nedenini ve yolunu söyler; tablo yüklenen satırı toplamdan ayırır

- **Durum:** kabul edildi (2026-11-22); adım VY-41. Plan: [plan/veri.md](../../plan/veri.md) VY-41.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  9. maddesi (P1):
  - oturum kapalıyken hücreye çift tıklamak küçük ve kesilen bir durum iletisine yol açtı;
  - geniş tabloda ve seçim kümelerinde yüklenen satır ile toplam satır ayrı görünmeli.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Veri katmanının tablosunda (ADR 9102 §5) düzenleme oturumu kapalıyken çift tıklama uzun bir cümleyi yalnız durum
  satırına yazıyordu. Satır dar olduğundan kesiliyordu.
- Çizim araçları aynı durumu bir bildirimle ve “Düzenlemeyi başlat” düğmesiyle söylüyordu (ADR 9128).
- Tablonun satırı kaynağın toplam sayısını yazıyordu. Sayfalar okundukça yüklenen satırlar ondan azdır ve bu
  görünmüyordu.

## §2 Karar

1. **Kapalı oturumda hücre.** Çift tıklama hücreyi açmaz. Neden hem durum satırına hem tam okunan bir bildirime yazılır:
   “… düzenlemeye açık değil; bir değeri değiştirmek için önce katmanın düzenleme oturumunu başlatın.” Bildirimin
   “Düzenlemeyi başlat” düğmesi oturumu açar; aynı çift tıklama ondan sonra hücreyi düzenler.
2. **Yüklenen ve toplam.** Okunan satırlar toplamdan azken tablonun satırı ikisini birlikte söyler: “450 satır (200
   yüklendi) · süzgeç ve sıra kaynakta”.

## §3 Kanıt

- **Masaüstü** (`gis/features/closed_cell_tests.rs`):
  - raporun `bakim` tablosunda oturum kapalıyken çift tıklama hücreyi açmıyor, bildirim düğmeyi sunuyor;
  - düğmeye basılınca oturum açılıyor, hücre düzenleniyor;
  - 450 satırlık tabloda satır “450 satır (200 yüklendi)” diyor.
- Öznitelik tablosu, veri katmanı, düzenleme, katman hedefi ve iz sınamaları (52) geçiyor.

## §4 Kalan

- İki sağlayıcıda (proje içi tablo ile veri tablosu) aynı araç düzeni ve terimler: Tablo/Form geçişi, sütun gizleme ve
  anahtarın gösterilişi veri tablosunda yok.
- Kayıt seç → haritada bul → bilgi kartı → form → süz → seçimi temizle zincirinin tek sınaması; boş metin, NULL, 0,
  kodlu değer ve Türkçe arama/sıralama örnekleri.
- Web (TODOS.md §0.2).
