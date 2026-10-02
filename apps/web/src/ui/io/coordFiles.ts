import type { FileKind } from '../../app/fileIO';

/**
 * The files a coordinate list import offers (Netcad NCN, TXT, CSV and their kin). Apart from the windows, which load
 * on first use: the command and Noktalar's İçe aktar pick the file at the click itself (the browser's file dialog
 * needs the user's gesture).
 */
export const COORD_FILES: FileKind = { description: 'Koordinat listesi (NCN, TXT, CSV)', accept: { 'text/plain': ['.ncn', '.txt', '.csv', '.xyz', '.dat', '.asc'] } };
