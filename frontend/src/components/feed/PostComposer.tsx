import { useState } from 'react';
import { useAuthStore } from '../../stores/authStore';

export default function PostComposer() {
  const { user } = useAuthStore();
  const [content, setContent] = useState('');
  const [isExpanded, setIsExpanded] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // TODO: Submit post
    console.log('Submitting post:', content);
    setContent('');
    setIsExpanded(false);
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
              placeholder="Share a game, achievement, or chess thought..."
              className="w-full bg-transparent text-white placeholder-slate-500 resize-none focus:outline-none"
              rows={isExpanded ? 3 : 1}
            />

            {isExpanded && (
              <div className="mt-3 flex items-center justify-between border-t border-slate-800 pt-3">
                {/* Post Type Buttons */}
                <div className="flex gap-2">
                  <button
                    type="button"
                    className="flex items-center gap-1 text-sm text-slate-400 hover:text-primary-400 transition-colors"
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
                    onClick={() => {
                      setIsExpanded(false);
                      setContent('');
                    }}
                    className="btn btn-ghost text-sm"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    disabled={!content.trim()}
                    className="btn btn-primary text-sm disabled:opacity-50"
                  >
                    Post
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
