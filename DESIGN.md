# ParkHub Design System

> Version 4.8.0 — Tailwind CSS 4, OKLCH, View Transitions, Container Queries

## Design Philosophy

ParkHub follows a **domain-aware material design** language. The interface adapts its color palette to the use-case context (residential, shared, rental, personal) while maintaining consistent interaction patterns across all modes.

**Creative direction:** A clean, modern productivity tool that feels premium but not flashy. Glass morphism and subtle gradients provide depth without visual noise. The design prioritizes information density and scan-ability for daily parking operations.

## Color System

All colors use **OKLCH** (perceptually uniform, wide gamut, better gradient interpolation than hex/rgb).

### Brand Palette — Teal

| Token | Value | Usage |
|-------|-------|-------|
| `primary-50..950` | oklch(0.97..0.23, 0.02..0.05, 175) | Primary actions, navigation active state |
| `accent-50..700` | oklch(0.98..0.52, 0.02..0.13, 90..55) | Highlights, CTAs, badges |

### Surface Palette — Slate

| Token | Value | Usage |
|-------|-------|-------|
| `surface-50..950` | oklch(0.98..0.14, 0.005..0.02, 260) | Backgrounds, cards, borders |

### Status Colors

| Token | Value | Usage |
|-------|-------|-------|
| `success` | oklch(0.65 0.17 160) | Confirmed bookings, active status |
| `warning` | oklch(0.74 0.16 75) | Pending, credits low |
| `danger` | oklch(0.58 0.22 25) | Cancelled, errors, expired |
| `info` | oklch(0.58 0.18 260) | Informational, neutral alerts |

### Use-Case Adaptive Palettes

The primary palette shifts based on `data-usecase` attribute:

| Use-Case | Hue Shift | Character |
|----------|-----------|-----------|
| Default (teal) | 175 | Universal parking |
| Residential (green) | 155 | Home/apartment complexes |
| Shared (purple) | 290 | Co-working, shared office |
| Rental (blue) | 260 | Commercial rental lots |
| Personal (red) | 10-15 | Individual/private use |

## Semantic Tokens

Components reference semantic tokens, not raw palette colors:

```
--theme-bg          Light: surface-50   Dark: surface-950
--theme-bg-subtle   Light: white        Dark: surface-900
--theme-bg-muted    Light: surface-100  Dark: surface-800
--theme-text        Light: surface-900  Dark: surface-100
--theme-text-muted  Light: surface-500  Dark: surface-400
--theme-border      Light: surface-200  Dark: surface-800
--theme-card-bg     Light: white        Dark: surface-900
```

### Glass Morphism Tokens

```
--glass-bg      Light: rgba(255,255,255,0.7)  Dark: rgba(15,23,42,0.6)
--glass-border  Light: rgba(255,255,255,0.3)  Dark: rgba(255,255,255,0.06)
--glass-blur    Light: 12px                    Dark: 16px
--glass-shadow  Light: 0 8px 32px rgba(0,0,0,0.06)
                Dark: 0 8px 32px rgba(0,0,0,0.3)
```

## Design Themes

12 built-in themes via `data-design-theme` attribute:

| Theme | Character |
|-------|-----------|
| Classic | Clean, minimal, professional |
| Glass | Frosted glass morphism with blur |
| Bento | Grid-focused, Japanese-inspired |
| Brutalist | Raw, bold, high contrast |
| Neon | Vibrant glow effects on dark |
| Warm | Earthy tones, soft gradients |
| Liquid | Fluid shapes, organic feel |
| Mono | Grayscale with single accent |
| Ocean | Deep blue maritime palette |
| Forest | Green earth tones |
| Synthwave | Retro 80s neon gradient |
| Zen | Minimal, whitespace-heavy |

## Typography

```
Font Stack: "Inter var", Inter, system-ui, -apple-system, "Segoe UI",
            Roboto, "Noto Sans", "Liberation Sans", sans-serif
Features:   cv11 (contextual alt), ss01 (open digits)
Optical:    font-optical-sizing: auto
Spacing:    letter-spacing: -0.011em (body)
```

