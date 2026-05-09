// Maia WASM inference — ported from feature-testing/maia-bot/index.html.
// Loads rating-bucketed ONNX models lazily and samples a move from the policy head.
//
// Usage:
//   const maia = new MaiaEngine({ modelsBaseUrl: '/models/maia' });
//   await maia.loadModel(1500);
//   const result = await maia.pickMove({ fen, historyFens, rating: 1500 });
//   // result.chosen: chess.js-style move object, result.ranked: top-N with probs

const ORT_WASM_BASE = 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.21.0/dist/';
const FILES = 'abcdefgh';
const PT_MAP = { p: 0, n: 1, b: 2, r: 3, q: 4, k: 5 };

export class MaiaEngine {
  constructor({ modelsBaseUrl = '/models/maia', onProgress = null } = {}) {
    this.modelsBaseUrl = modelsBaseUrl;
    this.onProgress = onProgress;
    this.sessions = {};
    this.lc0MoveIndex = null;
    this._ortReady = false;
  }

  async _ensureOrt() {
    if (this._ortReady) return;
    if (typeof ort === 'undefined') {
      throw new Error('onnxruntime-web (ort) not loaded — include the CDN script before MaiaEngine');
    }
    ort.env.wasm.wasmPaths = ORT_WASM_BASE;
    this._ortReady = true;
  }

  async _ensureMoveIndex() {
    if (this.lc0MoveIndex) return;
    const resp = await fetch(`${this.modelsBaseUrl}/lc0_moves.json`);
    if (!resp.ok) throw new Error(`Failed to load lc0_moves.json: ${resp.status}`);
    this.lc0MoveIndex = await resp.json();
  }

  async loadModel(rating) {
    await this._ensureOrt();
    await this._ensureMoveIndex();
    if (this.sessions[rating]) return this.sessions[rating];

    const url = `${this.modelsBaseUrl}/maia-${rating}.onnx`;
    const resp = await fetch(url);
    if (!resp.ok) throw new Error(`Failed to fetch ${url}: ${resp.status}`);

    const total = parseInt(resp.headers.get('Content-Length') || '0');
    const reader = resp.body.getReader();
    const chunks = [];
    let loaded = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      chunks.push(value);
      loaded += value.length;
      if (this.onProgress) this.onProgress({ rating, loaded, total, phase: 'download' });
    }
    const buf = new Uint8Array(loaded);
    let off = 0;
    for (const c of chunks) { buf.set(c, off); off += c.length; }

