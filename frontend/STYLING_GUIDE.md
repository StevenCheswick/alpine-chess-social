# Frontend Styling Guide

This document outlines the styling conventions and patterns used in the chess social media frontend, matching the Chess Wrapped visual style.

---

## Tech Stack

- **Tailwind CSS** v3.4.19 - Utility-first CSS framework
- **PostCSS** with Autoprefixer
- **Vite** for build tooling
- **Framer Motion** for animations

---

## Color Palette

### Primary Brand Color (Emerald/Teal/Cyan)

Uses Tailwind's default emerald palette:

| Token | Hex | Usage |
|-------|-----|-------|
| `emerald-400` | `#34d399` | Links, accents, interactive text |
| `emerald-500` | `#10b981` | Hover states on buttons |
| `emerald-600` | `#059669` | Primary buttons, CTAs |
| `emerald-900` | `#064e3b` | Dark gradient backgrounds |

### Gradient Accent (Emerald -> Teal -> Cyan)

```jsx
// Gradient text for headings
<h1 className="gradient-text">Title</h1>

// Manual gradient
<span className="text-transparent bg-clip-text bg-gradient-to-r from-emerald-400 via-teal-400 to-cyan-400">
```

### Background Colors (Dark Theme)

| Class | Usage |
|-------|-------|
| `bg-slate-950` | Page background (base) |
| `bg-slate-900` | Cards, elevated surfaces |
| `bg-slate-800` | Inputs, secondary surfaces |
| `bg-slate-700` | Hover states, tertiary surfaces |

### Text Colors

| Class | Usage |
|-------|-------|
| `text-white` | Primary text, headings |
| `text-slate-300` | Secondary text, labels |
| `text-slate-400` | Tertiary text, metadata |
| `text-slate-500` | Muted text, timestamps, helper text |

### Semantic Colors

| Purpose | Class |
|---------|-------|
| Success/Win | `text-green-400`, `text-green-500` |
| Error/Loss | `text-red-400`, `text-red-500` |
| Warning/Achievement | `text-amber-400` |
| Info/Loading | `text-blue-400` |

### Color with Opacity

Use Tailwind's opacity modifier for subtle backgrounds:

```
bg-red-500/10       # 10% opacity red
bg-amber-500/20     # 20% opacity amber
bg-emerald-500/40   # 40% opacity emerald (card borders)
border-red-500/20   # 20% opacity border
```

---

## Typography

### Font Stack

System fonts (no custom fonts):
```css
font-family: ui-sans-serif, system-ui, sans-serif;
```

### Text Sizes

| Class | Usage |
|-------|-------|
| `text-2xl font-bold` | Page titles |
| `text-xl font-bold` | Section headings |
| `text-lg font-semibold` | Subheadings |
| `text-base` | Body text (default) |
| `text-sm` | Secondary text, labels |
| `text-xs` | Metadata, timestamps, helper text |

### Font Weights

| Class | Usage |
|-------|-------|
| `font-bold` | Headlines, important data |
| `font-semibold` | Subheadings, emphasized text |
| `font-medium` | Labels, buttons, nav items |
| `font-normal` | Body text (default) |

### Common Typography Patterns

```jsx
// Label
<label className="block text-sm font-medium text-slate-300 mb-2">

// Timestamp
<span className="text-sm text-slate-500">

// Player info
<span className="font-medium text-sm text-white">
<span className="text-xs text-slate-400">(2000)</span>

// Gradient heading
<h1 className="text-2xl font-bold gradient-text">
```

---

## Spacing

### Standard Spacing Values

| Value | Pixels | Usage |
|-------|--------|-------|
| `1` | 4px | Micro spacing |
| `2` | 8px | Small gaps, tight spacing |
| `3` | 12px | Medium gaps |
| `4` | 16px | Standard padding, common gap |
| `6` | 24px | Large gaps, section spacing |
| `8` | 32px | Extra large spacing |

### Padding Patterns

| Element | Pattern |
|---------|---------|
| Cards | `p-4` or `p-6` |
| Buttons | `px-4 py-2` |
| Form inputs | `px-4 py-2` |
| Nav items | `px-4 py-3` |
| Modal content | `p-6` |
| List items | `p-3` or `py-2` |

