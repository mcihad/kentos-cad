// A municipality's pages on the portal (docs/adr/9000 §8).
import { dossierPage } from '../dossier/view';
import type { PageFn } from '../page';
import { orgDashboard, orgDossiers } from './lists';
import { orgDirectory, orgMembers, orgSettings } from './people';
import { orgNewTransfer } from './transfer';
import { orgProjects } from '../projects';

export const orgPages: [string, PageFn][] = [
  ['kurum/:slug', orgDashboard],
  ['kurum/:slug/basvurular', orgDossiers('application')],
  ['kurum/:slug/aktarimlar', orgDossiers('transfer')],
  ['kurum/:slug/aktarimlar/yeni', orgNewTransfer],
  ['kurum/:slug/meslek-mensuplari', orgDirectory],
  ['kurum/:slug/projeler', orgProjects],
  ['kurum/:slug/uyeler', orgMembers],
  ['kurum/:slug/ayarlar', orgSettings],
  ['kurum/:slug/dosya/:id', dossierPage('municipality')],
];
