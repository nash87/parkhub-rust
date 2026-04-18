# ParkHub Design Preview

Static HTML/CSS/JSX design prototype, served at `/preview/`.

## Origin

Exported from [claude.ai/design](https://claude.ai/design) on 2026-04-18 as a
handoff bundle. The designer iterated against snapshots of this repo's actual
`parkhub-web/` code (Dashboard, Book, AdminDashboard, Layout, theme tokens)
so the surfaces match the real app's visual language.

## What's here

Six linked surfaces, all wired into a single React 18 app using UMD + Babel
standalone (no build step needed — the browser compiles JSX at runtime):

| Surface | File | Notes |
|---|---|---|
| Marketing landing | `design/landing.jsx` | Hero, editions compare, features grid |
| Dashboard | `design/dashboard.jsx` | 5 KPIs, trend chart, live sensor feed, bookings |
| Book a spot | `design/book.jsx` | 3-step flow: lot → slot+time → confirm |
| Parking pass | `design/pass.jsx` | QR card, timeline, wallet/print/share |
| Admin lot editor | `design/admin.jsx` | Lot list, floor-plan editor, properties |
| Theme showcase | `design/showcase.jsx` | 12 themes × 5 use-case palettes |

Plus `design/shell.jsx` (sidebar + topbar), `design/icons.jsx` (icon set),
and `design/tokens.css` (OKLCH ramp derived from `DESIGN.md`).

## How to view

Start the dev server (`npm run dev` in `parkhub-web/`), then open
<http://localhost:4321/preview/>. A floating top-center screen picker jumps
between surfaces; a bottom-right Tweaks panel lets you swap use-case,
dark/light, edition (Rust/PHP), and theme — state persists in `localStorage`.

## Status

This is a **visual reference**, not production code. It deliberately uses
inline styles and CDN-loaded React so the bundle is fully self-contained and
editable without a Vite/Astro round-trip.

## Next steps (tracked in fop)

Port each surface from the prototype's inline-styled JSX into real TSX +
Tailwind 4 components that replace or augment the corresponding views under
`src/views/`. Preserve the design-token derivation (OKLCH ramp, use-case hue
shifts, semantic tokens) so the production app stays visually aligned with
the prototype across themes.
