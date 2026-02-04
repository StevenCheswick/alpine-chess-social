import { useState, useEffect } from 'react';
import { useAuthStore } from '../../stores/authStore';
import { postService, type Post } from '../../services/postService';
import { gameService, type Game } from '../../services/gameService';
import { ChessBoard } from '../chess';

interface PostComposerProps {
  onPostCreated?: (post: Post) => void;
}

export default function PostComposer({ onPostCreated }: PostComposerProps) {
  const { user } = useAuthStore();
  const [content, setContent] = useState('');
  const [isExpanded, setIsExpanded] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Game share state
  const [postType, setPostType] = useState<'text' | 'game_share'>('text');
  const [selectedGameId, setSelectedGameId] = useState<number | null>(null);
  const [games, setGames] = useState<Game[]>([]);
  const [isLoadingGames, setIsLoadingGames] = useState(false);
  const [keyPositionIndex, setKeyPositionIndex] = useState(0);

  // Fetch games when game share is selected
  useEffect(() => {
    if (postType === 'game_share' && games.length === 0 && !isLoadingGames) {
      setIsLoadingGames(true);
      gameService.getMyGames()
        .then((response) => {
          setGames(response.games);
        })
        .catch((err) => {
          console.error('Failed to load games:', err);
        })
        .finally(() => {
          setIsLoadingGames(false);
        });
    }
  }, [postType, games.length, isLoadingGames]);

  // Reset position when game changes
  useEffect(() => {
    setKeyPositionIndex(0);
  }, [selectedGameId]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!content.trim() || isSubmitting) return;

    // Validate game selection for game_share
    if (postType === 'game_share' && !selectedGameId) {
      setError('Please select a game to share');
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const post = await postService.createPost({
        content: content.trim(),
        postType,
        gameId: postType === 'game_share' ? selectedGameId ?? undefined : undefined,
        keyPositionIndex: postType === 'game_share' ? keyPositionIndex : undefined,
      });
      setContent('');
      setIsExpanded(false);
      setPostType('text');
      setSelectedGameId(null);
      setKeyPositionIndex(0);
      onPostCreated?.(post);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create post');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleCancel = () => {
    setIsExpanded(false);
    setContent('');
    setError(null);
    setPostType('text');
    setSelectedGameId(null);
    setKeyPositionIndex(0);
  };

  const toggleGameShare = () => {
    if (postType === 'game_share') {
      setPostType('text');
      setSelectedGameId(null);
      setKeyPositionIndex(0);
    } else {
      setPostType('game_share');
    }
  };

  const selectedGame = games.find(g => g.id === selectedGameId);

  const handlePositionChange = (_fen: string, moveIndex: number) => {
    setKeyPositionIndex(moveIndex);
  };

  return (
    <div className="card p-4">
      <div className="flex gap-3">
        {/* Avatar */}
        <div className="w-10 h-10 bg-slate-700 rounded-full flex-shrink-0 flex items-center justify-center">
          {user?.avatarUrl ? (
            <img src={user.avatarUrl} alt="" className="w-full h-full rounded-full object-cover" />
          ) : (
            <span className="text-white font-medium">
              {user?.username?.[0]?.toUpperCase() || '?'}
            </span>
          )}
        </div>

        {/* Input Area */}
        <div className="flex-1">
          <form onSubmit={handleSubmit}>
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              onFocus={() => setIsExpanded(true)}
              placeholder={postType === 'game_share'
                ? "Share your thoughts about this game..."
                : "Share a game, achievement, or chess thought..."}
              className="w-full bg-transparent text-white placeholder-slate-500 resize-none focus:outline-none"
              rows={isExpanded ? 3 : 1}
              disabled={isSubmitting}
            />

            {error && (
              <p className="text-red-400 text-sm mt-1">{error}</p>
            )}

            {/* Game Selector */}
            {isExpanded && postType === 'game_share' && (
              <div className="mt-3 border border-slate-700 rounded-lg p-3">
                <label className="block text-sm text-slate-400 mb-2">Select a game to share:</label>
                {isLoadingGames ? (
                  <p className="text-slate-500 text-sm">Loading games...</p>
                ) : games.length === 0 ? (
                  <p className="text-slate-500 text-sm">No games found. Sync your games from your profile first.</p>
                ) : (
                  <select
                    value={selectedGameId || ''}
                    onChange={(e) => setSelectedGameId(e.target.value ? Number(e.target.value) : null)}
                    className="w-full bg-slate-800 text-white border border-slate-600 rounded px-3 py-2 focus:outline-none focus:border-primary-400"
                  >
                    <option value="">Choose a game...</option>
                    {games.map((game) => (
                      <option key={game.id} value={game.id}>
                        vs {game.opponent} ({game.result}) - {game.timeControl || 'Unknown'} - {game.date ? new Date(game.date).toLocaleDateString() : 'Unknown date'}
                      </option>
                    ))}
                  </select>
                )}

                {/* Game Preview with ChessBoard */}
                {selectedGame && (
                  <div className="mt-3">
                    <div className="flex items-center justify-between mb-2">
                      <div className="text-sm">
                        <span className={selectedGame.result === '1-0' && selectedGame.userColor === 'white' || selectedGame.result === '0-1' && selectedGame.userColor === 'black' ? 'text-green-400' : selectedGame.result === '1/2-1/2' ? 'text-slate-400' : 'text-red-400'}>
                          {selectedGame.result === '1-0' && selectedGame.userColor === 'white' ? 'Won' :
                           selectedGame.result === '0-1' && selectedGame.userColor === 'black' ? 'Won' :
                           selectedGame.result === '1/2-1/2' ? 'Draw' : 'Lost'}
                        </span>
                        {' '}vs {selectedGame.opponent}
                        {selectedGame.opponentRating && <span className="text-slate-500"> ({selectedGame.opponentRating})</span>}
                      </div>
                      {selectedGame.tags.length > 0 && (
                        <div className="flex gap-1 flex-wrap">
                          {selectedGame.tags.map((tag) => (
                            <span key={tag} className="px-2 py-0.5 bg-primary-400/20 text-primary-400 rounded text-xs">
                              {tag}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>

                    {/* ChessBoard Preview */}
                    <div className="flex justify-center">
                      <ChessBoard
                        moves={selectedGame.moves || []}
                        startIndex={keyPositionIndex}
                        orientation={selectedGame.userColor === 'white' ? 'white' : 'black'}
                        whitePlayer={{ username: selectedGame.userColor === 'white' ? (user?.username || 'You') : selectedGame.opponent, rating: selectedGame.userColor === 'white' ? (selectedGame.userRating || undefined) : (selectedGame.opponentRating || undefined) }}
                        blackPlayer={{ username: selectedGame.userColor === 'black' ? (user?.username || 'You') : selectedGame.opponent, rating: selectedGame.userColor === 'black' ? (selectedGame.userRating || undefined) : (selectedGame.opponentRating || undefined) }}
                        showControls={true}
                        onPositionChange={handlePositionChange}
                      />
                    </div>

                    <p className="text-xs text-slate-500 text-center mt-2">
                      Navigate to the position you want to share (move {keyPositionIndex} of {selectedGame.moves?.length || 0})
                    </p>
                  </div>
                )}
              </div>
            )}

            {isExpanded && (
              <div className="mt-3 flex items-center justify-between border-t border-slate-800 pt-3">
                {/* Post Type Buttons */}
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={toggleGameShare}
                    className={`flex items-center gap-1 text-sm transition-colors ${
                      postType === 'game_share'
                        ? 'text-primary-400'
                        : 'text-slate-400 hover:text-primary-400'
                    }`}
                  >
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                    </svg>
                    <span>Game</span>
                  </button>
                  <button
                    type="button"
                    className="flex items-center gap-1 text-sm text-slate-400 hover:text-amber-400 transition-colors"
                  >
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z" />
                    </svg>
                    <span>Achievement</span>
                  </button>
                </div>

                {/* Submit */}
                <div className="flex gap-2">
                  <button
                    type="button"
                    onClick={handleCancel}
                    className="btn btn-ghost text-sm"
                    disabled={isSubmitting}
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={!content.trim() || isSubmitting || (postType === 'game_share' && !selectedGameId)}
                    className="btn btn-primary text-sm disabled:opacity-50"
                  >
                    {isSubmitting ? 'Posting...' : 'Post'}
                  </button>
                </div>
              </div>
            )}
          </form>
        </div>
      </div>
    </div>
  );
}
