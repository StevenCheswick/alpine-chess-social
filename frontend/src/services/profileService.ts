/**
 * Profile service for fetching and updating user profiles.
 */

import api from './api';

export interface Profile {
  id: number;
  username: string;
  displayName: string;
  chessComUsername: string;
  bio: string | null;
  avatarUrl: string | null;
  createdAt: string;
  gamesCount: number;
  isOwnProfile: boolean;
}

export interface UpdateProfileData {
  displayName?: string;
  bio?: string;
  chessComUsername?: string;
}

export const profileService = {
  /**
   * Get a user's public profile by username.
   */
  async getProfile(username: string): Promise<Profile> {
    return api.get<Profile>(`/api/users/${encodeURIComponent(username)}`);
  },

  /**
   * Update the current user's profile.
   */
  async updateProfile(data: UpdateProfileData): Promise<Profile> {
    return api.put<Profile>('/api/users/me', data);
  },
};

export default profileService;
