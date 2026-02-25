# Alpine Chess — Frontend Style Guide

Reference design: `DashboardPage.tsx`

---

## Foundations

### Colors

| Role | Token | Hex |
|------|-------|-----|
| **Page background** | `bg-slate-950` | `#020617` |
| **Card background** | `.card` class (gradient slate-900/950) | — |
| **Card border** | `border-emerald-500/40` | `rgba(16,185,129,0.4)` |
| **Card glow** | `shadow-[0_0_30px_rgba(16,185,129,0.15)]` | — |
| **Primary accent** | `emerald-400` / `emerald-500` | `#34d399` / `#10b981` |
| **Secondary accent** | `teal-400` / `teal-500` | `#2dd4bf` / `#14b8a6` |
| **Positive** | `emerald-400` | `#34d399` |
| **Negative** | `red-400` | `#f87171` |
| **Warning** | `amber-400` | `#fbbf24` |
| **Neutral** | `slate-400` | `#94a3b8` |
| **Primary text** | `text-white` | `#ffffff` |
| **Secondary text** | `text-slate-400` / `text-slate-500` | `#94a3b8` / `#64748b` |
| **Tertiary text** | `text-slate-600` | `#475569` |
| **Muted text** | `text-slate-700` | `#334155` |
| **Divider** | — | `rgba(51,65,85,0.4)` |
| **Divider (negative)** | — | `rgba(239,68,68,0.25)` |

### Typography

| Element | Font | Weight | Size |
|---------|------|--------|------|
| **Body / UI** | DM Sans | 400–700 | — |
| **Data / Numbers** | JetBrains Mono | 400–600 | — |
| **Page heading** | DM Sans | 700 (`font-bold`) | `text-2xl` |
| **Card heading** | DM Sans | 600 (`font-semibold`) | `text-sm` |
| **Metric label** | DM Sans | 500 (`font-medium`) | `text-[10px] uppercase tracking-[0.12em]` |
| **Metric value** | JetBrains Mono | 700 | `text-3xl font-mono` |
| **Metric subtitle** | DM Sans | 400 | `text-[11px]` |
| **Body text** | DM Sans | 400–500 | `text-sm` |
| **Small label** | DM Sans | 400 | `text-xs` / `text-[10px]` |
| **Inline data** | JetBrains Mono | 400–600 | `text-xs` / `text-sm` |

**Rules:**
- Numbers, percentages, ratings, and data values always use `font-mono`
- Labels and descriptions use the default sans (DM Sans)
- Uppercase labels get `tracking-[0.12em]` letter spacing
- Never use Inter, Roboto, Arial, or system fonts

### Spacing

| Context | Value |
|---------|-------|
| **Page padding** | `p-4 sm:p-6` |
| **Page max-width** | `max-w-5xl mx-auto` |
| **Section gap** | `mb-4` (tight) / `mb-6` (loose) |
| **Card padding** | `p-5` |
| **Grid gap** | `gap-4` |
| **Row items** | `gap-2` to `gap-3` |
| **Inline items** | `gap-1.5` to `gap-2` |

---

## Components

### Cards

Always use the `.card` class — never inline `bg-slate-800/50 border border-slate-700`.

```tsx
<div className="card p-5">
  <h2 className="text-sm font-semibold text-white mb-3">Card Title</h2>
  {/* content */}
</div>
```

The `.card` class provides:
- Gradient background (`from-slate-900/95 via-slate-950/95 to-slate-900/95`)
- 2px emerald border at 40% opacity
- Emerald glow shadow
- Backdrop blur
- `rounded-xl`

### Metric Cards (Hero Row)

4-column grid on desktop, 2-column on mobile:

```tsx
<div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
  <div className="card p-5">
    <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500 font-medium mb-2">
      Label
    </p>
    <p className="text-3xl font-bold text-white font-mono leading-none">
      1234
    </p>
    <p className="text-[11px] text-slate-600 mt-1.5">Subtitle</p>
  </div>
</div>
```

**Pattern:** uppercase micro-label → big mono number → muted subtitle.

### Gauge Ring (SVG)

For percentage metrics like accuracy:

