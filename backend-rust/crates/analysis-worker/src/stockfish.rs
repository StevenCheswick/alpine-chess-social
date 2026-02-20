//! Stockfish engine wrapper using UCI protocol (async I/O)

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use tracing::debug;

use crate::error::WorkerError;

/// Result of a single position evaluation
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Centipawn score (from engine's perspective, i.e., side to move)
    pub cp: Option<i32>,
    /// Mate in N moves (positive = engine wins, negative = engine loses)
    pub mate: Option<i32>,
    /// Best move in UCI notation
    pub best_move: String,
}

/// A single PV line from multi-PV analysis
#[derive(Debug, Clone)]
pub struct PvLine {
    /// Principal variation moves
    pub pv: Vec<String>,
    /// Centipawn score
    pub cp: Option<i32>,
    /// Mate in N
    pub mate: Option<i32>,
}

/// Stockfish engine instance
pub struct StockfishEngine {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StockfishEngine {
    /// Spawn a new Stockfish process and initialize UCI
    pub async fn new(path: &str) -> Result<Self, WorkerError> {
        let mut process = Command::new(path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| WorkerError::Stockfish(format!("Failed to spawn Stockfish: {e}")))?;

        let stdin = process.stdin.take().unwrap();
        let stdout = BufReader::new(process.stdout.take().unwrap());

        let mut engine = Self {
            process,
            stdin,
            stdout,
        };

        // Initialize UCI
        engine.send("uci").await?;
        engine.wait_for("uciok").await?;

        // Configure for analysis
        engine.send("setoption name Threads value 1").await?;
        engine.send("setoption name Hash value 256").await?;
        engine.send("setoption name UCI_AnalyseMode value true").await?;
        engine.send("isready").await?;
        engine.wait_for("readyok").await?;

        Ok(engine)
    }

    /// Send a command to Stockfish
    async fn send(&mut self, cmd: &str) -> Result<(), WorkerError> {
        debug!(cmd, "SF <");
        self.stdin
            .write_all(format!("{cmd}\n").as_bytes())
            .await
            .map_err(|e| WorkerError::Stockfish(format!("Failed to write to Stockfish: {e}")))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| WorkerError::Stockfish(format!("Failed to flush stdin: {e}")))?;
        Ok(())
    }

    /// Wait for a specific response line
    async fn wait_for(&mut self, expected: &str) -> Result<(), WorkerError> {
        let mut line = String::new();
        loop {
            line.clear();
            self.stdout
                .read_line(&mut line)
                .await
                .map_err(|e| WorkerError::Stockfish(format!("Failed to read from Stockfish: {e}")))?;
            let trimmed = line.trim();
            debug!(line = trimmed, "SF >");
            if trimmed == expected {
                return Ok(());
            }
        }
    }

    /// Evaluate a position and get the best move with score
    pub async fn evaluate(&mut self, fen: &str, nodes: u32) -> Result<EvalResult, WorkerError> {
        self.send(&format!("position fen {fen}")).await?;
        self.send(&format!("go nodes {nodes}")).await?;

        let mut result = EvalResult {
            cp: None,
            mate: None,
            best_move: String::new(),
        };

        let mut line = String::new();
        loop {
            line.clear();
            self.stdout
                .read_line(&mut line)
                .await
                .map_err(|e| WorkerError::Stockfish(format!("Failed to read from Stockfish: {e}")))?;
            let trimmed = line.trim();

            if trimmed.starts_with("info") && trimmed.contains(" pv ") {
                // Parse score from info line
                if let Some(cp) = parse_cp(trimmed) {
                    result.cp = Some(cp);
                    result.mate = None;
                }
                if let Some(mate) = parse_mate(trimmed) {
                    result.mate = Some(mate);
                    result.cp = None;
                }
            } else if trimmed.starts_with("bestmove") {
                // Parse best move
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    result.best_move = parts[1].to_string();
                }
                break;
            }
        }

        Ok(result)
    }

    /// Evaluate a position with multiple PV lines (for puzzle extension)
    pub async fn evaluate_multipv(
        &mut self,
        fen: &str,
        nodes: u32,
        multipv: u32,
    ) -> Result<Vec<PvLine>, WorkerError> {
        self.send(&format!("setoption name MultiPV value {multipv}")).await?;
        self.send(&format!("position fen {fen}")).await?;
        self.send(&format!("go nodes {nodes}")).await?;

        let mut lines: Vec<PvLine> = vec![
            PvLine {
                pv: vec![],
                cp: None,
                mate: None
            };
            multipv as usize
        ];
        let mut line = String::new();

        loop {
            line.clear();
            self.stdout
                .read_line(&mut line)
                .await
                .map_err(|e| WorkerError::Stockfish(format!("Failed to read from Stockfish: {e}")))?;
            let trimmed = line.trim();

            if trimmed.starts_with("info") && trimmed.contains(" pv ") {
                // Parse multipv index (1-based)
                let pv_idx = parse_multipv_index(trimmed).unwrap_or(1) - 1;
                if (pv_idx as usize) < lines.len() {
                    let entry = &mut lines[pv_idx as usize];
                    entry.cp = parse_cp(trimmed);
                    entry.mate = parse_mate(trimmed);
                    entry.pv = parse_pv(trimmed);
                }
            } else if trimmed.starts_with("bestmove") {
                break;
            }
        }

        // Reset MultiPV to 1
        self.send("setoption name MultiPV value 1").await?;

        Ok(lines)
    }

    /// Send quit command and wait for process to exit
    pub async fn quit(&mut self) {
        let _ = self.send("quit").await;
        let _ = self.process.wait().await;
    }
}

impl Drop for StockfishEngine {
    fn drop(&mut self) {
        // Best-effort synchronous kill in drop
        let _ = self.process.start_kill();
    }
}

/// Parse centipawn score from info line
fn parse_cp(line: &str) -> Option<i32> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "cp" && i + 1 < parts.len() {
            return parts[i + 1].parse().ok();
        }
    }
    None
}

/// Parse mate score from info line
fn parse_mate(line: &str) -> Option<i32> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "mate" && i + 1 < parts.len() {
            return parts[i + 1].parse().ok();
        }
    }
    None
}

/// Parse multipv index from info line
fn parse_multipv_index(line: &str) -> Option<u32> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "multipv" && i + 1 < parts.len() {
            return parts[i + 1].parse().ok();
        }
    }
    None
}

/// Parse PV moves from info line
fn parse_pv(line: &str) -> Vec<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    let mut in_pv = false;
    let mut moves = Vec::new();

    for part in parts {
        if part == "pv" {
            in_pv = true;
            continue;
        }
        if in_pv {
            // PV ends at next keyword or end of line
            if part.starts_with("bmc") || part == "string" {
                break;
            }
            moves.push(part.to_string());
        }
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cp() {
        let line = "info depth 20 seldepth 25 multipv 1 score cp 35 nodes 100000 pv e2e4";
        assert_eq!(parse_cp(line), Some(35));
    }

    #[test]
    fn test_parse_mate() {
        let line = "info depth 20 score mate 3 nodes 100000 pv e2e4";
        assert_eq!(parse_mate(line), Some(3));
    }

    #[test]
    fn test_parse_pv() {
        let line = "info depth 20 score cp 35 pv e2e4 e7e5 g1f3";
        let pv = parse_pv(line);
        assert_eq!(pv, vec!["e2e4", "e7e5", "g1f3"]);
    }
}
