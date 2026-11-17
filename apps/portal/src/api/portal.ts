// The portal's endpoints (docs/adr/9000 §8), one function each, typed by the generated contracts.
import { del, get, patch, post, put, query, upload } from './client';
import type {
  ApiTokenCreated,
  ApiTokenView,
  ApplicationCreate,
  AttachmentView,
  DossierActionRequest,
  DossierDashboard,
  DossierDetail,
  DossierDirection,
  DossierEdit,
  DossierFileImported,
  DossierFileView,
  DossierList,
  DossierMessageView,
  DossierSignatureView,
  FeeAssess,
  FeeAssessment,
  FeeDefinition,
  FeePaid,
  FeePreview,
  FeeSchedule,
  FeeScheduleCopy,
  FeeScheduleEdit,
  FeeTry,
  FileRevisions,
  MemberView,
  MunicipalityCard,
  NotificationSettings,
  NotificationSettingsEdit,
  Notifications,
  OrganizationPortal,
  PlatformEventView,
  PlatformSettings,
  PlatformSettingsEdit,
  PlatformSummary,
  PlatformTenant,
  PlatformTenantDetail,
  PlatformUser,
  PortalCatalog,
  PortalMe,
  ProfessionalCard,
  ProfessionalStatus,
  ProfessionalView,
  ProjectDetails,
  ProjectPage,
  SessionView,
  SigningRequestCreated,
  SigningTrust,
  SigningVerdict,
  Signup,
  TenantRole,
  TokenScope,
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
  setOffice: (id: string, officeId: string | null) => put<DossierDetail>(`/v1/dossiers/${id}/office`, { officeId }),
  fromUpload: (id: string, uploadId: string, docType: string | null, purpose: 'package' | 'attachment' = 'package') =>
    post<DossierFileView>(`/v1/dossiers/${id}/files/from-upload`, { uploadId, purpose, docType }),
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
  settings: () => get<PlatformSettings>('/v1/platform/settings'),
  setSettings: (edit: PlatformSettingsEdit) => put<PlatformSettings>('/v1/platform/settings', edit),
};

/** The account itself (docs/adr/9000 §12): its sessions and its personal access tokens. */
export const account = {
  sessions: () => get<SessionView[]>('/v1/me/sessions'),
  endSession: (id: string) => del<void>(`/v1/me/sessions/${id}`),
  tokens: () => get<ApiTokenView[]>('/v1/me/tokens'),
  createToken: (name: string, scope: TokenScope, expiresInDays: number) =>
    post<ApiTokenCreated>('/v1/me/tokens', { name, scope, expiresInDays }),
  revokeToken: (id: string) => del<void>(`/v1/me/tokens/${id}`),
  notificationSettings: () => get<NotificationSettings>('/v1/me/notification-settings'),
  setNotificationSettings: (edit: NotificationSettingsEdit) => put<NotificationSettings>('/v1/me/notification-settings', edit),
};

/** A cloud project's page (docs/adr/9000 §15): what it is, its drawing, its revisions and its attachments. */
export const cloud = {
  details: (tenant: string, project: string) => get<ProjectDetails>(`/v1/tenants/${tenant}/projects/${project}/details`),
  revisions: (tenant: string, project: string) => get<FileRevisions>(`/v1/tenants/${tenant}/projects/${project}/files`),
  revisionUrl: (tenant: string, project: string, revision: string) => `/v1/tenants/${tenant}/projects/${project}/files/${revision}`,
  previewUrl: (tenant: string, project: string, width = 1200) => `/v1/tenants/${tenant}/projects/${project}/preview.svg?width=${width}`,
  attachments: (tenant: string, project: string) => get<AttachmentView[]>(`/v1/tenants/${tenant}/projects/${project}/attachments`),
  attach: (tenant: string, project: string, uploadId: string, note: string | null) =>
    post<AttachmentView>(`/v1/tenants/${tenant}/projects/${project}/attachments`, { uploadId, note }),
  attachmentUrl: (tenant: string, project: string, id: string, inline = false) =>
    `/v1/tenants/${tenant}/projects/${project}/attachments/${id}${inline ? '?inline=1' : ''}`,
  removeAttachment: (tenant: string, project: string, id: string) => del<void>(`/v1/tenants/${tenant}/projects/${project}/attachments/${id}`),
};

/** Fees and charges (docs/adr/9003): an organisation's tariffs and a dossier's assessments. */
export const fees = {
  schedules: (tenant: string) => get<FeeSchedule[]>(`/v1/tenants/${tenant}/fees/schedules`),
  schedule: (tenant: string, id: string) => get<FeeSchedule>(`/v1/tenants/${tenant}/fees/schedules/${id}`),
  create: (tenant: string, input: FeeScheduleEdit) => post<FeeSchedule>(`/v1/tenants/${tenant}/fees/schedules`, input),
  update: (tenant: string, id: string, input: FeeScheduleEdit) => put<FeeSchedule>(`/v1/tenants/${tenant}/fees/schedules/${id}`, input),
  remove: (tenant: string, id: string) => del<void>(`/v1/tenants/${tenant}/fees/schedules/${id}`),
  activate: (tenant: string, id: string) => post<FeeSchedule>(`/v1/tenants/${tenant}/fees/schedules/${id}/activate`),
  copy: (tenant: string, id: string, input: FeeScheduleCopy) => post<FeeSchedule>(`/v1/tenants/${tenant}/fees/schedules/${id}/copy`, input),
  template: (tenant: string) => get<FeeDefinition>(`/v1/tenants/${tenant}/fees/template`),
  tryOut: (tenant: string, input: FeeTry) => post<FeePreview>(`/v1/tenants/${tenant}/fees/try`, input),
  ofDossier: (id: string) => get<FeeAssessment[]>(`/v1/dossiers/${id}/fees`),
  preview: (id: string, input: FeeAssess) => post<FeePreview>(`/v1/dossiers/${id}/fees/preview`, input),
  issue: (id: string, input: FeeAssess) => post<FeeAssessment>(`/v1/dossiers/${id}/fees`, input),
  paid: (id: string, fee: string, input: FeePaid) => post<FeeAssessment>(`/v1/dossiers/${id}/fees/${fee}/paid`, input),
  cancel: (id: string, fee: string, reason: string) => post<FeeAssessment>(`/v1/dossiers/${id}/fees/${fee}/cancel`, { reason }),
};

/** E-signing a dossier's files (docs/adr/9004): the server's side; the signing itself is KentOS İmza's (app/imza.ts). */
export const signing = {
  trust: () => get<SigningTrust>('/v1/signing/trust'),
  request: (dossier: string, fileIds: string[]) =>
    post<SigningRequestCreated>(`/v1/dossiers/${dossier}/signing`, { clientType: 'WEB', fileIds }),
  deliver: (transaction: string, result: unknown) => post<SigningVerdict>(`/v1/signing/${transaction}/result`, result),
  cancel: (transaction: string) => post<void>(`/v1/signing/${transaction}/cancel`),
  signatures: (dossier: string) => get<DossierSignatureView[]>(`/v1/dossiers/${dossier}/signatures`),
};
