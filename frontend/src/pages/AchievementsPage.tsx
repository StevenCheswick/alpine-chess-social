import { useState } from 'react';

type Tier = 'locked' | 'bronze' | 'silver' | 'gold' | 'platinum' | 'master';

interface Achievement {
  id: string;
  name: string;
  icon: string;
  tier: Tier;
  count: number;
}

interface AchievementCategory {
  name: string;
  achievements: Achievement[];
}

const tierThresholds: Record<Tier, number> = {
  locked: 0,
  bronze: 1,
  silver: 5,
  gold: 25,
  platinum: 100,
  master: 500,
};

const tierOrder: Tier[] = ['locked', 'bronze', 'silver', 'gold', 'platinum', 'master'];

function getNextTier(tier: Tier): Tier | null {
  const idx = tierOrder.indexOf(tier);
  if (idx === -1 || idx >= tierOrder.length - 1) return null;
  return tierOrder[idx + 1];
}

function getProgress(count: number, tier: Tier): { current: number; max: number; percent: number } {
  const nextTier = getNextTier(tier);
  if (!nextTier) return { current: count, max: count, percent: 100 };

  const currentThreshold = tierThresholds[tier];
  const nextThreshold = tierThresholds[nextTier];
  const progress = count - currentThreshold;
  const needed = nextThreshold - currentThreshold;

  return {
    current: count,
    max: nextThreshold,
    percent: Math.min((progress / needed) * 100, 100),
  };
}

const tierColors: Record<Tier, string> = {
  locked: 'bg-slate-800 border-slate-700',
  bronze: 'bg-gradient-to-br from-amber-900/40 to-amber-800/20 border-amber-700/50',
  silver: 'bg-gradient-to-br from-slate-600/40 to-slate-500/20 border-slate-400/50',
  gold: 'bg-gradient-to-br from-yellow-600/40 to-yellow-500/20 border-yellow-500/50',
  platinum: 'bg-gradient-to-br from-cyan-600/40 to-cyan-500/20 border-cyan-400/50',
  master: 'bg-gradient-to-br from-purple-600/40 to-purple-500/20 border-purple-400/50',
};

const progressBarColors: Record<Tier, string> = {
  locked: 'bg-slate-600',
  bronze: 'bg-amber-600',
  silver: 'bg-slate-400',
  gold: 'bg-yellow-500',
  platinum: 'bg-cyan-400',
  master: 'bg-purple-400',
};

// Mock data organized by category
const mockCategories: AchievementCategory[] = [
  {
    name: 'Basic Checkmates',
    achievements: [
      // Piece Mates
      { id: 'pm-1', name: 'Pawn Mate', icon: '♟', tier: 'silver', count: 12 },
      { id: 'pm-2', name: 'Knight Mate', icon: '♞', tier: 'gold', count: 31 },
      { id: 'pm-3', name: 'Bishop Mate', icon: '♝', tier: 'bronze', count: 3 },
      { id: 'pm-4', name: 'Rook Mate', icon: '♜', tier: 'platinum', count: 178 },
      { id: 'pm-5', name: 'Queen Mate', icon: '♛', tier: 'master', count: 892 },
      { id: 'pm-6', name: 'King Mate', icon: '♔', tier: 'locked', count: 0 },
      // Castle Mates
      { id: 'cm-1', name: 'Kingside Castle Mate', icon: '♚', tier: 'bronze', count: 2 },
      { id: 'cm-2', name: 'Queenside Castle Mate', icon: '♚', tier: 'locked', count: 0 },
      // En Passant
      { id: 'ep-1', name: 'En Passant Mate', icon: '♟', tier: 'locked', count: 0 },
      // Promotion Mates
      { id: 'promo-1', name: 'Queen Promotion Mate', icon: '♛', tier: 'silver', count: 8 },
      { id: 'promo-2', name: 'Rook Promotion Mate', icon: '♜', tier: 'locked', count: 0 },
      { id: 'promo-3', name: 'Bishop Promotion Mate', icon: '♝', tier: 'locked', count: 0 },
      { id: 'promo-4', name: 'Knight Promotion Mate', icon: '♞', tier: 'bronze', count: 1 },
    ],
  },
  {
    name: 'Named Checkmate Patterns',
    achievements: [
      { id: 'np-1', name: 'Smothered Mate', icon: '♞', tier: 'gold', count: 27 },
      { id: 'np-2', name: 'Back Rank Mate', icon: '♜', tier: 'platinum', count: 156 },
      { id: 'np-3', name: "Anastasia's Mate", icon: '♞', tier: 'locked', count: 0 },
      { id: 'np-4', name: 'Arabian Mate', icon: '♞', tier: 'silver', count: 6 },
      { id: 'np-5', name: "Boden's Mate", icon: '♝', tier: 'bronze', count: 2 },
      { id: 'np-6', name: "Blackburne's Mate", icon: '♝', tier: 'locked', count: 0 },
      { id: 'np-7', name: 'Opera Mate', icon: '♜', tier: 'gold', count: 34 },
      { id: 'np-8', name: "Morphy's Mate", icon: '♜', tier: 'silver', count: 11 },
      { id: 'np-9', name: "Greco's Mate", icon: '♛', tier: 'bronze', count: 4 },
      { id: 'np-10', name: "Damiano's Mate", icon: '♛', tier: 'silver', count: 9 },
      { id: 'np-11', name: "Legal's Mate", icon: '♞', tier: 'locked', count: 0 },
      { id: 'np-12', name: 'Dovetail Mate', icon: '♛', tier: 'gold', count: 42 },
      { id: 'np-13', name: 'Epaulette Mate', icon: '♛', tier: 'silver', count: 15 },
      { id: 'np-14', name: 'Hook Mate', icon: '♜', tier: 'bronze', count: 3 },
      { id: 'np-15', name: 'Corridor Mate', icon: '♜', tier: 'gold', count: 28 },
      { id: 'np-16', name: "Scholar's Mate", icon: '♛', tier: 'silver', count: 7 },
      { id: 'np-17', name: "Fool's Mate", icon: '♛', tier: 'locked', count: 0 },
      { id: 'np-18', name: 'Lawnmower Mate', icon: '♜', tier: 'platinum', count: 203 },
      { id: 'np-19', name: 'Kill Box Mate', icon: '♜', tier: 'bronze', count: 2 },
      { id: 'np-20', name: 'Corner Mate', icon: '♞', tier: 'silver', count: 8 },
    ],
  },
  {
    name: 'Game Achievements',
    achievements: [
      { id: 'ga-1', name: 'King Walk (3rd Rank)', icon: '♔', tier: 'gold', count: 45 },
      { id: 'ga-2', name: 'King Walk (2nd Rank)', icon: '♔', tier: 'silver', count: 12 },
      { id: 'ga-3', name: 'King Walk (1st Rank)', icon: '♔', tier: 'bronze', count: 3 },
      { id: 'ga-4', name: 'Biggest Comeback', icon: '♚', tier: 'silver', count: 15 },
      { id: 'ga-5', name: 'Clutch Win', icon: '♚', tier: 'gold', count: 38 },
      { id: 'ga-6', name: 'Longest Game', icon: '♟', tier: 'bronze', count: 4 },
      { id: 'ga-7', name: 'Stalemate', icon: '♚', tier: 'silver', count: 19 },
    ],
  },
  {
    name: 'Tactics',
    achievements: [
      { id: 't-1', name: 'Knight Fork', icon: '♞', tier: 'master', count: 523 },
      { id: 't-2', name: 'Skewer', icon: '♗', tier: 'gold', count: 67 },
      { id: 't-3', name: 'Pin', icon: '♗', tier: 'platinum', count: 234 },
      { id: 't-4', name: 'Discovered Attack', icon: '♜', tier: 'gold', count: 89 },
      { id: 't-5', name: 'Double Check', icon: '♚', tier: 'silver', count: 18 },
      { id: 't-6', name: 'Windmill', icon: '♜', tier: 'bronze', count: 2 },
    ],
  },
  {
    name: 'Sacrifices',
    achievements: [
      { id: 's-1', name: 'Queen Sacrifice', icon: '♛', tier: 'silver', count: 9 },
      { id: 's-2', name: 'Rook Sacrifice', icon: '♜', tier: 'gold', count: 45 },
      { id: 's-3', name: 'Bishop Sacrifice', icon: '♝', tier: 'platinum', count: 112 },
      { id: 's-4', name: 'Knight Sacrifice', icon: '♞', tier: 'gold', count: 67 },
      { id: 's-5', name: 'Exchange Sacrifice', icon: '♜', tier: 'silver', count: 23 },
    ],
  },
];