### Gap Patterns

| Context | Pattern |
|---------|---------|
| Flex row items | `gap-2` to `gap-4` |
| Flex column items | `gap-3` to `gap-6` |
| Form fields | `space-y-4` |
| Card sections | `space-y-6` |

---

## Border Radius

| Class | Size | Usage |
|-------|------|-------|
| `rounded` | 4px | Small elements, form inputs |
| `rounded-lg` | 8px | Cards, buttons, medium elements |
| `rounded-xl` | 12px | Large cards, prominent sections |
| `rounded-full` | 50% | Circles, avatars, badges |

---

## Component Classes

### Defined in `src/index.css`

```css
/* Card container - with gradient bg and emerald glow */
.card {
  @apply bg-gradient-to-br from-slate-900/95 via-slate-950/95 to-slate-900/95
         rounded-xl border-2 border-emerald-500/40 backdrop-blur-sm
         shadow-[0_0_30px_rgba(16,185,129,0.15)];
}

/* Base button */
.btn {
  @apply px-4 py-2 rounded-lg font-medium transition-colors duration-200;
}

/* Button variants */
.btn-primary {
  @apply bg-emerald-600 hover:bg-emerald-500 text-white;
}

.btn-secondary {
  @apply bg-slate-800 hover:bg-slate-700 text-white;
}

.btn-ghost {
  @apply bg-transparent hover:bg-slate-800 text-slate-300;
}

/* Form input */
.input {
  @apply bg-slate-800 border border-slate-700 rounded-lg px-4 py-2 text-white
         placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-emerald-500
         focus:border-emerald-500 transition-colors;
}

/* Gradient text utility */
.gradient-text {
  @apply text-transparent bg-clip-text bg-gradient-to-r
         from-emerald-400 via-teal-400 to-cyan-400;
}

/* Emerald glow effect */
.glow-emerald {
  @apply shadow-[0_0_20px_rgba(16,185,129,0.3)];
}
```

---

## Common UI Patterns

### Card

```jsx
<div className="card p-4">
  {/* Card content */}
</div>
```

### Button with Loading State

```jsx
<button
  disabled={isLoading}
  className="btn btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
>
  {isLoading ? 'Loading...' : 'Submit'}
</button>
```

### Form Field

```jsx
<div>
  <label className="block text-sm font-medium text-slate-300 mb-2">
    Label
  </label>
  <input className="input w-full" placeholder="Placeholder..." />
  <p className="mt-1 text-xs text-slate-500">Helper text</p>
</div>
```

### Modal

```jsx
<div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
  <div className="card p-6 w-full max-w-md mx-4">
    {/* Modal content */}
  </div>
</div>
```

### Error Message

```jsx
<div className="p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
  {error}
</div>
```

### Loading Spinner

```jsx
<div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />

// Or with emerald accent
<div className="w-8 h-8 border-4 border-slate-700 border-t-emerald-500 rounded-full animate-spin" />
```

### Empty State

```jsx
<div className="text-center py-12 text-slate-500">
  <p>No items found</p>
</div>
```

### Tag/Badge

```jsx
<span className="px-2 py-0.5 bg-amber-500/10 border border-amber-500/20 rounded text-xs text-amber-400">
  Achievement
</span>

// Emerald variant
<span className="px-2 py-0.5 bg-emerald-400/20 text-emerald-400 rounded text-xs">
  Tag
</span>
```

### Tab Button

```jsx
<button className={`px-6 py-3 rounded-lg font-medium transition-colors ${
  isActive
    ? 'bg-emerald-600 text-white'
    : 'bg-slate-800 text-slate-300 hover:bg-slate-700'
}`}>
  Tab Label
</button>
```

### Avatar

```jsx
// Small
<div className="w-10 h-10 bg-slate-700 rounded-full" />

// Large
<div className="w-24 h-24 bg-slate-700 rounded-full" />
```

### Player Color Indicator

