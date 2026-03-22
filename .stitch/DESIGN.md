# ParkHub Design System — The Kinetic Observatory

## 1. Overview & Creative North Star
**"The Kinetic Observatory"** — Beyond static utility dashboards. We curate a live, 
premium environment where data feels alive. Inspired by Linear, Vercel, and Raycast.

## 2. Colors & Surface Philosophy

### Palette (OKLCH)
- **Primary**: Teal `oklch(0.66 0.13 168)` / `#0d9488`
- **Primary Light**: `oklch(0.74 0.14 170)` / `#14b8a6`
- **Secondary**: Amber `oklch(0.74 0.16 75)` / `#f59e0b`
- **Surface Base**: Deep Slate `oklch(0.14 0.02 260)` / `#0f172a`
- **Surface Card**: `oklch(0.27 0.02 260)` / glass at 60% opacity
- **Success**: `oklch(0.65 0.17 160)` / `#22c55e`
- **Danger**: `oklch(0.58 0.22 25)` / `#ef4444`

### The "No-Line" Rule
1px borders are prohibited. Use tonal shifts, backdrop-blur, and ghost borders (outline at 15% opacity).

### Glass Morphism
- `backdrop-filter: blur(24px)`
- Background: surface at 60% alpha
- Ghost border: outline-variant at 15% opacity
- Ambient shadow: 40px blur, primary-tinted

## 3. Typography
- **Headlines**: Manrope, -0.02em tracking, bold
- **Body**: Inter, regular weight
- **Data/Numbers**: tabular-nums, Manrope
- **Scale**: display-lg for KPIs, headline-sm for card titles, body-md for data

## 4. Components

### Stat Cards (Bento Grid)
- Gradient border animation (primary → secondary at 135°)
- Animated counter on load (cubic ease-out)
- Inner glow on hover (primary at 15% opacity)
- Variable sizes: 1x1, 2x1, 1x2 in bento layout

### Buttons
- Primary: gradient fill (primary → primary-dark), rounded-lg, no border
- Secondary: ghost, outline at 20%, text in primary
- Hover: inner glow + scale(1.02)

### Charts
- Gradient fill under line (primary → transparent)
- Gridlines in surface-variant at 10%
- Animated path drawing on load

### Status Indicators
- Pulsing dot (2s ease-in-out infinite) for live data
- Color: success=green, warning=amber, danger=red
- Size: 8px with 20px glow radius

## 5. Motion
- Spring physics: stiffness 300, damping 30
- Stagger children: 50ms delay
- Page transitions: fade + translateY(12px)
- Hover: scale(1.02) + shadow elevation
- Duration: 250ms default, 150ms fast, 400ms slow

## 6. Do's and Don'ts
- DO use extreme whitespace between sections
- DO use primary-dim for chart lines
- DO overlap elements for depth
- DON'T use pure white (#fff) for text — use on-surface
- DON'T use standard dividers — use spacing or tonal shifts
- DON'T use sharp corners — minimum rounded-lg (1rem)
