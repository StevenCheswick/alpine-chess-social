import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { User, LinkedAccount } from '../types';

interface AuthState {
  user: User | null;
  linkedAccounts: LinkedAccount[];
  token: string | null;
  isAuthenticated: boolean;

  // Actions
  login: (user: User, token: string) => void;
  logout: () => void;
  updateUser: (user: Partial<User>) => void;
  setLinkedAccounts: (accounts: LinkedAccount[]) => void;
  addLinkedAccount: (account: LinkedAccount) => void;
  removeLinkedAccount: (platform: 'chess_com') => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      user: null,
      linkedAccounts: [],
      token: null,
      isAuthenticated: false,

      login: (user, token) =>
        set({
          user,
          token,
          isAuthenticated: true,
        }),

      logout: () =>
        set({
          user: null,
          linkedAccounts: [],
          token: null,
          isAuthenticated: false,
        }),

      updateUser: (updates) =>
        set((state) => ({
          user: state.user ? { ...state.user, ...updates } : null,
        })),

      setLinkedAccounts: (accounts) =>
        set({ linkedAccounts: accounts }),

      addLinkedAccount: (account) =>
        set((state) => ({
          linkedAccounts: [...state.linkedAccounts.filter(a => a.platform !== account.platform), account],
        })),

      removeLinkedAccount: (platform) =>
        set((state) => ({
          linkedAccounts: state.linkedAccounts.filter(a => a.platform !== platform),
        })),
    }),
    {
      name: 'chess-social-auth',
      partialize: (state) => ({
        user: state.user,
        token: state.token,
        isAuthenticated: state.isAuthenticated,
        linkedAccounts: state.linkedAccounts,
      }),
    }
  )
);
