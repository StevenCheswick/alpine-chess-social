// User types
export interface User {
  id: string | number;
  username: string;
  displayName: string;
  email: string;
  chessComUsername?: string;
  bio: string | null;
  avatarUrl: string | null;
  createdAt: string;
  isVerified: boolean;
  followerCount: number;
  followingCount: number;
}

export interface LinkedAccount {
  id: string;
  platform: 'chess_com' | 'lichess';
  platformUsername: string;
  isVerified: boolean;
  ratings: {
    bullet?: number;
    blitz?: number;
    rapid?: number;
    classical?: number;
  };
  lastSyncedAt: string;
}

// Post types
export type PostType = 'game_share' | 'achievement' | 'text' | 'puzzle';

export interface Post {
  id: string;
  author: User;
  postType: PostType;
  content: string;
  gameData: GameData | null;
  achievementData: AchievementData | null;
  likeCount: number;
  commentCount: number;
  isLiked: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface Comment {
  id: string;
  postId: string;
  author: User;
  content: string;
  createdAt: string;
  updatedAt: string;
}

// Game types
export interface GameData {
  id: string;
  platform: 'chess_com' | 'lichess';
  pgn: string;
  white: {
    username: string;
    rating: number;
  };
  black: {
    username: string;
    rating: number;
  };
  result: '1-0' | '0-1' | '1/2-1/2';
  timeControl: string;
  playedAt: string;
  gameUrl: string;
  // For display
  keyPositionFen?: string;
  keyPositionIndex?: number;
  allMoves?: string[];
}

// Achievement types
export type AchievementType =
  | 'smothered_mate'
  | 'castle_mate'
  | 'queen_sacrifice'
  | 'rook_sacrifice'
  | 'knight_fork'
  | 'back_rank_mate'
  | 'en_passant_mate'
  | 'pawn_mate'
  | 'king_mate'
  | 'windmill'
  | 'biggest_comeback'
  | 'longest_game'
  | 'king_walk';

export interface AchievementData {
  type: AchievementType;
  displayName: string;
  description: string;
  gameData?: GameData;
  value?: number; // e.g., material deficit for comeback
}

export interface UserAchievement {
  id: string;
  type: AchievementType;
  displayName: string;
  count: number;
  tier: 'bronze' | 'silver' | 'gold' | 'platinum';
  bestGame: GameData | null;
  firstAchievedAt: string;
  lastAchievedAt: string;
}

// Notification types
export type NotificationType = 'follow' | 'like' | 'comment' | 'achievement';

export interface Notification {
  id: string;
  type: NotificationType;
  isRead: boolean;
  createdAt: string;
  // Polymorphic data based on type
  actor?: User;
  post?: Post;
  achievement?: AchievementData;
}

// API types
export interface PaginatedResponse<T> {
  data: T[];
  nextCursor: string | null;
  hasMore: boolean;
}

export interface ApiError {
  message: string;
  code: string;
}
