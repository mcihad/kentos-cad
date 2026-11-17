"""Türkiye imar ve yangın mevzuatının mimari denetimi: başvuru gününde yürürlükte olan kural seti (docs/adr/8078).

KentOS'un standart kural paketi (plan/bim-plan.md K10, BIM-13): kurallar KentOS'un Rust'ta ölçtüğü olguları okur
(``facts.building``, ``facts.permit``), çizimi okumaz. Maddenin hangi hâlinin uygulandığını künyedeki başvuru tarihi
seçer (``yururluk``); gün yoksa ve sonuç ona göre değişiyorsa bulgu "eksik veri"dir. Her bulgu kaldı, eksik veri ya da
uzman incelemesidir; her maddenin özet satırı geçti, kaldı, eksik veri, uzman incelemesi ya da kapsam dışıdır.

- ``asansor``: Planlı Alanlar İmar Yönetmeliği 34/1 ve 34/8 (28.7.2018, 11.7.2021, 1.7.2026 hâlleri), Sivas İmar
  Yönetmeliği 34/1 ve 34/8.
- ``mimari.denetle``: masaüstünün Denetim sekmesi ve başsız denetim tek süreçte bunu çağırır.

Bulgu çizimin ölçüsüdür, onay değildir.
"""
