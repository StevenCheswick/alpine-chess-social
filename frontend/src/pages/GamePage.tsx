import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { AnalyzableChessBoard } from '../components/chess';
import { useAuthStore } from '../stores/authStore';
import { API_BASE_URL } from '../config/api';
import { analyzeGame } from '../services/analysisService';
import { analyzeGameProxy } from '../services/analysisProxy';
import type { GameAnalysis, FullAnalysis } from '../types/analysis';
import AnalyzeButton from '../components/chess/AnalyzeButton';
import GameAnalysisPanel from '../components/chess/GameAnalysisPanel';

interface Game {
  id: string;
  opponent: string;
  opponentRating: number | null;
  userRating: number | null;
  result: 'W' | 'L' | 'D';
  timeControl: string;
  date: string;
  tags: string[];
  moves: string[];
  userColor: 'white' | 'black';
  source: 'chess_com';
}

export default function GamePage() {
  const { gameId } = useParams<{ gameId: string }>();
  const navigate = useNavigate();
  const { user, token } = useAuthStore();
  const [game, setGame] = useState<Game | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Analysis state
  const [analysis, setAnalysis] = useState<GameAnalysis | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [analysisProgress, setAnalysisProgress] = useState(0);
  
  // Move navigation state
  const [currentMoveIndex, setCurrentMoveIndex] = useState(0);

  const displayUsername = user?.chessComUsername;

  // Handle position changes from the board
  const handlePositionChange = (_fen: string, moveIndex: number) => {
    setCurrentMoveIndex(moveIndex);
  };

  // Handle move click from analysis panel
  const handleMoveClick = (moveIndex: number) => {
    setCurrentMoveIndex(moveIndex);
  };

  useEffect(() => {
    if (gameId) {
      loadGame();
      loadAnalysis();
    }
  }, [gameId]);

  const loadGame = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${API_BASE_URL}/api/games/${gameId}`, {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (!response.ok) {
        if (response.status === 404) {
          throw new Error('Game not found');
        }
        throw new Error(`Failed to load game: ${response.statusText}`);
      }

      const data = await response.json();
      setGame(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load game');
    } finally {
      setLoading(false);
    }
  };

  const loadAnalysis = async () => {
    try {
      const response = await fetch(`${API_BASE_URL}/api/games/${gameId}/analysis`, {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (response.ok) {
        const data = await response.json();
        if (data) {
          setAnalysis(data);
        }
      }
    } catch (err) {
      // Analysis not found is fine, just means game hasn't been analyzed
      console.log('No existing analysis found');
    }
  };

  const saveAnalysis = async (analysisData: GameAnalysis | FullAnalysis) => {
    try {
      // Extract theme tags from puzzles + endgame segments if present
      const fullAnalysis = analysisData as FullAnalysis;
      const tags = new Set<string>();

      if (fullAnalysis.puzzles) {
        for (const puzzle of fullAnalysis.puzzles) {
          for (const theme of puzzle.themes) tags.add(theme);
        }
      }
      if (fullAnalysis.endgame_segments) {
        for (const seg of fullAnalysis.endgame_segments) {
          tags.add(seg.endgame_type);
        }
      }

      const payload = tags.size > 0
        ? { ...analysisData, tags: [...tags] }
        : analysisData;

      await fetch(`${API_BASE_URL}/api/games/${gameId}/analysis`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(payload),
      });
    } catch (err) {
      console.error('Failed to save analysis:', err);
    }
  };

  const handleAnalyze = async () => {
    if (!game || analyzing) return;

    setAnalyzing(true);
    setAnalysisProgress(0);

    try {
      let result: GameAnalysis;

      // Try WebSocket proxy first (includes puzzles + endgame analysis)
      try {
        result = await analyzeGameProxy(
          gameId!,
          game.moves,
          100000,
          (progress) => setAnalysisProgress(progress),
        );
      } catch {
        // Fallback to client-side analysis
        result = await analyzeGame(game.moves, game.userColor, {
          onProgress: (progress) => setAnalysisProgress(progress),
        });
      }

      setAnalysis(result);
      await saveAnalysis(result);
    } catch (err) {
      console.error('Analysis failed:', err);
    } finally {
      setAnalyzing(false);
      setAnalysisProgress(0);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="w-8 h-8 border-4 border-slate-700 border-t-emerald-500 rounded-full animate-spin" />
      </div>
    );
  }

  if (error || !game) {
    return (
      <div className="space-y-6">
        <button
          onClick={() => navigate(-1)}
          className="inline-flex items-center gap-2 text-slate-400 hover:text-white transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Games
        </button>

        <div className="card p-8 text-center">
          <div className="w-16 h-16 bg-slate-800 rounded-full flex items-center justify-center mx-auto mb-4">
            <svg className="w-8 h-8 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-white mb-2">Game not found</h2>
          <p className="text-slate-400">
            {error || "This game doesn't exist or you don't have access to it."}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-[calc(100vh-5.5rem)] flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-3 flex-shrink-0">
        <button
          onClick={() => navigate(-1)}
          className="inline-flex items-center gap-2 text-slate-400 hover:text-white transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          Back to Games
        </button>

        <AnalyzeButton
          onClick={handleAnalyze}
          isAnalyzing={analyzing}
          isAnalyzed={!!analysis}
          progress={analyzing ? analysisProgress : undefined}
        />
      </div>

      {/* Main content: Board and Analysis side by side */}
      <div className="flex flex-col xl:flex-row gap-4 flex-1 min-h-0">
        {/* Chess Board - constrained to available height */}
        <div className="xl:max-w-[min(520px,calc(100vh-12rem))] flex-shrink-0">
          <AnalyzableChessBoard
            moves={game.moves}
            orientation={game.userColor}
            whitePlayer={{
              username: game.userColor === 'white' ? displayUsername || '' : game.opponent,
              rating: game.userColor === 'white' ? game.userRating || undefined : game.opponentRating || undefined,
            }}
            blackPlayer={{
              username: game.userColor === 'black' ? displayUsername || '' : game.opponent,
              rating: game.userColor === 'black' ? game.userRating || undefined : game.opponentRating || undefined,
            }}
            showAnalysis={false}
            analysis={analysis || undefined}
            externalMoveIndex={currentMoveIndex}
            onPositionChange={handlePositionChange}
          />
        </div>

        {/* Analysis Panel - Side (offset to align with board, below engine lines) */}
        {analysis && (
          <div className="flex-1 min-w-0">
            <GameAnalysisPanel
              analysis={analysis}
              userColor={game.userColor}
              moves={game.moves}
              onMoveClick={handleMoveClick}
              currentMoveIndex={currentMoveIndex}
            />
          </div>
        )}
      </div>
    </div>
  );
}
