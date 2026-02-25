import { Outlet } from 'react-router-dom';

export default function AuthLayout() {
  return (
    <div className="min-h-screen flex" style={{ background: '#000' }}>
      {/* Left side - Branding */}
      <div className="hidden lg:flex lg:w-1/2 bg-gradient-to-br from-emerald-900 via-slate-900 to-slate-950 p-12 flex-col justify-between">
        <div>
          <div className="flex items-center gap-3">
            <div className="w-12 h-12 bg-gradient-to-br from-emerald-400 to-teal-500 rounded-xl flex items-center justify-center shadow-[0_0_20px_rgba(16,185,129,0.4)]">
              <span className="text-white text-2xl">♞</span>
            </div>
            <span className="text-2xl font-bold text-white">Alpine Chess</span>
          </div>
        </div>

        <div className="space-y-6">
          <h1 className="text-4xl font-bold text-white leading-tight">
            Share your chess journey with the world
          </h1>
          <p className="text-lg text-slate-400">
            Connect your Chess.com account, showcase your best games,
            and improve with a community of players.
          </p>
          <div className="flex gap-8 text-slate-400">
            <div>
              <div className="text-3xl font-bold text-white">50+</div>
              <div className="text-sm">Opening Lines</div>
            </div>
            <div>
              <div className="text-3xl font-bold text-white">2</div>
              <div className="text-sm">Platforms Supported</div>
            </div>
            <div>
              <div className="text-3xl font-bold text-white">∞</div>
              <div className="text-sm">Games to Share</div>
            </div>
          </div>
        </div>

        <div className="text-slate-600 text-sm">
          © 2025 Alpine Chess. All rights reserved.
        </div>
      </div>

      {/* Right side - Auth forms */}
      <div className="w-full lg:w-1/2 flex items-center justify-center p-8">
        <div className="w-full max-w-md">
          <Outlet />
        </div>
      </div>
    </div>
  );
}