```jsx
// White pieces
<div className="w-2.5 h-2.5 rounded-full bg-white" />

// Black pieces
<div className="w-2.5 h-2.5 rounded-full bg-slate-900 border border-slate-600" />
```

### Link Styling

```jsx
// Standard link
<a className="text-emerald-400 hover:text-emerald-300">Link</a>
```

---

## Responsive Design

### Breakpoints

| Prefix | Min Width | Usage |
|--------|-----------|-------|
| `sm:` | 640px | Tablets |
| `lg:` | 1024px | Desktop |

### Common Patterns

```jsx
// Show on desktop, hide on mobile
<div className="hidden lg:block">

// Sidebar layout offset
<main className="pt-16 lg:pl-64">

// Full width mobile, constrained desktop
<div className="w-full lg:max-w-lg">

// Desktop-only text
<span className="hidden sm:block">
```

### Layout Structure

- **Mobile**: Bottom navigation bar (fixed)
- **Desktop**: Left sidebar + top navbar

---

## Animations & Transitions

### Transition Classes

| Class | Duration | Usage |
|-------|----------|-------|
| `transition-colors` | 150ms | Color changes on hover |
| `transition-transform` | 150ms | Rotations, scaling |
| `transition` | 150ms | General transitions |
| `duration-200` | 200ms | Slightly slower |
| `duration-300` | 300ms | Smooth animations |

### Animation Classes

| Class | Effect |
|-------|--------|
| `animate-spin` | Continuous rotation (loaders) |
| `animate-pulse` | Opacity pulse (notifications) |

### Common Patterns

```jsx
// Hover state transition
<button className="hover:bg-slate-800 transition-colors">

// Rotation toggle
<svg className={`transition-transform ${isOpen ? 'rotate-180' : ''}`} />

// Smooth height/progress
<div className="transition-all duration-300 ease-out" style={{ height: `${percent}%` }} />

// Pulse indicator
<span className="w-1.5 h-1.5 bg-blue-400 rounded-full animate-pulse" />
```

---

## State Patterns

### Hover States

```
hover:bg-slate-800
hover:bg-slate-700
hover:text-white
hover:bg-emerald-500
hover:text-emerald-300
```

### Focus States

```
focus:outline-none
focus:ring-2
focus:ring-emerald-500
focus:border-emerald-500
```

### Disabled States

```
disabled:opacity-50
disabled:cursor-not-allowed
```

---

## Chess-Specific Components

### Platform Badges

```jsx
// Chess.com
<div className="w-4 h-4 rounded bg-green-600" />

// Lichess
<div className="w-4 h-4 rounded bg-white" />
```

### Game Result Colors

```jsx
<span className={result === 'W' ? 'text-green-500' : 'text-red-500'}>
  {resultLabel}
</span>
```

### Eval Bar

```jsx
<div className="w-6 h-full bg-white rounded overflow-hidden flex flex-col">
  <div
    className="bg-slate-600 transition-all duration-300 ease-out"
    style={{ height: `${blackPercent}%` }}
  />
</div>
```

---

## File Structure

```
frontend/
├── src/
│   ├── index.css          # Global styles, Tailwind directives, component classes
│   ├── components/        # Reusable components
│   └── pages/             # Page components
├── tailwind.config.js     # Tailwind config (important: true)
├── postcss.config.js      # PostCSS configuration
└── vite.config.ts         # Build configuration
```

---

## Best Practices

1. **Use component classes** (`.card`, `.btn`, `.input`) for consistency
2. **Follow the color hierarchy**: white > slate-300 > slate-400 > slate-500
3. **Use emerald for interactive elements** (buttons, links, focus states)
4. **Use semantic colors** for status (green=success, red=error, amber=warning)
5. **Apply transitions** on interactive elements
6. **Use opacity modifiers** for subtle backgrounds (`bg-color/10`)
7. **Follow spacing patterns** consistently (gap-2/3/4, p-4/6)
8. **Use rounded-lg** for most elements, rounded-xl for cards
9. **Add disabled states** to all interactive elements
10. **Cards have emerald glow** - subtle shadow effect for premium feel
