interface AnalyzeButtonProps {
  onClick: () => void;
  isAnalyzing: boolean;
  isAnalyzed: boolean;
  progress?: number;
  disabled?: boolean;
}

export default function AnalyzeButton({
  onClick,
  isAnalyzing,
  isAnalyzed,
  progress = 0,
  disabled = false
}: AnalyzeButtonProps) {
  if (isAnalyzed) {
    return (
      <button
        disabled
        className="flex items-center gap-2 px-4 py-2 bg-slate-700 text-slate-400 rounded-lg font-medium cursor-not-allowed"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
        </svg>
        Analyzed
      </button>
    );
  }

  if (isAnalyzing) {
    return (
      <button
        disabled
        className="flex items-center gap-2 px-4 py-2 bg-slate-700 text-white rounded-lg font-medium cursor-not-allowed"
      >
        <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
        <span>Analyzing{progress > 0 ? ` ${progress}%` : '...'}</span>
      </button>
    );
  }

  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className="flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-emerald-500 to-teal-500 hover:from-emerald-400 hover:to-teal-400 text-white rounded-lg font-medium transition-all duration-200 shadow-[0_0_12px_rgba(16,185,129,0.3)] disabled:opacity-50 disabled:cursor-not-allowed"
    >
      <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
      Analyze Game
    </button>
  );
}
