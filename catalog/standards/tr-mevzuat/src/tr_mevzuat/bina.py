"""Kuralların ortak okuduğu: binanın katları ve konutları (BYKHY 48/1'in muafiyeti), olguların hangi KentOS'tan geldiği,
yağmurlama ayarı, adların ve ölçülerin yazımı."""

from __future__ import annotations

from dataclasses import dataclass

from kentos import ext

#: Ölçülerin payı: olgular alanı 0.0001 m²'ye, uzunluğu 1 mm'ye yuvarlar; sınırdaki ölçü sınırı aşmış sayılmaz.
PAY = 1e-6


def metre(v: float) -> str:
    """Bir uzunluk, iki basamakla: “22.52 m”."""
    return f"{v:.2f} m"


def metrekare(v: float) -> str:
    """Bir alan, iki basamakla: “12.00 m²”."""
    return f"{v:.2f} m²"


def mahal(r: ext.RoomFacts) -> str:
    """Bir mahal, numarası ve adıyla: “Z02 Salon”."""
    parts = [p for p in (r.number, r.name) if p]
    return " ".join(parts) if parts else "Adsız mahal"


def bolum(u: ext.UnitFacts) -> str:
    """Bir bağımsız bölüm, numarası ve niteliğiyle: “1 numaralı bağımsız bölüm (Daire)”."""
    text = f"{u.number} numaralı bağımsız bölüm"
    return f"{text} ({u.nature})" if u.nature else text


def kat_adlari(a: ext.ArchitectureFacts) -> dict[str | None, str]:
    """Katların adları, kimlikleriyle; katsız mahallerin “katsız”."""
    names: dict[str | None, str] = {s.id: s.name or s.id for s in a.levels}
    names[None] = "katsız"
    return names


def konutlar(a: ext.ArchitectureFacts) -> dict[str, ext.UnitFacts]:
    """Konut olan bağımsız bölümler, kimlikleriyle (olguların ``dwelling``'i; eklenti ve ortak alan değil)."""
    return {u.id: u for u in a.units if u.kind == "unit" and u.dwelling}


def yeni_olgular(a: ext.ArchitectureFacts) -> bool:
    """Olgular mahalin türünü, dar kenarını, yağmurlama ayarını ve merdivene açılan kapıyı veren bir KentOS'tan mı
    (ADR 8057 §53): o KentOS sınırı bulunan her mahalin dar kenarını yazar. Eskisinin olgularında bu alanlar boş ya da
    ``False`` okunur; projenin yağmurlama ayarı ve kapının merdivene açıldığı o zaman bilinmez."""
    return any(r.narrow is not None for r in a.rooms)


@dataclass(frozen=True)
class Yagmurlama:
    """BYKHY 48/2'nin ve Ek-5/B'nin hangi sütunu okunur: projenin ayarı biliniyorsa onunki (``sutun`` 0: yağmurlama
    sistemi yok, 1: var), bilinmiyorsa ikisi de (``sutun`` None: yağmurlamasız sınırı aşan uyarı, iki sınırı da aşan
    hata); ``soz`` bulgunun söylediği."""

    sutun: int | None
    soz: str


def yagmurlama(a: ext.ArchitectureFacts) -> Yagmurlama:
    """Projenin yağmurlama ayarı (KCAD 226). Ayar verilmemiş proje yağmurlama sistemsiz sayılır: ayarın varsayılanı
    budur, KentOS Mimari'nin yangın denetimi de böyle okur ve olgular ayarı yalnız işaretliyse ``True`` yazar. Olgular
    ayarı vermeyen eski bir KentOS'tansa ayar bilinmez."""
    if not yeni_olgular(a):
        return Yagmurlama(None, "olgularda projenin yağmurlama ayarı yok (eski KentOS)")
    if a.sprinklers:
        return Yagmurlama(1, "projenin ayarına göre binada yağmurlama sistemi var")
    return Yagmurlama(0, "projenin ayarında yağmurlama sistemi işaretli değil")


def karar(measure: float, limits: tuple[float, float], y: Yagmurlama) -> tuple[str, float, str] | None:
    """Bir ölçünün iki sütunlu sınıra (yağmurlama sistemi yok, var) göre kararı: önemi, aştığı sınır ve söz; aşmıyorsa
    yok. Ayar biliniyorsa yalnız onun sınırı (aşarsa hata); bilinmiyorsa yağmurlamasız sınırı aşan uyarı, yağmurlamalıyı
    da aşan hata."""
    if y.sutun is not None:
        limit = limits[y.sutun]
        return ("error", limit, f"{y.soz}, sınır {metre(limit)}") if measure > limit + PAY else None
    if measure > limits[1] + PAY:
        return ("error", limits[1], f"{y.soz}; yağmurlama sistemi olsa da sınır ({metre(limits[1])}) aşılıyor")
    if measure > limits[0] + PAY:
        return ("warning", limits[0], f"{y.soz}; binada yağmurlama sistemi yoksa sınır ({metre(limits[0])}) aşılıyor")
    return None


@dataclass(frozen=True)
class Bina:
    """BYKHY 48/1'in okuduğu: katların sayısı (zemin ve üstü, mahali olan bodrumlar) ve konut bağımsız bölümleri."""

    katlar: int
    konutlar: int

    @property
    def kacis_mesafesi_aranmaz(self) -> bool:
        """BYKHY 48/1: bodrum katlar dahil 4 katı geçmeyen konutlarda ve tek evlerde kaçış mesafesi aranmaz."""
        return self.konutlar > 0 and (self.katlar <= 4 or self.konutlar == 1)


def bina(a: ext.ArchitectureFacts) -> Bina:
    """Binanın katları ve konutları: ±0.00 ve üstündeki her kat, altındakilerden mahali olanlar (KentOS Mimari'nin
    yangın denetiminin saydığı gibi); konut, olguların konut dediği bağımsız bölüm."""
    used = {r.level for r in a.rooms}
    storeys = sum(1 for s in a.levels if s.elevation >= -PAY or s.id in used)
    return Bina(max(storeys, 1), len(konutlar(a)))
