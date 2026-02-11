import api from './api';

export interface EndgameTypeStat {
  type: string;
  games: number;
  userAvgCpLoss: number;
  opponentAvgCpLoss: number;
}

export interface EndgameStats {
  totalGamesWithEndgame: number;
  typeStats: EndgameTypeStat[];
}

export async function getEndgameStats(): Promise<EndgameStats> {
  return api.get<EndgameStats>('/api/games/endgame-stats');
}
