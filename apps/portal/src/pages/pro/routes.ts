// A professional's pages on the portal (docs/adr/9000 §8).
import { dossierPage } from '../dossier/view';
import type { PageFn } from '../page';
import { proNewApplication } from './apply';
import { proDashboard, proDossiers } from './lists';
import { proProfile, proProjects } from './profile';

export const proPages: [string, PageFn][] = [
  ['mesleki', proDashboard],
  ['mesleki/basvurular', proDossiers('application')],
  ['mesleki/basvurular/yeni', proNewApplication],
  ['mesleki/isler', proDossiers('transfer')],
  ['mesleki/projeler', proProjects],
  ['mesleki/profil', proProfile],
  ['mesleki/dosya/:id', dossierPage('professional')],
];