```tsx
const GAUGE_R = 50;
const GAUGE_C = 2 * Math.PI * GAUGE_R; // ~314.16
const offset = GAUGE_C * (1 - value / 100);

<div className="relative w-20 h-20">
  <svg viewBox="0 0 120 120" style={{ transform: 'rotate(-90deg)' }}>
    <circle cx="60" cy="60" r={GAUGE_R} fill="none"
      strokeWidth="7" stroke="rgba(51,65,85,0.5)" />
    <circle cx="60" cy="60" r={GAUGE_R} fill="none"
      strokeWidth="7" stroke="#34d399" strokeLinecap="round"
      strokeDasharray={GAUGE_C} strokeDashoffset={offset} />
  </svg>
  <div className="absolute inset-0 flex items-center justify-center">
    <span className="text-xl font-bold text-emerald-400 font-mono">
      {value}<span className="text-sm">%</span>
    </span>
  </div>
</div>
```

### Stacked Horizontal Bar

For breakdowns (move quality, etc.). Pure CSS, no charting library:

```tsx
<div className="flex items-center gap-1 h-8 rounded-lg overflow-hidden">
  {segments.map((seg, i) => (
    <div
      key={seg.key}
      className={`h-full relative ${i === 0 ? 'rounded-l-md' : ''} ${i === last ? 'rounded-r-md' : ''}`}
      style={{ width: `${seg.pct}%`, backgroundColor: seg.color }}
    >
      {seg.pct >= 4 && (
        <div className="absolute inset-0 flex items-center justify-center
          font-mono font-semibold text-[10px] text-white/90">
          {seg.pct >= 10 ? `${seg.label} ${seg.pct}%` : `${seg.pct}%`}
        </div>
      )}
    </div>
  ))}
</div>
```

**Rules:**
- >= 10% width: show short label + percentage
- >= 4% width: show percentage only
- < 4% width: no label
- Light backgrounds (`good`, `inaccuracy`) use `text-slate-800/80`
- Dark backgrounds use `text-white/90`

### Legends

Centered below charts/bars, using small dots:

```tsx
<div className="flex gap-4 mt-3 text-[10px] text-slate-500 justify-center flex-wrap">
  <span className="flex items-center gap-1.5">
    <span className="w-2 h-2 rounded-sm" style={{ backgroundColor: color }} />
    Label
  </span>
</div>
```

For line chart legends, use thin lines instead of dots:
```tsx
<span className="w-2 h-[2px] bg-blue-400 inline-block rounded" />
```

### Divider Lines

Thin 1px lines between header and content:

```tsx
// Neutral (emerald/default sections)
<div className="h-px mb-3" style={{ background: 'rgba(51,65,85,0.4)' }} />

// Negative (red sections)
<div className="h-px mb-3" style={{ background: 'rgba(239,68,68,0.25)' }} />
```

### List Rows (Game Lists, Opening Lines)

Tight rows with monospace ranking, hover state:

```tsx
<div className="flex items-center justify-between px-2 py-2.5 rounded-lg
  hover:bg-slate-700/25 transition-colors">
  <div className="flex items-center gap-2.5">
    <span className="text-[11px] text-slate-600 font-mono w-4 text-right">
      {index + 1}
    </span>
    <div>
      <div className="flex items-center gap-1.5">
        <span className="text-[13px] text-white font-medium">Primary text</span>
        <span className="text-[10px] text-slate-600 font-mono">(secondary)</span>
      </div>
      <div className="flex items-center gap-2 text-[10px]">
        <span className="text-emerald-400">Status</span>
        <span className="text-slate-700">&middot;</span>
        <span className="text-slate-600">Date</span>
      </div>
    </div>
  </div>
  <span className="text-sm font-bold text-emerald-400 font-mono">Value</span>
</div>
```

**Pattern:**
- Rank number: `text-[11px] font-mono text-slate-600 w-4 text-right`
- Primary: `text-[13px] text-white font-medium`
- Secondary: `text-[10px] text-slate-600 font-mono`
- Meta row: `text-[10px]` with middot separators
- Trailing value: `text-sm font-bold font-mono` in accent color
- Hover: `hover:bg-slate-700/25`

---

## Charts (Recharts)

### Shared Config