    if (this.onProgress) this.onProgress({ rating, phase: 'init' });
    const sess = await ort.InferenceSession.create(buf.buffer, { executionProviders: ['wasm'] });
    this.sessions[rating] = sess;
    if (this.onProgress) this.onProgress({ rating, phase: 'done' });
    return sess;
  }

  _parseFenBoard(fen) {
    const ranks = fen.split(' ')[0].split('/');
    const board = new Array(64).fill(null);
    for (let ri = 0; ri < 8; ri++) {
      const rank = 7 - ri;
      let file = 0;
      for (const ch of ranks[ri]) {
        if (ch >= '1' && ch <= '8') file += parseInt(ch);
        else {
          board[rank * 8 + file] = { type: ch.toLowerCase(), isWhite: ch === ch.toUpperCase() };
          file++;
        }
      }
    }
    return board;
  }

  _fillHistorySlot(data, slotIdx, boardArr, sideIsWhite) {
    const base = slotIdx * 13;
    for (let sq = 0; sq < 64; sq++) {
      const piece = boardArr[sq];
      if (!piece) continue;
      let tsq = sq;
      if (!sideIsWhite) {
        const r = Math.floor(sq / 8);
        const f = sq % 8;
        tsq = (7 - r) * 8 + f;
      }
      const row = Math.floor(tsq / 8);
      const col = tsq % 8;
      const isOurs = piece.isWhite === sideIsWhite;
      const plane = base + (isOurs ? 0 : 6) + PT_MAP[piece.type];
      data[plane * 64 + row * 8 + col] = 1.0;
    }
  }

  _boardToLc0Planes(fen, historyFens) {
    const parts = fen.split(' ');
    const castling = parts[2];
    const isWhite = parts[1] === 'w';
    const data = new Float32Array(112 * 64);

    this._fillHistorySlot(data, 0, this._parseFenBoard(fen), isWhite);
    if (historyFens) {
      for (let h = 0; h < Math.min(historyFens.length, 7); h++) {
        this._fillHistorySlot(data, h + 1, this._parseFenBoard(historyFens[h]), isWhite);
      }
    }

    if (isWhite) {
      if (castling.includes('Q')) for (let i = 0; i < 64; i++) data[104 * 64 + i] = 1;
      if (castling.includes('K')) for (let i = 0; i < 64; i++) data[105 * 64 + i] = 1;
      if (castling.includes('q')) for (let i = 0; i < 64; i++) data[106 * 64 + i] = 1;
      if (castling.includes('k')) for (let i = 0; i < 64; i++) data[107 * 64 + i] = 1;
    } else {
      if (castling.includes('q')) for (let i = 0; i < 64; i++) data[104 * 64 + i] = 1;
      if (castling.includes('k')) for (let i = 0; i < 64; i++) data[105 * 64 + i] = 1;
      if (castling.includes('Q')) for (let i = 0; i < 64; i++) data[106 * 64 + i] = 1;
      if (castling.includes('K')) for (let i = 0; i < 64; i++) data[107 * 64 + i] = 1;
    }

    const halfmove = parseInt(parts[4]) || 0;
    if (halfmove > 0) {
      const hv = halfmove / 100.0;
      for (let i = 0; i < 64; i++) data[109 * 64 + i] = hv;
    }
    for (let i = 0; i < 64; i++) data[111 * 64 + i] = 1.0;
    return data;
  }

  _moveToLc0Uci(from, to, promotion, isBlack) {
    let fromFile = from.charCodeAt(0) - 97;
    let fromRank = parseInt(from[1]) - 1;
    let toFile = to.charCodeAt(0) - 97;
    let toRank = parseInt(to[1]) - 1;
    if (isBlack) { fromRank = 7 - fromRank; toRank = 7 - toRank; }
    let uci = FILES[fromFile] + (fromRank + 1) + FILES[toFile] + (toRank + 1);
    if (promotion) {
      if (promotion !== 'q') {
        const withSuffix = uci + promotion;
        if (withSuffix in this.lc0MoveIndex) return withSuffix;
      }
      const withQ = uci + 'q';
      if (withQ in this.lc0MoveIndex) return withQ;
    }
    return uci;
  }

  async pickMove({ chess, historyFens = [], rating = 1500 }) {
    const session = await this.loadModel(rating);
    const fen = chess.fen();
    const isBlack = chess.turn() === 'b';
    const legalMoves = chess.moves({ verbose: true });
    if (!legalMoves.length) return null;

    const planes = this._boardToLc0Planes(fen, historyFens);
    const feeds = { '/input/planes': new ort.Tensor('float32', planes, [1, 112, 8, 8]) };
    const results = await session.run(feeds);
    const policyLogits = results['/output/policy'].data;

    const moveData = legalMoves.map(m => {
      const lc0Uci = this._moveToLc0Uci(m.from, m.to, m.promotion, isBlack);
      const idx = this.lc0MoveIndex[lc0Uci];
      const logit = idx !== undefined ? policyLogits[idx] : -1000;
      return { move: m, lc0Uci, idx, logit };
    });

    const logits = moveData.map(d => d.logit);
    const maxL = Math.max(...logits);
    const exps = logits.map(l => Math.exp(l - maxL));
    const sumExp = exps.reduce((a, b) => a + b, 0);
    moveData.forEach((d, i) => d.prob = exps[i] / sumExp);
    moveData.sort((a, b) => b.prob - a.prob);

    const r = Math.random();
    let cum = 0;
    let chosen = moveData[0];
    for (const d of moveData) {
      cum += d.prob;
      if (r < cum) { chosen = d; break; }
    }
    return { chosen, ranked: moveData.slice(0, 8), rating };
  }
}
