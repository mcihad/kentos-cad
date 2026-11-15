"""Türkiye mevzuatının mimari çizimdeki denetim kuralları (KentOS'un birinci taraf eklentisi).

Kurallar yalnız KentOS'un Rust'ta ölçtüğü olguları okur (``facts.project.architecture``: katlar, kat çıkışları,
mahallerin türü, net alanı ve dar kenarı, bağımsız bölümler, kaçış yürüyüşleri, projenin yağmurlama ayarı) ve yalnız
olguların kesin söylediğine karar verir; yönetmeliğin istisnası olguların göremediği bir şeye bağlıysa karar
verilemediğini söyler. Kaynaklar:

- Planlı Alanlar İmar Yönetmeliği (PAİY), RG 3.7.2017/30113: ``paiy`` ve ``piyes_eksik``.
- Binaların Yangından Korunması Hakkında Yönetmelik (BYKHY), RG 19.12.2007/26735: ``bykhy``.

Her bulgu yönetmeliğin maddesini ve fıkrasını söyler; bulgu çizimin ölçüsüdür, onay değildir.
"""
