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
    followers: (username: string) => `/api/users/${username}/followers`,
    following: (username: string) => `/api/users/${username}/following`,
    follow: (username: string) => `/api/users/${username}/follow`,
    linkAccount: '/api/users/me/link-account',
  },
  // Posts
  posts: {
    create: '/api/posts',
    get: (id: string) => `/api/posts/${id}`,
    delete: (id: string) => `/api/posts/${id}`,
    like: (id: string) => `/api/posts/${id}/like`,
    comments: (id: string) => `/api/posts/${id}/comments`,
  },
  // Feed
  feed: {
    home: '/api/feed',
    discover: '/api/feed/discover',
    user: (username: string) => `/api/users/${username}/posts`,
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
