import { useParams, Link } from 'react-router-dom';

export default function PostPage() {
  const { postId } = useParams<{ postId: string }>();

  return (
    <div className="space-y-4">
      <Link to="/" className="flex items-center gap-2 text-slate-400 hover:text-white transition-colors">
        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
        </svg>
        <span>Back</span>
      </Link>

      <div className="card p-6">
        <h1 className="text-xl font-bold text-white mb-4">Post Details</h1>
        <p className="text-slate-400">Post ID: {postId}</p>
        <p className="text-slate-400 mt-4">
          Full post view with comments coming soon...
        </p>
      </div>
    </div>
  );
}
