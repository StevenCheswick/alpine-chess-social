/**
 * Authentication service for register, login, and user management.
 */

import api from './api';
import type { User } from '../types';

interface AuthResponse {
  user: User;
  token: string;
}

interface RegisterData {
  username: string;
  email: string;
  password: string;
  chessComUsername: string;
}

interface LoginData {
  email: string;
  password: string;
}

export const authService = {
  /**
   * Register a new user account.
   */
  async register(data: RegisterData): Promise<AuthResponse> {
    return api.post<AuthResponse>('/api/auth/register', data);
  },

  /**
   * Login with email and password.
   */
  async login(data: LoginData): Promise<AuthResponse> {
    return api.post<AuthResponse>('/api/auth/login', data);
  },

  /**
   * Get the current authenticated user.
   */
  async getCurrentUser(): Promise<User> {
    return api.get<User>('/api/auth/me');
  },
};

export default authService;
