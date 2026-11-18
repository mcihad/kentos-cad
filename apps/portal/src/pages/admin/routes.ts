// The platform administrators' pages on the portal (docs/adr/9000 §8).
import type { PageFn } from '../page';
import { adminDashboard, adminEvents, adminSettings } from './overview';
import { adminProfessionals, adminUsers } from './people';
import { adminTenant, adminTenants } from './tenants';

export const adminPages: [string, PageFn][] = [
  ['sistem', adminDashboard],
  ['sistem/kurumlar', adminTenants],
  ['sistem/kurumlar/:id', adminTenant],
  ['sistem/meslek-mensuplari', adminProfessionals],
  ['sistem/hesaplar', adminUsers],
  ['sistem/kayit', adminEvents],
  ['sistem/ayarlar', adminSettings],
];