### Text Wrapping (2026 CSS)

- **Headings (h1-h3):** `text-wrap: balance` — prevents orphans
- **Body (p, li):** `text-wrap: pretty` — smarter line breaks

## Spacing & Sizing

| Token | Value |
|-------|-------|
| `--radius-sm` | 0.375rem |
| `--radius-md` | 0.5rem |
| `--radius-lg` | 0.75rem |
| `--radius-xl` | 1rem |
| `--radius-2xl` | 1.25rem |
| `--radius-full` | 9999px |

### Shadows (progressive depth)

- `--shadow-xs`: subtle lift (cards at rest)
- `--shadow-sm`: slight elevation (hover state)
- `--shadow-md`: medium depth (dropdowns)
- `--shadow-lg`: high elevation (modals)
- `--shadow-xl`: maximum depth (overlays)

### Animation

- `--animate-fast`: 150ms (micro-interactions)
- `--animate-normal`: 250ms (state transitions)
- `--animate-slow`: 400ms (page transitions)

## Component Library

All components are CSS utility-based (`@utility` in Tailwind v4):

### Cards

| Utility | Usage |
|---------|-------|
| `glass-card` | Frosted glass morphism surface with blur |
| `card` | Solid card with subtle border and shadow |
| `stat-card` | Numeric KPI display card |
| `gradient-border` | Card with animated gradient border via mask |

### Buttons

| Utility | Usage |
|---------|-------|
| `btn` | Base button (flex, padding, radius, transitions) |
| `btn-primary` | Primary action (primary-700 bg) |
| `btn-secondary` | Secondary action (muted bg + border) |
| `btn-ghost` | Minimal (transparent bg) |
| `btn-sm` | Compact variant |
| `btn-icon` | Square icon-only button |
| `btn-shimmer` | Animated loading state |

### Badges

| Utility | Usage |
|---------|-------|
| `badge` | Base inline badge |
| `badge-success` | Confirmed/active |
| `badge-warning` | Pending/low |
| `badge-error` | Error/cancelled |
| `badge-info` | Informational |
| `badge-primary` | Brand color |
| `badge-gray` | Neutral |

### Form

| Utility | Usage |
|---------|-------|
| `input` | Text input with focus ring (primary-500) |

### Loading

| Utility | Usage |
|---------|-------|
| `skeleton` | Shimmer loading placeholder |
| `pulse-dot` | Live status indicator |

## Cutting-Edge CSS (2026)

### View Transitions

```css
@view-transition { navigation: auto; }
```

Smooth page transitions between routes — no JavaScript needed.

### Container Queries

```css
@utility container-card { container-type: inline-size; container-name: card; }
@utility container-panel { container-type: inline-size; container-name: panel; }
```

Self-responsive inner components that adapt to their container, not viewport.

### Scroll-Driven Animations

```css
@utility scroll-reveal {
  animation: slide-in-up linear both;
  animation-timeline: view();
  animation-range: entry 0% entry 40%;
}
```

Elements animate in as they enter the viewport — zero JavaScript, progressive enhancement.

### OKLCH Color Space

All colors defined in OKLCH for perceptually uniform gradients and wider gamut support.

## Accessibility

- **Focus rings:** 2px solid primary-500, 2px offset on all interactive elements
- **Touch targets:** 44px minimum on mobile (btn-icon, interactive elements)
- **Reduced motion:** All animations disabled via `prefers-reduced-motion: reduce`
- **Color contrast:** Semantic tokens ensure WCAG AA compliance in both light/dark
- **Print styles:** Booking pass optimized for A4 print

## Icons

[Phosphor Icons](https://phosphoricons.com/) — thin-stroke, consistent line weight, React components.

## Responsive Strategy

1. **Mobile-first** with Tailwind breakpoints (sm, md, lg, xl, 2xl)
2. **Container queries** for self-responsive components (cards, panels)
3. **5 mobile viewports** tested: iPhone 14 Pro, iPhone 15 Pro Max, Galaxy S24, iPad Pro, Pixel 8
4. **3 Playwright projects:** Desktop Chrome, Pixel 5, iPhone 14
