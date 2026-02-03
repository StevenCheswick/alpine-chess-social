import { useState } from 'react';
import { useAuthStore } from '../stores/authStore';

export default function SettingsPage() {
  const { user, updateUser } = useAuthStore();
  const [displayName, setDisplayName] = useState(user?.displayName || '');
  const [bio, setBio] = useState(user?.bio || '');

  const handleSave = () => {
    updateUser({ displayName, bio });
    // TODO: API call to save
  };

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-white">Settings</h1>

      {/* Profile Settings */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-white mb-4">Profile</h2>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1">
              Display Name
            </label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="input w-full"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-300 mb-1">
              Bio
            </label>
            <textarea
              value={bio}
              onChange={(e) => setBio(e.target.value)}
              className="input w-full"
              rows={3}
            />
          </div>
          <button onClick={handleSave} className="btn btn-primary">
            Save Changes
          </button>
        </div>
      </div>

      {/* Linked Accounts */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-white mb-4">Linked Chess Accounts</h2>
        <div className="space-y-3">
          {/* Chess.com */}
          <div className="flex items-center justify-between p-4 bg-slate-800 rounded-lg">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-green-600 rounded-lg flex items-center justify-center">
                <span className="text-white font-bold">C</span>
              </div>
              <div>
                <p className="font-medium text-white">Chess.com</p>
                <p className="text-sm text-slate-400">Not connected</p>
              </div>
            </div>
            <button className="btn btn-secondary text-sm">Connect</button>
          </div>

          {/* Lichess */}
          <div className="flex items-center justify-between p-4 bg-slate-800 rounded-lg">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-slate-600 rounded-lg flex items-center justify-center">
                <span className="text-white font-bold">L</span>
              </div>
              <div>
                <p className="font-medium text-white">Lichess</p>
                <p className="text-sm text-slate-400">Not connected</p>
              </div>
            </div>
            <button className="btn btn-secondary text-sm">Connect</button>
          </div>
        </div>
      </div>

      {/* Danger Zone */}
      <div className="card p-6 border-red-500/20">
        <h2 className="text-lg font-semibold text-red-400 mb-4">Danger Zone</h2>
        <p className="text-slate-400 mb-4">
          Once you delete your account, there is no going back. Please be certain.
        </p>
        <button className="btn bg-red-500/10 text-red-400 hover:bg-red-500/20 border border-red-500/20">
          Delete Account
        </button>
      </div>
    </div>
  );
}
