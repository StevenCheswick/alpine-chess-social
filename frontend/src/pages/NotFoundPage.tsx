import { Link } from 'react-router-dom';

export default function NotFoundPage() {
  return (
    <div className="min-h-screen bg-black flex items-center justify-center p-4">
      <div className="text-center">
        <div className="text-6xl mb-4">♞</div>
        <h1 className="text-4xl font-bold text-white mb-2">404</h1>
        <p className="text-xl text-slate-400 mb-6">
          Looks like this piece moved to a different square
        </p>
        <Link to="/" className="btn btn-primary">
          Back to Home
        </Link>
      </div>
    </div>
  );
}
