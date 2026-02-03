import { useParams } from 'react-router-dom';
import { useState } from 'react';
import { useAuthStore } from '../stores/authStore';

export default function ProfilePage() {
  const { username } = useParams<{ username: string }>();
  const { user } = useAuthStore();
  const [isFollowing, setIsFollowing] = useState(false);

  const isOwnProfile = user !== null && user.username === username;

  // Mock profile data
  const profile = {
    username,
    displayName: isOwnProfile && user ? user.displayName : 'Chess Player',
    bio: 'Passionate chess player. Love tactics and endgames!',
    avatarUrl: null,
    followerCount: 1234,
    followingCount: 567,
    gamesPlayed: 2500,
    achievements: 45,
    linkedAccounts: [
      { platform: 'chess_com', username: 'chesscom_user', rating: 1850 },
    ],
  };

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
                {profile.displayName[0]}
              </span>
            )}
          </div>

          {/* Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-3 mb-1">
              <h1 className="text-2xl font-bold text-white">{profile.displayName}</h1>
              {!isOwnProfile && (
                <button
                  onClick={() => setIsFollowing(!isFollowing)}
                  className={`btn text-sm ${isFollowing ? 'btn-secondary' : 'btn-primary'}`}
                >
                  {isFollowing ? 'Following' : 'Follow'}
                </button>
              )}
              {isOwnProfile && (
                <button className="btn btn-secondary text-sm">
                  Edit Profile
                </button>
              )}
            </div>
            <p className="text-slate-400 mb-3">@{profile.username}</p>
            <p className="text-white mb-4">{profile.bio}</p>

            {/* Stats */}
            <div className="flex gap-6 text-sm">
              <div>
                <span className="font-bold text-white">{profile.followerCount.toLocaleString()}</span>
                <span className="text-slate-400 ml-1">Followers</span>
              </div>
              <div>
                <span className="font-bold text-white">{profile.followingCount.toLocaleString()}</span>
                <span className="text-slate-400 ml-1">Following</span>
              </div>
              <div>
                <span className="font-bold text-white">{profile.gamesPlayed.toLocaleString()}</span>
                <span className="text-slate-400 ml-1">Games</span>
              </div>
              <div>
                <span className="font-bold text-white">{profile.achievements}</span>
                <span className="text-slate-400 ml-1">Achievements</span>
              </div>
            </div>
          </div>
        </div>

        {/* Linked Accounts */}
        {profile.linkedAccounts.length > 0 && (
          <div className="mt-6 pt-6 border-t border-slate-800">
            <h3 className="text-sm font-medium text-slate-400 mb-3">Linked Accounts</h3>
            <div className="flex gap-4">
              {profile.linkedAccounts.map((account) => (
                <div
                  key={account.platform}
                  className="flex items-center gap-2 px-3 py-2 bg-slate-800 rounded-lg"
                >
                  <div className="w-6 h-6 bg-green-600 rounded flex items-center justify-center">
                    <span className="text-white text-xs font-bold">
                      {account.platform === 'chess_com' ? 'C' : 'L'}
                    </span>
                  </div>
                  <div>
                    <p className="text-sm text-white">{account.username}</p>
                    <p className="text-xs text-slate-400">{account.rating} Blitz</p>
                  </div>
                </div>
              ))}
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
    </div>
  );
}