```tsx
// Grid
<CartesianGrid strokeDasharray="3 3" stroke="rgba(51,65,85,0.25)" />

// Axes — no axis lines, no tick lines, slate-600 labels
<XAxis dataKey="date"
  tick={{ fill: '#475569', fontSize: 10 }}
  axisLine={false} tickLine={false}
  interval="preserveStartEnd" />

// Tooltip
<Tooltip content={<CustomTooltip />} />
```

### Chart Card Layout

```tsx
<div className="card p-5">
  <div className="flex items-baseline justify-between mb-4">
    <h2 className="text-sm font-semibold text-white">Chart Title</h2>
    <span className="text-[10px] text-slate-600 font-mono">Subtitle</span>
  </div>
  <div className="h-48">
    <ResponsiveContainer width="100%" height="100%">
      {/* chart */}
    </ResponsiveContainer>
  </div>
</div>
```

### Line Styles

| Chart | Color | Width |
|-------|-------|-------|
| Accuracy | `#34d399` (emerald-400) | 2 |
| Opening phase | `#60a5fa` (blue-400) | 1.5 |
| Middlegame phase | `#fbbf24` (amber-400) | 1.5 |
| Endgame phase | `#f87171` (red-400) | 1.5 |
| Inaccuracy | `#fbbf24` (amber-400) | 1.5 |
| Mistake | `#fb923c` (orange-400) | 1.5 |
| Blunder | `#f87171` (red-400) | 1.5 |
| Rating | `#a78bfa` (violet-400) | 2 |

All lines: `type="monotone"`, `dot={false}`, `activeDot={{ r: 3 }}`, `isAnimationActive={false}`.

### Custom Tooltip

```tsx
<div className="bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 shadow-xl">
  <p className="text-slate-400 text-[11px] mb-1">{label}</p>
  <p className="text-white font-mono text-xs">{name}: {value}</p>
</div>
```

---

## Layout Patterns

### 2-Column Grid (Primary)

Used for charts, game lists, opening habits:

```tsx
<div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
```

### 4-Column Grid (Hero Metrics)

```tsx
<div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
```

### Full-Width Section

For elements that span the full width (move quality bar):

```tsx
<div className="card p-5 mb-4">
```

---

## Semantic Color Assignments

| Meaning | Color |
|---------|-------|
| Win / Positive / Good | `emerald-400` (`#34d399`) |
| Loss / Negative / Bad | `red-400` (`#f87171`) |
| Draw / Neutral | `slate-400` (`#94a3b8`) |
| Warning / Inaccuracy | `amber-400` (`#fbbf24`) |
| Mistake | `orange-400`–`orange-500` |
| Book moves | `cyan-500` (`#06b6d4`) |
| Rating | `violet-400` (`#a78bfa`) |
| Opening phase | `blue-400` (`#60a5fa`) |

---

## Buttons

Defined in `index.css`:

```
.btn             — base: px-4 py-2 rounded-lg font-medium transition-colors
.btn-primary     — emerald/teal gradient + glow shadow
.btn-secondary   — bg-slate-800 hover:bg-slate-700
.btn-ghost       — transparent hover:bg-slate-800
```

For inline actions (like list row clicks), use:
```tsx
className="hover:bg-slate-700/25 transition-colors cursor-pointer"
```

---

## Do / Don't

**Do:**
- Use `.card` class for all containers
- Use `font-mono` for all numbers and data values
- Use `text-[10px]` / `text-[11px]` for micro labels
- Use 2-column grids to maximize information density
- Use inline `style` for dynamic widths/colors (stacked bars)
- Keep chart heights compact: `h-44` to `h-48`
- Use `activeDot={{ r: 3 }}` for chart hover feedback
- Use rgba inline styles for divider colors

**Don't:**
- Use `bg-slate-800/50 border border-slate-700` for cards (use `.card`)
- Use gradients on chart fills or card backgrounds beyond the `.card` class
- Use text larger than `text-sm` for card headings
- Use `text-lg` or `text-xl` inside cards (reserved for page-level headings only)
- Use axis lines or tick lines on charts
- Add chart animations (`isAnimationActive={false}`)
- Use orange for costliest habits (use red-400)
- Mix DM Sans for numbers — always `font-mono` for data
