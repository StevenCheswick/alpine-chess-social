import { useParams } from 'react-router-dom';
import { useState, useEffect } from 'react';
import { useAuthStore } from '../stores/authStore';
import { profileService, type Profile } from '../services/profileService';
import EditProfileModal from '../components/EditProfileModal';

export default function ProfilePage() {
  const { username } = useParams<{ username: string }>();
  const { user, updateUser } = useAuthStore();
  const [isFollowing, setIsFollowing] = useState(false);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showEditModal, setShowEditModal] = useState(false);

  useEffect(() => {
    if (!username) return;

    const fetchProfile = async () => {
      setLoading(true);
      setError(null);
      try {
        const data = await profileService.getProfile(username);
        setProfile(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load profile');
      } finally {
        setLoading(false);
      }
    };

    fetchProfile();
  }, [username]);

  const handleProfileUpdate = (updatedProfile: Profile) => {
    setProfile(updatedProfile);
    // Also update the auth store if it's the current user
    if (updatedProfile.isOwnProfile && user) {
      updateUser({
        displayName: updatedProfile.displayName,
        bio: updatedProfile.bio,
      });
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-slate-400">Loading profile...</div>
      </div>
    );
  }

  if (error || !profile) {
    return (
      <div className="card p-8 text-center">
        <p className="text-red-400 mb-2">{error || 'User not found'}</p>
        <p className="text-slate-500 text-sm">The user you're looking for doesn't exist.</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Profile Header */}
      <div className="card p-6">
        <div className="flex items-start gap-4">
          {/* Avatar */}
          <div className="w-24 h-24 bg-slate-700 rounded-full flex items-center justify-center flex-shrink-0">
            {profile.avatarUrl ? (
              <img src={profile.avatarUrl} alt="" className="w-full h-full rounded-full object-cover" />
            ) : (
              <span className="text-3xl font-bold text-white">
                {profile.displayName[0]?.toUpperCase() || '?'}
              </span>
            )}
          </div>

          {/* Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-3 mb-1">
              <h1 className="text-2xl font-bold text-white">{profile.displayName}</h1>
              {!profile.isOwnProfile && (
                <button
                  onClick={() => setIsFollowing(!isFollowing)}
                  className={`btn text-sm ${isFollowing ? 'btn-secondary' : 'btn-primary'}`}
                >
                  {isFollowing ? 'Following' : 'Follow'}
                </button>
              )}
              {profile.isOwnProfile && (
                <button
                  onClick={() => setShowEditModal(true)}
                  className="btn btn-secondary text-sm"
                >
                  Edit Profile
                </button>
              )}
            </div>
            <p className="text-slate-400 mb-3">@{profile.username}</p>
            {profile.bio && <p className="text-white mb-4">{profile.bio}</p>}

            {/* Stats */}
            <div className="flex gap-6 text-sm">
              <div>
                <span className="font-bold text-white">0</span>
                <span className="text-slate-400 ml-1">Followers</span>
              </div>
              <div>
                <span className="font-bold text-white">0</span>
                <span className="text-slate-400 ml-1">Following</span>
              </div>
              <div>
                <span className="font-bold text-white">{profile.gamesCount.toLocaleString()}</span>
                <span className="text-slate-400 ml-1">Games</span>
              </div>
              <div>
                <span className="font-bold text-white">0</span>
                <span className="text-slate-400 ml-1">Achievements</span>
              </div>
            </div>
          </div>
        </div>

        {/* Linked Chess.com Account */}
        {profile.chessComUsername && (
          <div className="mt-6 pt-6 border-t border-slate-800">
            <h3 className="text-sm font-medium text-slate-400 mb-3">Linked Accounts</h3>
            <div className="flex gap-4">
              <div className="flex items-center gap-2 px-3 py-2 bg-slate-800 rounded-lg">
                <div className="w-6 h-6 bg-green-600 rounded flex items-center justify-center">
                  <span className="text-white text-xs font-bold">C</span>
                </div>
                <div>
                  <p className="text-sm text-white">{profile.chessComUsername}</p>
                  <p className="text-xs text-slate-400">Chess.com</p>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Tabs */}
      <div className="flex border-b border-slate-800">
        <button className="px-6 py-3 text-white border-b-2 border-primary-500 font-medium">
          Posts
        </button>
        <button className="px-6 py-3 text-slate-400 hover:text-white transition-colors">
          Games
        </button>
        <button className="px-6 py-3 text-slate-400 hover:text-white transition-colors">
          Achievements
        </button>
      </div>

      {/* Posts placeholder */}
      <div className="card p-8 text-center text-slate-400">
        <p>No posts yet</p>
      </div>

      {/* Edit Profile Modal */}
      {showEditModal && (
        <EditProfileModal
          profile={profile}
          onClose={() => setShowEditModal(false)}
          onSave={handleProfileUpdate}
        />
      )}
    </div>
  );
}
