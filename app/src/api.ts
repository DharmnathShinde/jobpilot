import Constants from 'expo-constants';

// EXPO_PUBLIC_API_URL wins; otherwise app.json → extra.apiUrl.
// On an Android emulator use http://10.0.2.2:8080; on a phone use your computer's LAN IP.
export const API_URL: string =
  process.env.EXPO_PUBLIC_API_URL ?? (Constants.expoConfig?.extra as { apiUrl?: string } | undefined)?.apiUrl ?? 'http://localhost:8080';

export type Experience = { company: string; role: string; start: string; end: string; bullets: string[] };
export type Project = { name: string; description: string; url: string };
export type Education = { institution: string; degree: string; year: string };
export type Link = { label: string; url: string };

export type Profile = {
  full_name: string;
  headline: string;
  location: string;
  email: string;
  phone: string;
  summary: string;
  years_experience: number;
  skills: string[];
  experience: Experience[];
  projects: Project[];
  education: Education[];
  links: Link[];
  confirmed: boolean;
};

export type Preferences = {
  target_roles: string[];
  locations: string[];
  remote_ok: boolean;
  expected_ctc_lpa: number | null;
  notice_days: number | null;
  min_score: number;
  run_time: string; // "07:00:00"
  timezone: string;
  active_days: number[];
  max_jobs_per_run: number;
  schedule_enabled: boolean;
  last_run_on: string | null;
};

export type Job = {
  id: string;
  source: string;
  title: string;
  company: string;
  location: string;
  url: string;
  description: string;
  posted_at: string | null;
  required_skills: string[];
  min_years: number | null;
  max_years: number | null;
  score: number;
  matched_skills: string[];
  missing_skills: string[];
  explanation: string;
  queue_status: 'new' | 'saved' | 'skipped' | 'applied';
  created_at: string;
};

export type ResumeChange = { kind: 'reordered' | 'wording' | 'not_added'; text: string };
export type TailoredResume = { id: string; job_id: string; content: Profile; changes: ResumeChange[]; keyword_coverage: number; created_at: string };
export type Answer = { key: string; label: string; value: string };

export type ApplicationStatus = 'saved' | 'applied' | 'replied' | 'interview' | 'offer' | 'rejected';
export type Application = {
  id: string;
  job_id: string;
  status: ApplicationStatus;
  resume_id: string | null;
  notes: string;
  next_reminder: string | null;
  applied_at: string | null;
  updated_at: string;
  title: string;
  company: string;
  location: string;
  url: string;
  score: number;
};
export type AppEvent = { id: string; kind: string; note: string; at: string };

export type Source = { id: string; kind: 'greenhouse' | 'lever' | 'ashby'; board: string; company: string; created_at: string };

export type Today = {
  queue_count: number;
  reviewed_today: number;
  applied_today: number;
  run_time: string;
  last_run: { at: string; added: number } | null;
  top_matches: Job[];
};

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

let token: string | null = null;
let onUnauthorized: (() => void) | null = null;

export function setToken(t: string | null) {
  token = t;
}
export function setUnauthorizedHandler(fn: () => void) {
  onUnauthorized = fn;
}

async function request<T>(method: string, path: string, body?: unknown, isForm = false): Promise<T> {
  const headers: Record<string, string> = {};
  if (token) headers.Authorization = `Bearer ${token}`;
  if (body !== undefined && !isForm) headers['Content-Type'] = 'application/json';
  let res: Response;
  try {
    res = await fetch(`${API_URL}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : isForm ? (body as FormData) : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, `Can't reach the server at ${API_URL}`);
  }
  const text = await res.text();
  const data = text ? safeJson(text) : null;
  if (!res.ok) {
    if (res.status === 401 && token) onUnauthorized?.();
    const serverMsg = data && typeof data === 'object' && 'error' in data ? String((data as { error: unknown }).error) : '';
    throw new ApiError(res.status, serverMsg || `Request failed (${res.status})`);
  }
  return (data ?? (text as unknown)) as T;
}

function safeJson(t: string): unknown {
  try {
    return JSON.parse(t);
  } catch {
    return t;
  }
}

type AuthRes = { token: string; user: { id: string; email: string; name: string } };

export const api = {
  register: (email: string, password: string, name: string) => request<AuthRes>('POST', '/auth/register', { email, password, name }),
  login: (email: string, password: string) => request<AuthRes>('POST', '/auth/login', { email, password }),
  me: () => request<{ id: string; email: string; name: string; profile_confirmed: boolean }>('GET', '/me'),
  deleteAccount: () => request<{ deleted: boolean }>('DELETE', '/me'),

  profile: () => request<Profile>('GET', '/profile'),
  saveProfile: (p: Profile) => request<Profile>('PUT', '/profile', p),
  uploadCv: (file: { uri: string; name: string; mimeType?: string; webFile?: File }) => {
    const form = new FormData();
    if (file.webFile) form.append('file', file.webFile, file.name);
    // React Native's FormData accepts { uri, name, type } objects.
    else form.append('file', { uri: file.uri, name: file.name, type: file.mimeType ?? 'application/octet-stream' } as unknown as Blob);
    return request<{ profile: Profile; method: 'ai' | 'rules' }>('POST', '/profile/cv', form, true);
  },

  preferences: () => request<Preferences>('GET', '/preferences'),
  savePreferences: (p: Partial<Preferences>) => request<Preferences>('PUT', '/preferences', p),

  sources: () => request<Source[]>('GET', '/sources'),
  addSource: (kind: Source['kind'], board: string, company: string) => request<Source>('POST', '/sources', { kind, board, company }),
  deleteSource: (id: string) => request<{ deleted: boolean }>('DELETE', `/sources/${id}`),

  today: () => request<Today>('GET', '/today'),
  runNow: () => request<{ fetched: number; added: number; errors: string[] }>('POST', '/runs'),
  jobs: (status: 'new' | 'saved' | 'skipped' | 'applied' = 'new') => request<Job[]>('GET', `/jobs?status=${status}`),
  job: (id: string) => request<Job>('GET', `/jobs/${id}`),
  addJob: (j: { title: string; company: string; location: string; url: string; description: string; via?: 'share' | 'manual' }) =>
    request<Job>('POST', '/jobs', j),
  setJobStatus: (id: string, status: 'new' | 'saved' | 'skipped') => request<Job>('POST', `/jobs/${id}/status`, { status }),
  tailor: (id: string) => request<TailoredResume>('POST', `/jobs/${id}/tailor`),
  resume: (id: string) => request<TailoredResume>('GET', `/resumes/${id}`),
  resumeHtml: (id: string) => request<string>('GET', `/resumes/${id}/html`),
  answers: (id: string) => request<Answer[]>('GET', `/jobs/${id}/answers`),

  applications: () => request<{ counts: Record<ApplicationStatus, number>; applications: Application[] }>('GET', '/applications'),
  application: (id: string) => request<{ application: Application; events: AppEvent[] }>('GET', `/applications/${id}`),
  createApplication: (job_id: string, status: ApplicationStatus = 'applied', resume_id?: string) =>
    request<Application>('POST', '/applications', { job_id, status, resume_id }),
  updateApplication: (id: string, patch: { status?: ApplicationStatus; notes?: string; next_reminder?: string | null; event_note?: string }) =>
    request<Application>('PATCH', `/applications/${id}`, patch),
};
