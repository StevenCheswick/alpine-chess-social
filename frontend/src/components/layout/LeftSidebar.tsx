import { NavLink } from 'react-router-dom';
import { useAuthStore } from '../../stores/authStore';

const navItems = [
  {
    label: 'Profile',
    path: '/profile', // Will be replaced with actual username
    icon: (
      <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
      </svg>
    ),
  },
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
  // Opening Tree - disabled for soft beta
  // {
  //   label: 'Opening Tree',
  //   path: '/opening-tree',
  //   icon: (
  //     <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
  //       <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
  //     </svg>
  //   ),
  // },
  {
    label: 'Trainer',
    path: '/trainer',
    icon: (
      <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth="2">
        <circle cx="12" cy="12" r="10" />
        <circle cx="12" cy="12" r="6" />
        <circle cx="12" cy="12" r="2" />
        <line x1="12" y1="2" x2="12" y2="6" />
        <line x1="12" y1="18" x2="12" y2="22" />
        <line x1="2" y1="12" x2="6" y2="12" />
        <line x1="18" y1="12" x2="22" y2="12" />
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
    label: 'Endgames',
    path: '/endgames',
    icon: (
      <svg className="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <rect x="8" y="2" width="8" height="4" rx="1" strokeLinecap="round" strokeLinejoin="round" />
        <path strokeLinecap="round" strokeLinejoin="round" d="M10 6v3M14 6v3M7 9h10l-1 7H8L7 9zM8 16h8v2a2 2 0 01-2 2h-4a2 2 0 01-2-2v-2z" />
      </svg>
    ),
  },
];

export default function LeftSidebar() {
  const { user } = useAuthStore();

  return (
    <aside className="hidden lg:block w-64 fixed left-0 top-16 bottom-0 border-r border-slate-800 bg-slate-950 p-4">
      <nav className="space-y-1">
        {navItems.map((item) => {
          // Replace profile path with actual username
          const path = item.path === '/profile' ? `/u/${user?.username}` : item.path;

          return (
            <NavLink
              key={item.label}
              to={path}
              className={({ isActive }) =>
                `flex items-center gap-3 px-4 py-3 rounded-lg transition-colors ${
                  isActive
                    ? 'bg-slate-800 text-white'
                    : 'text-slate-400 hover:bg-slate-800/50 hover:text-white'
                }`
              }
            >
              {item.icon}
              <span className="font-medium">{item.label}</span>
            </NavLink>
          );
        })}
      </nav>

    </aside>
  );
}
