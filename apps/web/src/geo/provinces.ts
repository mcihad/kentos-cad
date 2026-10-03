/**
 * Türkiye's 81 provinces with the centre of each provincial capital (docs/adr/0165 §3): a new project's start view
 * and the TM zone it suggests. The positions are the city centres to about a hundredth of a degree, for a view and
 * a zone, never a measurement. The single source of the list: fixtures/crs/v1/provinces.json is written from it
 * (scripts/fixtures/record-crs.test.ts) for the desktop.
 */

export interface Province {
  /** The licence plate code, 1–81. */
  readonly code: number;
  readonly name: string;
  /** Degrees north. */
  readonly lat: number;
  /** Degrees east. */
  readonly lon: number;
}

const p = (code: number, name: string, lat: number, lon: number): Province => ({ code, name, lat, lon });

export const PROVINCES: readonly Province[] = [
  p(1, 'Adana', 37.0, 35.3213),
  p(2, 'Adıyaman', 37.7648, 38.2786),
  p(3, 'Afyonkarahisar', 38.7507, 30.5567),
  p(4, 'Ağrı', 39.7191, 43.0503),
  p(5, 'Amasya', 40.6499, 35.8353),
  p(6, 'Ankara', 39.9334, 32.8597),
  p(7, 'Antalya', 36.8969, 30.7133),
  p(8, 'Artvin', 41.1828, 41.8183),
  p(9, 'Aydın', 37.856, 27.8416),
  p(10, 'Balıkesir', 39.6484, 27.8826),
  p(11, 'Bilecik', 40.1451, 29.9799),
  p(12, 'Bingöl', 38.8854, 40.498),
  p(13, 'Bitlis', 38.4006, 42.1095),
  p(14, 'Bolu', 40.735, 31.6061),
  p(15, 'Burdur', 37.7203, 30.2908),
  p(16, 'Bursa', 40.1885, 29.061),
  p(17, 'Çanakkale', 40.1553, 26.4142),
  p(18, 'Çankırı', 40.6013, 33.6134),
  p(19, 'Çorum', 40.5506, 34.9556),
  p(20, 'Denizli', 37.7765, 29.0864),
  p(21, 'Diyarbakır', 37.9144, 40.2306),
  p(22, 'Edirne', 41.6818, 26.5623),
  p(23, 'Elazığ', 38.681, 39.2264),
  p(24, 'Erzincan', 39.75, 39.5),
  p(25, 'Erzurum', 39.9, 41.27),
  p(26, 'Eskişehir', 39.7767, 30.5206),
  p(27, 'Gaziantep', 37.0662, 37.3833),
  p(28, 'Giresun', 40.9128, 38.3895),
  p(29, 'Gümüşhane', 40.4386, 39.5086),
  p(30, 'Hakkari', 37.5833, 43.7333),
  p(31, 'Hatay', 36.2021, 36.16),
  p(32, 'Isparta', 37.7648, 30.5566),
  p(33, 'Mersin', 36.8, 34.6333),
  p(34, 'İstanbul', 41.0082, 28.9784),
  p(35, 'İzmir', 38.4192, 27.1287),
  p(36, 'Kars', 40.6167, 43.1),
  p(37, 'Kastamonu', 41.3887, 33.7827),
  p(38, 'Kayseri', 38.7312, 35.4787),
  p(39, 'Kırklareli', 41.7333, 27.2167),
  p(40, 'Kırşehir', 39.1425, 34.1709),
  p(41, 'Kocaeli', 40.8533, 29.8815),
  p(42, 'Konya', 37.8667, 32.4833),
  p(43, 'Kütahya', 39.4167, 29.9833),
  p(44, 'Malatya', 38.3552, 38.3095),
  p(45, 'Manisa', 38.6191, 27.4289),
  p(46, 'Kahramanmaraş', 37.5858, 36.9371),
  p(47, 'Mardin', 37.3212, 40.7245),
  p(48, 'Muğla', 37.2153, 28.3636),
  p(49, 'Muş', 38.9462, 41.7539),
  p(50, 'Nevşehir', 38.6939, 34.6857),
  p(51, 'Niğde', 37.9667, 34.6833),
  p(52, 'Ordu', 40.9839, 37.8764),
  p(53, 'Rize', 41.0201, 40.5234),
  p(54, 'Sakarya', 40.694, 30.4358),
  p(55, 'Samsun', 41.2928, 36.3313),
  p(56, 'Siirt', 37.9333, 41.95),
  p(57, 'Sinop', 42.0231, 35.1531),
  p(58, 'Sivas', 39.7477, 37.0179),
  p(59, 'Tekirdağ', 40.9833, 27.5167),
  p(60, 'Tokat', 40.3167, 36.55),
  p(61, 'Trabzon', 41.0015, 39.7178),
  p(62, 'Tunceli', 39.1079, 39.5401),
  p(63, 'Şanlıurfa', 37.1591, 38.7969),
  p(64, 'Uşak', 38.6823, 29.4082),
  p(65, 'Van', 38.4891, 43.4089),
  p(66, 'Yozgat', 39.8181, 34.8147),
  p(67, 'Zonguldak', 41.4564, 31.7987),
  p(68, 'Aksaray', 38.3687, 34.037),
  p(69, 'Bayburt', 40.2552, 40.2249),
  p(70, 'Karaman', 37.1759, 33.2287),
  p(71, 'Kırıkkale', 39.8468, 33.5153),
  p(72, 'Batman', 37.8812, 41.1351),
  p(73, 'Şırnak', 37.5164, 42.4611),
  p(74, 'Bartın', 41.6344, 32.3375),
  p(75, 'Ardahan', 41.1105, 42.7022),
  p(76, 'Iğdır', 39.9237, 44.045),
  p(77, 'Yalova', 40.65, 29.2667),
  p(78, 'Karabük', 41.2061, 32.6204),
  p(79, 'Kilis', 36.7184, 37.1212),
  p(80, 'Osmaniye', 37.0742, 36.2464),
  p(81, 'Düzce', 40.8438, 31.1565),
];

/** The province of a plate code, or none. */
export const provinceByCode = (code: number): Province | undefined => PROVINCES.find((x) => x.code === code);

/** Provinces whose name starts or holds the query, Turkish case folded (“iz” finds İzmir), or whose plate code it is; all for none. */
export function searchProvinces(query: string): Province[] {
  const q = query.trim().toLocaleLowerCase('tr-TR');
  if (!q) return [...PROVINCES];
  if (/^\d+$/.test(q)) {
    // A plate code: the exact one first, then those it begins (“3”: 3, 30–39; “06”: Ankara).
    const begins = PROVINCES.filter((x) => String(x.code).startsWith(q) || String(x.code).padStart(2, '0').startsWith(q));
    return [...begins.filter((x) => x.code === Number(q)), ...begins.filter((x) => x.code !== Number(q))];
  }
  const starts = PROVINCES.filter((x) => x.name.toLocaleLowerCase('tr-TR').startsWith(q));
  const holds = PROVINCES.filter((x) => !starts.includes(x) && x.name.toLocaleLowerCase('tr-TR').includes(q));
  return [...starts, ...holds];
}

/** The list as the file shared with the desktop holds it (fixtures/crs/v1/provinces.json). */
export function provincesFixture() {
  return {
    format: 'kentos.provinces',
    version: 1,
    source: 'src/geo/provinces.ts',
    note: 'İl merkezlerinin yaklaşık konumu (derece, yüzde bir): yeni projenin başlangıç görünümü ve önerilen dilim içindir, ölçü değildir.',
    provinces: PROVINCES,
  };
}
