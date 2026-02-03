import { useState } from 'react';
import { profileService, type Profile, type UpdateProfileData } from '../services/profileService';

interface EditProfileModalProps {
  profile: Profile;
  onClose: () => void;
  onSave: (updatedProfile: Profile) => void;
}

export default function EditProfileModal({ profile, onClose, onSave }: EditProfileModalProps) {
  const [displayName, setDisplayName] = useState(profile.displayName);
  const [bio, setBio] = useState(profile.bio || '');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    // Validate
    if (!displayName.trim()) {
      setError('Display name cannot be empty');
      setLoading(false);
      return;
    }

    if (displayName.length > 50) {
      setError('Display name must be at most 50 characters');
      setLoading(false);
      return;
    }

    if (bio.length > 500) {
      setError('Bio must be at most 500 characters');
      setLoading(false);
      return;
    }

    try {
      const data: UpdateProfileData = {};

      // Only include fields that changed
      if (displayName !== profile.displayName) {
        data.displayName = displayName.trim();
      }
      if (bio !== (profile.bio || '')) {
        data.bio = bio.trim();
      }

      // If nothing changed, just close
      if (Object.keys(data).length === 0) {
        onClose();
        return;
      }

      const updatedProfile = await profileService.updateProfile(data);
      onSave(updatedProfile);
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update profile');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="card p-6 w-full max-w-md mx-4">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-bold text-white">Edit Profile</h2>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white transition-colors"
            disabled={loading}
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="p-3 bg-red-500/10 border border-red-500/50 rounded-lg text-red-400 text-sm">
              {error}
            </div>
          )}

          <div>
            <label htmlFor="displayName" className="block text-sm font-medium text-slate-300 mb-2">
              Display Name
            </label>
            <input
              type="text"
              id="displayName"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="input w-full"
              placeholder="Your display name"
              maxLength={50}
              disabled={loading}
            />
            <p className="mt-1 text-xs text-slate-500">{displayName.length}/50 characters</p>
          </div>

          <div>
            <label htmlFor="bio" className="block text-sm font-medium text-slate-300 mb-2">
              Bio
            </label>
            <textarea
              id="bio"
              value={bio}
              onChange={(e) => setBio(e.target.value)}
              className="input w-full h-24 resize-none"
              placeholder="Tell us about yourself..."
              maxLength={500}
              disabled={loading}
            />
            <p className="mt-1 text-xs text-slate-500">{bio.length}/500 characters</p>
          </div>

          <div className="flex gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              className="btn btn-secondary flex-1"
              disabled={loading}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary flex-1"
              disabled={loading}
            >
              {loading ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
