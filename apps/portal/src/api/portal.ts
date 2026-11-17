// The portal's endpoints (docs/adr/9000 §8), one function each, typed by the generated contracts.
import { del, get, patch, post, put, query, upload } from './client';
import type {
  ApplicationCreate,
  DossierActionRequest,
  DossierDashboard,
  DossierDetail,
  DossierDirection,
  DossierEdit,
  DossierFileImported,
  DossierFileView,
  DossierList,
  DossierMessageView,
  MemberView,
  MunicipalityCard,
  Notifications,
  OrganizationPortal,
  PlatformEventView,
  PlatformSummary,
  PlatformTenant,
  PlatformTenantDetail,
  PlatformUser,
  PortalCatalog,
  PortalMe,
  ProfessionalCard,
  ProfessionalStatus,
  ProfessionalView,
  ProjectPage,
  Signup,
  TenantRole,
  TransferCreate,
} from './types';

export type Created = { id: string };

/** What a list of dossiers asks for. */
export type ListAsk = {
  direction?: DossierDirection;
  status?: string;
  q?: string;
  mine?: boolean;
  awaiting?: boolean;
  limit?: number;
  offset?: number;
};

export const portal = {
  me: () => get<PortalMe>('/v1/portal/me'),
  catalog: () => get<PortalCatalog>('/v1/portal/catalog'),
  login: (login: string, password: string) => post<unknown>('/v1/auth/login', { login, password }),
  logout: () => post<unknown>('/v1/auth/logout'),
  signupInfo: () => get<{ open: boolean }>('/v1/portal/signup'),
  signUp: (input: Signup) => post<PortalMe>('/v1/portal/signup', input),
  editProfessional: (input: { officeName: string | null; phone: string | null; province: string | null; listed: boolean }) =>
    put<PortalMe>('/v1/portal/me/professional', input),
  changePassword: (current: string, next: string) => post<void>('/v1/portal/me/password', { current, new: next }),
  municipalities: () => get<MunicipalityCard[]>('/v1/portal/municipalities'),
  notifications: (unread = false) => get<Notifications>(`/v1/me/notifications${query({ unread: unread || undefined })}`),
  readNotifications: (ids: string[], all = false) => post<Notifications>('/v1/me/notifications/read', { ids, all }),
  projects: (view: 'mine' | 'organization', tenant?: string) =>
    get<ProjectPage>(`/v1/me/catalog${query({ view, tenant, sort: 'updated', limit: 100 })}`),
};

export const professional = {
  dossiers: (ask: ListAsk) => get<DossierList>(`/v1/me/dossiers${query(ask)}`),
  summary: () => get<DossierDashboard>('/v1/me/dossiers/summary'),
  apply: (input: ApplicationCreate) => post<Created>('/v1/me/dossiers', input),
};

export const organization = {
  portal: (tenant: string) => get<OrganizationPortal>(`/v1/tenants/${tenant}/portal`),
  setPortal: (
    tenant: string,
    input: { acceptsApplications: boolean; applicationKinds: string[]; contactEmail: string | null; contactPhone: string | null; address: string | null },
  ) => put<OrganizationPortal>(`/v1/tenants/${tenant}/portal`, input),
  summary: (tenant: string) => get<DossierDashboard>(`/v1/tenants/${tenant}/portal/summary`),
  dossiers: (tenant: string, ask: ListAsk) => get<DossierList>(`/v1/tenants/${tenant}/dossiers${query(ask)}`),
  transfer: (tenant: string, input: TransferCreate) => post<Created>(`/v1/tenants/${tenant}/dossiers`, input),
  directory: (tenant: string, q?: string, profession?: string) =>
    get<ProfessionalCard[]>(`/v1/tenants/${tenant}/professionals${query({ q, profession })}`),
  members: (tenant: string) => get<MemberView[]>(`/v1/tenants/${tenant}/members`),
  addMember: (tenant: string, input: { login: string; password: string; displayName: string; email: string | null; role: TenantRole }) =>
    post<MemberView[]>(`/v1/tenants/${tenant}/members`, input),
  updateMember: (tenant: string, user: string, input: { role: TenantRole; status: string; seat: boolean }) =>
    patch<MemberView[]>(`/v1/tenants/${tenant}/members/${user}`, input),
};

export const dossier = {
  get: (id: string) => get<DossierDetail>(`/v1/dossiers/${id}`),
  edit: (id: string, input: DossierEdit) => patch<DossierDetail>(`/v1/dossiers/${id}`, input),
  remove: (id: string) => del<void>(`/v1/dossiers/${id}`),
  act: (id: string, input: DossierActionRequest) => post<DossierDetail>(`/v1/dossiers/${id}/actions`, input),
  message: (id: string, body: string, attachments: string[]) =>
    post<DossierMessageView>(`/v1/dossiers/${id}/messages`, { body, attachments }),
  upload: (id: string, file: File, docType: string | null, purpose: 'package' | 'attachment' = 'package') =>
    upload<DossierFileView>(`/v1/dossiers/${id}/files${query({ name: file.name, docType, purpose })}`, file),
  fromProject: (id: string, tenantId: string, projectId: string, docType?: string) =>
    post<DossierFileView>(`/v1/dossiers/${id}/files/from-project`, { tenantId, projectId, docType: docType ?? null }),
  removeFile: (id: string, file: string) => del<void>(`/v1/dossiers/${id}/files/${file}`),
  fileUrl: (id: string, file: string) => `/v1/dossiers/${id}/files/${file}`,
  importFile: (id: string, file: string, tenantId: string | null) =>
    post<DossierFileImported>(`/v1/dossiers/${id}/files/${file}/import`, { tenantId }),
};

export const platform = {
  summary: () => get<PlatformSummary>('/v1/platform/summary'),
  tenants: () => get<PlatformTenant[]>('/v1/platform/tenants'),
  tenant: (id: string) => get<PlatformTenantDetail>(`/v1/platform/tenants/${id}`),
  createTenant: (input: unknown) => post<Created>('/v1/platform/tenants', input),
  updateTenant: (id: string, input: unknown) => patch<PlatformTenantDetail>(`/v1/platform/tenants/${id}`, input),
  users: (q?: string) => get<PlatformUser[]>(`/v1/platform/users${query({ q })}`),
  createUser: (input: { login: string; password: string; displayName: string; email: string | null }) =>
    post<Created>('/v1/platform/users', input),
  editUser: (id: string, input: { status?: string; password?: string }) => patch<void>(`/v1/platform/users/${id}`, input),
  professionals: (status?: ProfessionalStatus) => get<ProfessionalView[]>(`/v1/platform/professionals${query({ status })}`),
  review: (id: string, status: ProfessionalStatus, note: string) =>
    post<void>(`/v1/platform/professionals/${id}/review`, { status, note }),
  events: (limit = 100) => get<PlatformEventView[]>(`/v1/platform/events${query({ limit })}`),
  settings: () => get<{ signupOpen: boolean }>('/v1/platform/settings'),
  setSettings: (signupOpen: boolean) => put<{ signupOpen: boolean }>('/v1/platform/settings', { signupOpen }),
};
