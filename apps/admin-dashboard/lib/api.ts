const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || "/api";

// Types
export interface User {
  id: string;
  email: string;
  name: string;
  role: "admin" | "user";
  isActive: boolean;
  createdAt: string;
  lastLoginAt?: string;
}

export interface AuditLog {
  id: string;
  userId: string;
  userEmail: string;
  action: string;
  resource: string;
  details: Record<string, unknown>;
  ipAddress: string;
  userAgent: string;
  createdAt: string;
}

export interface TenantSettings {
  name: string;
  retentionDays: number;
  maxUsers: number;
  features: {
    audioRecording: boolean;
    screenRecording: boolean;
    aiInsights: boolean;
  };
}

export interface DashboardMetrics {
  totalFrames: number;
  totalAudioChunks: number;
  activeUsers: number;
  storageUsed: number;
  framesTrend: { date: string; count: number }[];
  audioTrend: { date: string; count: number }[];
}

// Auth
export function setToken(token: string) {
  localStorage.setItem("token", token);
}

export function getToken(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("token");
}

export function removeToken() {
  localStorage.removeItem("token");
}

async function fetchWithAuth(url: string, options: RequestInit = {}) {
  const token = getToken();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...((options.headers as Record<string, string>) || {}),
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(`${API_BASE_URL}${url}`, {
    ...options,
    headers,
  });

  if (response.status === 401) {
    removeToken();
    window.location.href = "/login";
    throw new Error("Unauthorized");
  }

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: "Unknown error" }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

// API Client
export const api = {
  // Auth
  login: (email: string, password: string) =>
    fetch(`${API_BASE_URL}/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    }).then((r) => r.json()),

  // Dashboard
  getMetrics: (): Promise<DashboardMetrics> => fetchWithAuth("/metrics"),

  // Users
  getUsers: (): Promise<User[]> => fetchWithAuth("/users"),
  getUser: (id: string): Promise<User> => fetchWithAuth(`/users/${id}`),
  updateUser: (id: string, data: Partial<User>): Promise<User> =>
    fetchWithAuth(`/users/${id}`, {
      method: "PATCH",
      body: JSON.stringify(data),
    }),
  deleteUser: (id: string): Promise<void> =>
    fetchWithAuth(`/users/${id}`, { method: "DELETE" }),

  // Audit Logs
  getAuditLogs: (filters?: {
    startDate?: string;
    endDate?: string;
    action?: string;
    userId?: string;
    page?: number;
    limit?: number;
  }): Promise<{ logs: AuditLog[]; total: number }> => {
    const params = new URLSearchParams();
    if (filters?.startDate) params.append("startDate", filters.startDate);
    if (filters?.endDate) params.append("endDate", filters.endDate);
    if (filters?.action) params.append("action", filters.action);
    if (filters?.userId) params.append("userId", filters.userId);
    if (filters?.page) params.append("page", filters.page.toString());
    if (filters?.limit) params.append("limit", filters.limit.toString());
    return fetchWithAuth(`/audit?${params.toString()}`);
  },

  // Settings
  getSettings: (): Promise<TenantSettings> => fetchWithAuth("/settings"),
  updateSettings: (data: Partial<TenantSettings>): Promise<TenantSettings> =>
    fetchWithAuth("/settings", {
      method: "PATCH",
      body: JSON.stringify(data),
    }),
};
