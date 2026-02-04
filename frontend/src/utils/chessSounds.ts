/**
 * Chess move sound effects
 */

export type MoveType = 'move' | 'capture' | 'check' | 'checkmate' | 'promotion' | 'castle';

// Game event sounds (for game start/end)
export type GameEventType = 'gamestart' | 'victory' | 'defeat' | 'draw';

/**
 * Determines the type of move for sound selection
 */
export function getMoveType(move: any): MoveType {
  if (!move || typeof move !== 'object') {
    return 'move';
  }

  // Check for castling
  if (move.san && (move.san.includes('O-O') || move.san.includes('0-0'))) {
    return 'castle';
  }
  if (move.piece === 'k' && move.flags) {
    const flagsStr = String(move.flags);
    if (flagsStr === 'b' || flagsStr === 'q' || flagsStr.startsWith('b') || flagsStr.startsWith('q')) {
      return 'castle';
    }
  }

  // Check for promotion
  if (move.promotion || (move.san && move.san.includes('='))) {
    return 'promotion';
  }

  // Check for checkmate
  if (move.checkmate || (move.san && move.san.endsWith('#'))) {
    return 'checkmate';
  }

  // Check for check
  if (move.check || (move.san && move.san.endsWith('+'))) {
    return 'check';
  }

  // Check for capture
  if (move.capture || (move.san && move.san.includes('x'))) {
    return 'capture';
  }
  if (move.flags && (move.flags.includes('c') || move.flags.includes('e'))) {
    return 'capture';
  }

  return 'move';
}

const isIOS = () => {
  return /iPhone|iPad|iPod/i.test(navigator.userAgent) ||
    (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
};

/**
 * Plays the appropriate sound for a chess move
 */
export function playMoveSound(moveType: MoveType, volume: number = 0.5): void {
  try {
    const soundMap: Record<MoveType, string> = {
      move: '/sounds/move-self.mp3',
      capture: '/sounds/capture.mp3',
      check: '/sounds/move-check.mp3',
      checkmate: '/sounds/game-end.mp3',
      promotion: '/sounds/promote.mp3',
      castle: '/sounds/castle.mp3',
    };

    const soundPath = soundMap[moveType];
    if (!soundPath) return;

    const adjustedVolume = isIOS() ? volume * 0.8 : volume;
    const audio = new Audio(soundPath);
    audio.volume = Math.max(0, Math.min(1, adjustedVolume));
    audio.play().catch(() => {
      // Silently fail - user may not have interacted with page yet
    });
  } catch (error) {
    console.error('[ChessSounds] Error playing sound:', error);
  }
}

/**
 * Plays game event sounds (start, victory, defeat, draw)
 */
export function playGameSound(eventType: GameEventType, volume: number = 0.5): void {
  try {
    const soundMap: Record<GameEventType, string> = {
      gamestart: '/sounds/game-start.mp3',
      victory: '/sounds/game-win-long.mp3',
      defeat: '/sounds/game-lose-long.mp3',
      draw: '/sounds/game-draw.mp3',
    };

    const soundPath = soundMap[eventType];
    if (!soundPath) return;

    const adjustedVolume = isIOS() ? volume * 0.8 : volume;
    const audio = new Audio(soundPath);
    audio.volume = Math.max(0, Math.min(1, adjustedVolume));
    audio.play().catch(() => {
      // Silently fail - user may not have interacted with page yet
    });
  } catch (error) {
    console.error('[ChessSounds] Error playing game sound:', error);
  }
}
