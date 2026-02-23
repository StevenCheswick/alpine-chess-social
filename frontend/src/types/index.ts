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
}

export type ChessPlatform = 'chess_com';

export interface LinkedAccount {
  id: string;
  platform: 'chess_com';
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

// Game types
export interface GameData {
  id: string;
  platform: 'chess_com';
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
