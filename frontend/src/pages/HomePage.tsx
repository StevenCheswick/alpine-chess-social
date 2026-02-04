import { useState, useEffect, useCallback } from 'react';
import PostCard from '../components/feed/PostCard';
import PostComposer from '../components/feed/PostComposer';
import { postService, type Post as ApiPost } from '../services/postService';
import type { Post } from '../types';

// Transform API post to full Post type with defaults for fields we don't have yet
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

export default function HomePage() {
  const [posts, setPosts] = useState<Post[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);

  const fetchPosts = useCallback(async () => {
    try {
      setIsLoading(true);
      setError(null);
      const response = await postService.getPosts();
      setPosts(response.posts.map(transformPost));
      setHasMore(response.hasMore);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load posts');
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchPosts();
  }, [fetchPosts]);

  const handlePostCreated = (post: ApiPost) => {
    setPosts((prev) => [transformPost(post), ...prev]);
  };

  const loadMore = async () => {
    if (!hasMore) return;
    try {
      const response = await postService.getPosts(20, posts.length);
      setPosts((prev) => [...prev, ...response.posts.map(transformPost)]);
      setHasMore(response.hasMore);
    } catch (err) {
      console.error('Failed to load more posts:', err);
    }
  };

  return (
    <div className="space-y-4">
      {/* Post Composer */}
      <PostComposer onPostCreated={handlePostCreated} />

      {/* Loading State */}
      {isLoading && (
        <div className="text-center py-8 text-slate-400">
          Loading posts...
        </div>
      )}

      {/* Error State */}
      {error && (
        <div className="text-center py-8 text-red-400">
          {error}
          <button
            onClick={fetchPosts}
            className="block mx-auto mt-2 text-primary-400 hover:underline"
          >
            Try again
          </button>
        </div>
      )}

      {/* Empty State */}
      {!isLoading && !error && posts.length === 0 && (
        <div className="text-center py-8 text-slate-400">
          No posts yet. Be the first to share something!
        </div>
      )}

      {/* Feed */}
      {!isLoading && !error && posts.length > 0 && (
        <div className="space-y-4">
          {posts.map((post) => (
            <PostCard key={post.id} post={post} />
          ))}
        </div>
      )}

      {/* Load More */}
      {hasMore && !isLoading && (
        <div className="py-4 text-center">
          <button
            onClick={loadMore}
            className="text-slate-400 hover:text-white transition-colors"
          >
            Load more posts
          </button>
        </div>
      )}
    </div>
  );
}