function AchievementTile({ achievement }: { achievement: Achievement }) {
  const isLocked = achievement.tier === 'locked';
  const isMaster = achievement.tier === 'master';
  const progress = getProgress(achievement.count, achievement.tier);
  const nextTier = getNextTier(achievement.tier);

  return (
    <div
      className={`relative p-4 rounded-xl border transition-transform hover:scale-105 ${
        tierColors[achievement.tier]
      } ${isLocked ? 'opacity-40' : ''}`}
    >
      <div className="text-3xl mb-2">{achievement.icon}</div>
      <div className={`text-sm font-medium ${isLocked ? 'text-slate-500' : 'text-white'}`}>
        {achievement.name}
      </div>
      {!isLocked && (
        <div className="mt-2">
          {!isMaster && nextTier ? (
            <>
              <div className="h-1 bg-slate-700 rounded-full overflow-hidden">
                <div
                  className={`h-full ${progressBarColors[achievement.tier]} transition-all`}
                  style={{ width: `${progress.percent}%` }}
                />
              </div>
              <div className="text-xs text-slate-500 mt-1">
                {progress.current}/{progress.max}
              </div>
            </>
          ) : (
            <div className="text-xs text-slate-400">{achievement.count}×</div>
          )}
        </div>
      )}
    </div>
  );
}

export default function AchievementsPage() {
  const [showLocked, setShowLocked] = useState(true);

  const allAchievements = mockCategories.flatMap(c => c.achievements);
  const earned = allAchievements.filter(a => a.tier !== 'locked').length;
  const total = allAchievements.length;

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Achievements</h1>
          <p className="text-slate-400 text-sm mt-1">{earned} of {total} unlocked</p>
        </div>
        <button
          onClick={() => setShowLocked(!showLocked)}
          className="text-sm text-slate-400 hover:text-white transition-colors"
        >
          {showLocked ? 'Hide locked' : 'Show locked'}
        </button>
      </div>

      {/* Categories */}
      {mockCategories.map((category) => {
        const filtered = showLocked
          ? category.achievements
          : category.achievements.filter(a => a.tier !== 'locked');

        if (filtered.length === 0) return null;

        const categoryEarned = category.achievements.filter(a => a.tier !== 'locked').length;
        const categoryTotal = category.achievements.length;

        return (
          <section key={category.name}>
            <div className="flex items-baseline gap-3 mb-4">
              <h2 className="text-lg font-semibold text-white">{category.name}</h2>
              <span className="text-sm text-slate-500">{categoryEarned}/{categoryTotal}</span>
            </div>
            <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 gap-3">
              {filtered.map((achievement) => (
                <AchievementTile key={achievement.id} achievement={achievement} />
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}
