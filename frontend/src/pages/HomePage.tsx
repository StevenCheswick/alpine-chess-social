import { useState } from 'react';
import PostCard from '../components/feed/PostCard';
import PostComposer from '../components/feed/PostComposer';
import type { Post } from '../types';

// Mock posts for development
const mockPosts: Post[] = [
  {
    id: '1',
    author: {
      id: '2',
      username: 'grandmaster_dan',
      displayName: 'Daniel Naroditsky',
      email: 'dan@example.com',
      bio: 'GM | Chess teacher | Speedrun enthusiast',
      avatarUrl: null,
      createdAt: '2024-01-01',
      isVerified: true,
      followerCount: 15000,
      followingCount: 200,
    },
    postType: 'game_share',
    content: 'Just played a beautiful smothered mate! Knight delivers the final blow while the king has nowhere to run.',
    gameData: {
      id: 'game1',
      platform: 'chess_com',
      pgn: '1. e4 e5 2. Nf3 Nc6 3. Bc4 Nf6 4. Ng5 d5 5. exd5 Nxd5 6. Nxf7 Kxf7 7. Qf3+ Ke6 8. Nc3 Ncb4 9. O-O c6 10. a3 Na6 11. d4 Nac7 12. Nxd5 Nxd5 13. dxe5 Be7 14. Bf4 Rf8 15. Qg4+ Kd7 16. e6+ Kc7 17. Qxg7 Qe8 18. Rfe1 Bd6 19. Bxd6+ Kxd6 20. Qf6 Kc7 21. Rad1 Rd8 22. Rxd5 cxd5 23. Qf4+ Kc6 24. Bxd5+ Kb6 25. e7 Rxd5 26. e8=Q Rd1 27. Qb5#',
      white: { username: 'grandmaster_dan', rating: 2650 },
      black: { username: 'challenger123', rating: 2400 },
      result: '1-0',
      timeControl: '3+0',
      playedAt: '2025-01-15T10:30:00Z',
      gameUrl: 'https://chess.com/game/12345',
      keyPositionFen: 'r1b1q3/pp2R3/1kp2Q2/1B1r4/5P2/P7/1PP2PPP/4R1K1 b - - 0 27',
      keyPositionIndex: 53,
      allMoves: ['e4', 'e5', 'Nf3', 'Nc6', 'Bc4', 'Nf6', 'Ng5', 'd5', 'exd5', 'Nxd5', 'Nxf7', 'Kxf7', 'Qf3+', 'Ke6', 'Nc3', 'Ncb4', 'O-O', 'c6', 'a3', 'Na6', 'd4', 'Nac7', 'Nxd5', 'Nxd5', 'dxe5', 'Be7', 'Bf4', 'Rf8', 'Qg4+', 'Kd7', 'e6+', 'Kc7', 'Qxg7', 'Qe8', 'Rfe1', 'Bd6', 'Bxd6+', 'Kxd6', 'Qf6', 'Kc7', 'Rad1', 'Rd8', 'Rxd5', 'cxd5', 'Qf4+', 'Kc6', 'Bxd5+', 'Kb6', 'e7', 'Rxd5', 'e8=Q', 'Rd1', 'Qb5#'],
    },
    achievementData: {
      type: 'smothered_mate',
      displayName: 'Smothered Mate',
      description: 'Deliver checkmate with a knight while the enemy king is surrounded by its own pieces',
    },
    likeCount: 234,
    commentCount: 18,
    isLiked: false,
    createdAt: '2025-01-15T11:00:00Z',
    updatedAt: '2025-01-15T11:00:00Z',
  },
  {
    id: '2',
    author: {
      id: '3',
      username: 'tactical_queen',
      displayName: 'Tactical Queen',
      email: 'queen@example.com',
      bio: 'Tactics trainer | 2200 FIDE',
      avatarUrl: null,
      createdAt: '2024-06-01',
      isVerified: false,
      followerCount: 450,
      followingCount: 120,
    },
    postType: 'text',
    content: 'What\'s your favorite opening as Black against 1. e4? I\'ve been playing the Sicilian Dragon lately and loving the attacking chances!',
    gameData: null,
    achievementData: null,
    likeCount: 56,
    commentCount: 42,
    isLiked: true,
    createdAt: '2025-01-15T09:30:00Z',
    updatedAt: '2025-01-15T09:30:00Z',
  },
  {
    id: '3',
    author: {
      id: '4',
      username: 'endgame_wizard',
      displayName: 'Endgame Wizard',
      email: 'wizard@example.com',
      bio: 'Endgame specialist | Rook endings are my jam',
      avatarUrl: null,
      createdAt: '2024-03-15',
      isVerified: true,
      followerCount: 890,
      followingCount: 65,
    },
    postType: 'achievement',
    content: 'Finally achieved my first Queen Sacrifice that led to checkmate! The feeling is incredible.',
    gameData: {
      id: 'game2',
      platform: 'lichess',
      pgn: '1. d4 d5 2. c4 e6 3. Nc3 Nf6 4. Bg5 Be7 5. e3 O-O',
      white: { username: 'endgame_wizard', rating: 1950 },
      black: { username: 'opponent456', rating: 1980 },
      result: '1-0',
      timeControl: '10+0',
      playedAt: '2025-01-14T18:00:00Z',
      gameUrl: 'https://lichess.org/game/abcde',
      keyPositionFen: 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1',
      keyPositionIndex: 0,
      allMoves: ['d4', 'd5', 'c4', 'e6', 'Nc3', 'Nf6', 'Bg5', 'Be7', 'e3', 'O-O'],
    },
    achievementData: {
      type: 'queen_sacrifice',
      displayName: 'Queen Sacrifice',
      description: 'Sacrifice your queen and go on to win the game',
    },
    likeCount: 312,
    commentCount: 27,
    isLiked: false,
    createdAt: '2025-01-14T19:00:00Z',
    updatedAt: '2025-01-14T19:00:00Z',
  },
];

export default function HomePage() {
  const [posts] = useState<Post[]>(mockPosts);

  return (
    <div className="space-y-4">
      {/* Post Composer */}
      <PostComposer />

      {/* Feed */}
      <div className="space-y-4">
        {posts.map((post) => (
          <PostCard key={post.id} post={post} />
        ))}
      </div>

      {/* Load More */}
      <div className="py-4 text-center">
        <button className="text-slate-400 hover:text-white transition-colors">
          Load more posts
        </button>
      </div>
    </div>
  );
}
