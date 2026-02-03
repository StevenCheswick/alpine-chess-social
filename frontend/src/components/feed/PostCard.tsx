import { useState } from 'react';
import { Link } from 'react-router-dom';
import { ChessBoard } from '../chess';
import type { Post } from '../../types';

interface PostCardProps {
  post: Post;
}

function formatTimeAgo(dateString: string): string {
  const date = new Date(dateString);
  const now = new Date();
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  if (seconds < 604800) return `${Math.floor(seconds / 86400)}d`;
  return date.toLocaleDateString();
}

export default function PostCard({ post }: PostCardProps) {
  const [isLiked, setIsLiked] = useState(post.isLiked);
  const [likeCount, setLikeCount] = useState(post.likeCount);

  const handleLike = () => {
    setIsLiked(!isLiked);
    setLikeCount(isLiked ? likeCount - 1 : likeCount + 1);
  };

  return (
    <article className="card">
      {/* Header */}
      <div className="p-4 pb-3">
        <div className="flex items-start gap-3">
          {/* Avatar */}
          <Link to={`/u/${post.author.username}`}>
            <div className="w-10 h-10 bg-slate-700 rounded-full flex items-center justify-center">
              {post.author.avatarUrl ? (
                <img
                  src={post.author.avatarUrl}
                  alt=""
                  className="w-full h-full rounded-full object-cover"
                />
              ) : (
                <span className="text-white font-medium">
                  {post.author.displayName[0]}
                </span>
              )}
            </div>
          </Link>

          {/* Author Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <Link
                to={`/u/${post.author.username}`}
                className="font-semibold text-white hover:underline truncate"
              >
                {post.author.displayName}
              </Link>
              {post.author.isVerified && (
                <svg className="w-4 h-4 text-primary-400 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
                </svg>
              )}
            </div>
            <div className="flex items-center gap-1 text-sm text-slate-500">
              <span>@{post.author.username}</span>
              <span>·</span>
              <span>{formatTimeAgo(post.createdAt)}</span>
            </div>
          </div>

          {/* More Menu */}
          <button className="p-1 text-slate-500 hover:text-white transition-colors">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 12h.01M12 12h.01M19 12h.01M6 12a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0zm7 0a1 1 0 11-2 0 1 1 0 012 0z" />
            </svg>
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="px-4 pb-3">
        <p className="text-white whitespace-pre-wrap">{post.content}</p>
      </div>

      {/* Achievement Badge */}
      {post.achievementData && (
        <div className="px-4 pb-3">
          <div className="inline-flex items-center gap-2 px-3 py-1.5 bg-amber-500/10 border border-amber-500/20 rounded-full">
            <span className="text-amber-400">★</span>
            <span className="text-sm font-medium text-amber-400">
              {post.achievementData.displayName}
            </span>
          </div>
        </div>
      )}

      {/* Game Board */}
      {post.gameData && (
        <div className="px-4 pb-3">
          <ChessBoard
            fen={post.gameData.keyPositionFen}
            moves={post.gameData.allMoves}
            startIndex={post.gameData.keyPositionIndex}
            orientation={post.gameData.white.username === post.author.username ? 'white' : 'black'}
            whitePlayer={post.gameData.white}
            blackPlayer={post.gameData.black}
            gameUrl={post.gameData.gameUrl}
          />
        </div>
      )}

      {/* Actions */}
      <div className="px-4 py-3 border-t border-slate-800 flex items-center gap-6">
        {/* Like */}
        <button
          onClick={handleLike}
          className={`flex items-center gap-2 transition-colors ${
            isLiked ? 'text-red-500' : 'text-slate-400 hover:text-red-500'
          }`}
        >
          <svg
            className="w-5 h-5"
            fill={isLiked ? 'currentColor' : 'none'}
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"
            />
          </svg>
          <span className="text-sm">{likeCount}</span>
        </button>

        {/* Comment */}
        <Link
          to={`/post/${post.id}`}
          className="flex items-center gap-2 text-slate-400 hover:text-primary-400 transition-colors"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
            />
          </svg>
          <span className="text-sm">{post.commentCount}</span>
        </Link>

        {/* Share */}
        <button className="flex items-center gap-2 text-slate-400 hover:text-primary-400 transition-colors">
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z"
            />
          </svg>
        </button>
      </div>
    </article>
  );
}
