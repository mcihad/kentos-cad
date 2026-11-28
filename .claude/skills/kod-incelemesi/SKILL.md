---
name: kod-incelemesi
description: KentOS'ta kod incelemesi, denetim ya da refactor planı istendiğinde izlenecek yöntem — kodu okumak, bütün depoda ölçmek, bulguyu dosya ve satırıyla yazmak, alt ajan bulgusunu doğrulamak. "Kodu incele", "review et", "durumu çıkar", "eksikleri bul" gibi isteklerde kullanın.
---

# Kod incelemesi

Kural (ADR 9400 §1): inceleme kodun okunmasından çıkar. Dosya boyutları, bekçi çıktıları ve plan belgeleri tek başına
inceleme değildir; sahip böyle çıkarılmış bir planı reddetti.

1. **Omurgayı baştan sona okuyun.** İlgili alanın giriş dosyaları, durum türleri, dağıtım ve kayıt noktaları. Okunan
   dosyalar raporda sayılır.
2. **Bütün depoda betikle ölçün.** İşlev boyları, tür boyları, kalıp sayımları, `git log --numstat` ile değişim
   sıklığı, `cargo metadata` ile bağımlılık grafiği. Tuzaklar:
   - `git ls-files | xargs wc -l` birden çok `total` satırı basar; tek geçişte (Python) sayın.
   - Sayımda sınama kodunu ayırın: `tests/`, `*_tests.rs`, `*_scenes.rs`, dosya içi `#[cfg(test)]`.
   - Biçim ve boy için `cargo fmt --all -- --check`'i yalnız denetim olarak koşun.
3. **Bulguyu kanıtıyla yazın.** `yol:satır`, kodun ne yaptığı, kaç yerde olduğu, önerilen düzeltme. Önem: doğruluk ve
   veri kaybı, yapı, temizlik.
4. **Alt ajan bulgusu ipucudur.** Kullanıcının açık izni olmadan alt ajan başlatılmaz. İnceleyici salt okunurdur
   (`.claude/agents/inceleyici.md`), ayrık alanlarda çalışır. Her bulgunun andığı satır açılıp doğrulanmadan rapora
   girmez; doğrulanmayan “inceleyiciye göre” diye işaretlenir.
5. **Okunmayanı söyleyin.** Kapsanmayan crate ve klasörler raporda açıkça yazılır.
6. **Düzeltmeleri ayırın.** Önceki bir ölçüm ya da iddia yanlış çıktıysa rapor bunu söyler.

Plan `plan/<ad>.md`'ye (madde kimlikleriyle), kararlar ADR'ye yazılır. Örnek: `plan/bakim.md`, ADR 9400.
