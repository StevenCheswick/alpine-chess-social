import { useState } from 'react';
import { profileService, type Profile, type UpdateProfileData } from '../services/profileService';
import { API_BASE_URL } from '../config/api';

interface EditProfileModalProps {
  profile: Profile;
  onClose: () => void;
  onSave: (updatedProfile: Profile) => void;
  onDelete: () => void;
}

export default function EditProfileModal({ profile, onClose, onSave, onDelete }: EditProfileModalProps) {
  const [displayName, setDisplayName] = useState(profile.displayName);
  const [bio, setBio] = useState(profile.bio || '');
  const [chessComUsername, setChessComUsername] = useState(profile.chessComUsername || '');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleting, setDeleting] = useState(false);

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

    if (chessComUsername.length > 50) {
      setError('Chess.com username must be at most 50 characters');
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
      if (chessComUsername !== (profile.chessComUsername || '')) {
        data.chessComUsername = chessComUsername.trim();
      }

      // If nothing changed, just close
      if (Object.keys(data).length === 0) {
        onClose();
        return;
      }

      const updatedProfile = await profileService.updateProfile(data);
      
      // If user just linked a Chess.com account, trigger initial games sync
      const newlyLinkedChessCom = !profile.chessComUsername && data.chessComUsername;
      if (newlyLinkedChessCom) {
        // Trigger games sync in background (don't await - let it run)
        fetch(`${API_BASE_URL}/api/games?username=${encodeURIComponent(data.chessComUsername!)}`)
          .then(() => console.log('Initial games sync started'))
          .catch(err => console.error('Failed to start games sync:', err));
      }
      
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
      <div className="card p-6 w-full max-w-md mx-4 max-h-[90vh] overflow-y-auto">
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

          <div className="pt-4 border-t border-slate-800">
            <h3 className="text-sm font-medium text-slate-300 mb-3">Linked Accounts</h3>
            <div>
              <label htmlFor="chessComUsername" className="block text-sm font-medium text-slate-300 mb-2">
                Chess.com Username
              </label>
              <div className="flex gap-2">
                <div className="w-10 h-10 bg-green-600 rounded-lg flex items-center justify-center flex-shrink-0">
                  <span className="text-white text-sm font-bold">C</span>
                </div>
                <input
                  type="text"
                  id="chessComUsername"
                  value={chessComUsername}
                  onChange={(e) => setChessComUsername(e.target.value)}
                  className="input flex-1"
                  placeholder="your_chess_com_username"
                  maxLength={50}
                  disabled={loading}
                />
              </div>
              <p className="mt-1 text-xs text-slate-500">Enter your Chess.com username to sync your games</p>
            </div>
          </div>

          <div className="flex gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              className="btn btn-secondary flex-1"
              disabled={loading || deleting}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary flex-1"
              disabled={loading || deleting}
            >
              {loading ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </form>

        {/* Danger Zone */}
        <div className="mt-6 pt-6 border-t border-red-500/30">
          <h3 className="text-sm font-medium text-red-400 mb-3">Danger Zone</h3>
          {!showDeleteConfirm ? (
            <button
              onClick={() => setShowDeleteConfirm(true)}
              className="px-4 py-2 bg-red-500/10 border border-red-500/50 text-red-400 rounded-lg hover:bg-red-500/20 transition-colors text-sm"
              disabled={loading || deleting}
            >
              Delete Account
            </button>
          ) : (
            <div className="space-y-3">
              <p className="text-sm text-red-400">
                This will permanently delete your account and all associated data. This cannot be undone.
              </p>
              <div className="flex gap-3">
                <button
                  onClick={() => setShowDeleteConfirm(false)}
                  className="btn btn-secondary text-sm"
                  disabled={deleting}
                >
                  Cancel
                </button>
                <button
                  onClick={async () => {
                    setDeleting(true);
                    setError(null);
                    try {
                      await profileService.deleteAccount();
                      onDelete();
                    } catch (err) {
                      setError(err instanceof Error ? err.message : 'Failed to delete account');
                      setDeleting(false);
                    }
                  }}
                  className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors text-sm"
                  disabled={deleting}
                >
                  {deleting ? 'Deleting...' : 'Confirm Delete'}
                </button>
              </div>
            </div>
          )}
      </div>
    </div>
  );
}
