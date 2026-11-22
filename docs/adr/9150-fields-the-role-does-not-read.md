# ADR 9150: Rolün okumadığı alan boş gösterilmez, adıyla söylenir

- **Durum:** kabul edildi (2026-11-23); adım VY-49. Plan: [plan/veri.md](../../plan/veri.md) VY-49.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  29. maddesi (P2):
  - Veri girişi rolüne dönen kayıtta yalnız `Durum` ve `Bolge` vardı;
  - alt tablo yine `Not` başlığını boş hücreyle gösterdi, oysa sahip ve görüntüleyici “Teknik ekip notu” görüyordu;
  - saha çalışanı kayıtta not olmadığını sanabilir;
  - istenen:
    - alan listesinde rolün ve yayımlanmış formun okunabilir alanlarını uygulamak;
    - gerçek NULL, boş metin ve rol yüzünden gönderilmeyen değeri ayırmak;
    - tablo, bilgi kartı ve dışa aktarımda aynı kümeyi kullanmak.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Sunucu, formlar üzerinden çalışan çağırana (Veri girişi) katmanın canlı formlarının göstermediği alanları göndermez
  (`form_guard::projected`, ADR 0411 §6). Kural ortak çözümleyicidedir (`kentos_forms::reach`, ADR 0550): alanın düzeyi
  `Hidden`, `Redacted` ya da `Open`'dır.
- Masaüstünde böyle bir çağıranın belgesi yalnız öznitelik yazar (`ObjectPolicy::AttributesOnly`).
- Öznitelik tablosu sütunlarını katmanın alanlarından ve nesnelerin özniteliklerinden kuruyordu. Gönderilmeyen alan
  katmanın alanı olduğu için boş sütun olarak görünüyordu.
- **Araştırma.** Başvuru alınan bulut CBS'sinde (23 Kasım 2026'da okundu) bir katmanın görünümünden alanlar çıkarılabilir:
  görünümü kullananların erişmesi gerekmeyen alanlar (kişi adı, sağlık bilgisi gibi) görünüme hiç konmaz. Kullanan, alanı
  boş bir değer olarak değil, hiç almaz.

  Kaynak: [barındırılan görünümler](https://doc.esri.com/en/arcgis-online/latest/manage-data/create-hosted-views.html).

## §2 Karar

1. **Tek kural.** Masaüstü formlar üzerinden çalışan çağıran için katmanın alanlarını ortak çözümleyiciyle ayırır
   (`withheld_fields`): canlı formların göstermediği alanlar okunmaz. Başka herkes için liste boştur.
2. **Tablo.**
   - Okunmayan alan sütun değildir. Nesnelerin özniteliklerinde duruyorsa da (rol değişmeden önce okunmuş bir kopya)
     sütunu ve hücreleri kalkar.
   - Tablonun alt satırı adını söyler: “Rolünüz bu alanları okumuyor, gösterilmiyor: Not”.
   - Kopyalama, dosyaya yazma ve istatistik tablonun görünen sütunlarından gider: okunmayan alan onlara da girmez.
3. **Bilgi kartı.** Okunmayan alan satır değildir; kartın sonunda “Rolünüz okumuyor: Not” satırı adını söyler.

## §3 Kanıt

- **Masaüstü** (`gis/features/role_fields_tests.rs`), binaların yayımlanmış formu (Durum, Kat, Açıklama, Tarih,
  Onaylı) ile formun göstermediği `Not` alanında:
  - sahip `Not` sütununu görüyor;
  - Veri girişi görmüyor, alt satır `Not`'u adıyla söylüyor;
  - kart `Not`'u satır olarak göstermiyor, “Rolünüz okumuyor: Not” diyor.
- Öznitelik tablosu, Bilgi kartı, form rolleri ve form sahneleri sınamaları (147) geçiyor.
- **Resim** (`.run/gui-evidence/kullanim-9150/`, Git'e girmez): Veri girişinin tablosu ve alt satırı.

## §4 Kalan

- **NULL ile boş metin.** Tabloda gerçek NULL ile boş metin hücresi hâlâ aynı görünüyor; ayrı bir işaret ister.
- **Veri katmanı tablosu.** Kaynakta duran satırların tablosunda (ADR 9102) rolün alan süzgeci uygulanmadı.
- **Gerçek iki hesap.** Bulutta üç hesapla kabul koşturulmadı. Sunucu tarafı yetki sınamaları önceden geçiyor.
- Web (TODOS.md §0.2).
