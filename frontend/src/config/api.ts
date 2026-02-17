// API Configuration
// For now, we'll use mock data. When backend is ready, update this.

export const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8000';
export const ANALYSIS_WS_URL = import.meta.env.VITE_ANALYSIS_WS_URL || 'ws://localhost:8000';

export const API_ENDPOINTS = {
  // Auth
  auth: {
    login: '/api/auth/login',
    register: '/api/auth/register',
    refresh: '/api/auth/refresh',
    logout: '/api/auth/logout',
  },
  // Users
  users: {
    me: '/api/users/me',
    profile: (username: string) => `/api/users/${username}`,
    linkAccount: '/api/users/me/link-account',
  },
  // Achievements
  achievements: {
    user: (username: string) => `/api/users/${username}/achievements`,
    sync: '/api/achievements/sync',
  },
  // Games
  games: {
    sync: '/api/games/sync',
    get: (id: string) => `/api/games/${id}`,
  },
} as const;
