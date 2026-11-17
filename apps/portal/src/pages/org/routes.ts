// A municipality's pages on the portal (docs/adr/9000 §8).
import { dossierPage } from '../dossier/view';
import type { PageFn } from '../page';
import { orgDashboard, orgDossiers } from './lists';
import { orgDirectory, orgMembers, orgSettings } from './people';
import { orgNewTransfer } from './transfer';
import { officeDashboard, officeDossiers } from './office';
import { orgBySlug } from '../../app/state';
import { orgProjects } from '../projects';
import { orgFeeSchedule, orgFees } from './fees';

/** A municipality's page or an office's, by the organisation's category (docs/adr/9000 §18). */
const byCategory = (municipality: PageFn, office: PageFn): PageFn => (ctx) =>
  orgBySlug(ctx.params.slug)?.category === 'office' ? office(ctx) : municipality(ctx);

export const orgPages: [string, PageFn][] = [
  ['kurum/:slug', byCategory(orgDashboard, officeDashboard)],
  ['kurum/:slug/basvurular', byCategory(orgDossiers('application'), officeDossiers('application'))],
  ['kurum/:slug/isler', officeDossiers('transfer')],
  ['kurum/:slug/aktarimlar', orgDossiers('transfer')],
  ['kurum/:slug/aktarimlar/yeni', orgNewTransfer],
  ['kurum/:slug/meslek-mensuplari', orgDirectory],
  ['kurum/:slug/projeler', orgProjects],
  ['kurum/:slug/harclar', orgFees],
  ['kurum/:slug/harclar/:id', orgFeeSchedule],
  ['kurum/:slug/uyeler', orgMembers],
  ['kurum/:slug/ayarlar', orgSettings],
  ['kurum/:slug/dosya/:id', byCategory(dossierPage('municipality'), dossierPage('office'))],
];
