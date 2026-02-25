import { Outlet, NavLink } from 'react-router-dom';
import Navbar from './Navbar';
import LeftSidebar from './LeftSidebar';
import { useAuthStore } from '../../stores/authStore';

const mobileNavItems = [
  {
    label: 'Games',
    path: '/games',
    icon: (
      <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
    ),
  },
  {
    label: 'Puzzles',
    path: '/puzzles',
    icon: (
      <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
        <path strokeLinecap="round" strokeLinejoin="round" d="M12 2C9.5 2 8 3.5 8 5.5c0 1.5.5 2 1 2.5L8 10h8l-1-2c.5-.5 1-1 1-2.5C16 3.5 14.5 2 12 2z" />
        <rect x="7" y="10" width="10" height="2" rx="0.5" />
        <path strokeLinecap="round" strokeLinejoin="round" d="M8 12v7a3 3 0 003 3h2a3 3 0 003-3v-7" />
      </svg>
    ),
  },
  {
    label: 'Trainer',
    path: '/trainer',
    icon: (
      <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth="2">
        <circle cx="12" cy="12" r="10" />
        <circle cx="12" cy="12" r="6" />
        <circle cx="12" cy="12" r="2" />
      </svg>
    ),
  },
  {
    label: 'Dashboard',
    path: '/dashboard',
    icon: (
      <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    ),
  },
  {
    label: 'Profile',
    path: '/profile', // replaced with username at render
    icon: (
      <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
      </svg>
    ),
  },
];

export default function MainLayout() {
  const { user } = useAuthStore();

  return (
    <div className="min-h-screen" style={{ background: '#000' }}>
      <Navbar />
      <LeftSidebar />

      {/* Main content area — bottom padding on mobile for nav bar */}
      <main className="pt-16 lg:pl-64 pb-20 lg:pb-0">
        <div className="max-w-6xl mx-auto px-4 py-3">
          <Outlet />
        </div>
      </main>

      {/* Mobile bottom navigation */}
      <nav className="lg:hidden fixed bottom-0 left-0 right-0 h-16 border-t border-emerald-500/30 flex items-center justify-around px-2 z-50" style={{ background: '#0a0a0a' }}>
        {mobileNavItems.map((item) => {
          const path = item.path === '/profile' ? `/u/${user?.username}` : item.path;
          return (
            <NavLink
              key={item.label}
              to={path}
              className={({ isActive }) =>
                `flex flex-col items-center gap-0.5 px-2 py-1.5 rounded-lg text-[10px] font-medium transition-colors ${
                  isActive ? 'text-emerald-400' : 'text-slate-500'
                }`
              }
            >
              {item.icon}
              <span>{item.label}</span>
            </NavLink>
          );
        })}
      </nav>
    </div>
  );
}
