import { useParams } from 'react-router-dom';
import { useState, useEffect } from 'react';
import { useAuthStore } from '../stores/authStore';
import { profileService, type Profile } from '../services/profileService';
import { postService, type Post as ApiPost } from '../services/postService';
import EditProfileModal from '../components/EditProfileModal';
import PostCard from '../components/feed/PostCard';
import type { Post } from '../types';

// Transform API post to full Post type (same as HomePage)
function transformPost(apiPost: ApiPost): Post {
  return {
    id: String(apiPost.id),
    author: {
      id: apiPost.author.id,
      username: apiPost.author.username,
      displayName: apiPost.author.displayName,
      email: '',
      bio: null,
      avatarUrl: apiPost.author.avatarUrl,
      createdAt: '',
      isVerified: false,
      followerCount: 0,
      followingCount: 0,
    },
    postType: apiPost.postType,
    content: apiPost.content,
    gameData: apiPost.gameData ? {
      id: apiPost.gameData.id,
      platform: 'chess_com',
      pgn: '',
      white: { username: '', rating: apiPost.gameData.opponentRating || 0 },
      black: { username: apiPost.gameData.opponent, rating: apiPost.gameData.opponentRating || 0 },
      result: apiPost.gameData.result as '1-0' | '0-1' | '1/2-1/2',
      timeControl: apiPost.gameData.timeControl || '',
      playedAt: apiPost.gameData.date || '',
      gameUrl: '',
      allMoves: apiPost.gameData.moves,
      keyPositionFen: 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1',
      keyPositionIndex: apiPost.gameData.keyPositionIndex || 0,
    } : null,
    achievementData: null,
    likeCount: 0,
    commentCount: 0,
    isLiked: false,
    createdAt: apiPost.createdAt,
    updatedAt: apiPost.createdAt,
  };
}

export default function ProfilePage() {
  const { username } = useParams<{ username: string }>();
  const { user, updateUser } = useAuthStore();
  const [isFollowing, setIsFollowing] = useState(false);
  const [profile, setProfile] = useState<Profile | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showEditModal, setShowEditModal] = useState(false);

  // Posts state
  const [posts, setPosts] = useState<Post[]>([]);
  const [postsLoading, setPostsLoading] = useState(false);
  const [postsError, setPostsError] = useState<string | null>(null);
  const [hasMorePosts, setHasMorePosts] = useState(false);
  const [totalPosts, setTotalPosts] = useState(0);

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

  // Fetch posts when username changes
  useEffect(() => {
    if (!username) return;

    const fetchPosts = async () => {
      setPostsLoading(true);
      setPostsError(null);
      try {
        const response = await postService.getUserPosts(username);
        setPosts(response.posts.map(transformPost));
        setHasMorePosts(response.hasMore);
        setTotalPosts(response.total);
      } catch (err) {
        setPostsError(err instanceof Error ? err.message : 'Failed to load posts');
      } finally {
        setPostsLoading(false);
      }
    };

    fetchPosts();
  }, [username]);

  const loadMorePosts = async () => {
    if (!username || !hasMorePosts) return;
    try {
      const response = await postService.getUserPosts(username, 20, posts.length);
      setPosts((prev) => [...prev, ...response.posts.map(transformPost)]);
      setHasMorePosts(response.hasMore);
    } catch (err) {
      console.error('Failed to load more posts:', err);
    }
  };

  const handleProfileUpdate = (updatedProfile: Profile) => {
    // Preserve isOwnProfile since the PUT response doesn't include it
    setProfile({ ...updatedProfile, isOwnProfile: true });
    // Update the auth store - we know it's our own profile since we're editing it
    if (user) {
      updateUser({
        displayName: updatedProfile.displayName,
        bio: updatedProfile.bio,
        chessComUsername: updatedProfile.chessComUsername,
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
                <span className="font-bold text-white">{(profile.gamesCount || 0).toLocaleString()}</span>
                <span className="text-slate-400 ml-1">Games</span>
              </div>
              <div>
                <span className="font-bold text-white">{totalPosts}</span>
                <span className="text-slate-400 ml-1">Posts</span>
              </div>
            </div>
          </div>
        </div>

        {/* Linked Accounts Section */}
        <div className="mt-6 pt-6 border-t border-slate-800">
          <h3 className="text-sm font-medium text-slate-400 mb-3">Linked Accounts</h3>
          <div className="flex gap-4">
            {profile.chessComUsername ? (
              <div className="flex items-center gap-2 px-3 py-2 bg-slate-800 rounded-lg">
                <div className="w-6 h-6 bg-green-600 rounded flex items-center justify-center">
                  <span className="text-white text-xs font-bold">C</span>
                </div>
                <div>
                  <p className="text-sm text-white">{profile.chessComUsername}</p>
                  <p className="text-xs text-slate-400">Chess.com</p>
                </div>
              </div>
            ) : profile.isOwnProfile ? (
              <button
                onClick={() => setShowEditModal(true)}
                className="flex items-center gap-2 px-3 py-2 bg-slate-800 hover:bg-slate-700 rounded-lg transition-colors"
              >
                <div className="w-6 h-6 bg-slate-600 rounded flex items-center justify-center">
                  <span className="text-white text-xs font-bold">C</span>
                </div>
                <div className="text-left">
                  <p className="text-sm text-white">Link Chess.com</p>
                  <p className="text-xs text-slate-400">Not linked</p>
                </div>
              </button>
            ) : (
              <div className="flex items-center gap-2 px-3 py-2 bg-slate-800 rounded-lg opacity-50">
                <div className="w-6 h-6 bg-slate-600 rounded flex items-center justify-center">
                  <span className="text-white text-xs font-bold">C</span>
                </div>
                <div>
                  <p className="text-sm text-slate-400">Chess.com</p>
                  <p className="text-xs text-slate-500">Not linked</p>
                </div>
              </div>
            )}
          </div>
        </div>
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

      {/* Posts Section */}
      {postsLoading ? (
        <div className="card p-8 text-center text-slate-400">
          <p>Loading posts...</p>
        </div>
      ) : postsError ? (
        <div className="card p-8 text-center text-red-400">
          <p>{postsError}</p>
        </div>
      ) : posts.length === 0 ? (
        <div className="card p-8 text-center text-slate-400">
          <p>No posts yet</p>
        </div>
      ) : (
        <div className="space-y-4">
          {posts.map((post) => (
            <PostCard key={post.id} post={post} />
          ))}
          {hasMorePosts && (
            <div className="py-4 text-center">
              <button
                onClick={loadMorePosts}
                className="text-slate-400 hover:text-white transition-colors"
              >
                Load more posts
              </button>
            </div>
          )}
        </div>
      )}

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
