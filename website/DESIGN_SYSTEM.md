# Codex Switch Website Design System

This file defines the website UI rules. Treat it as a project constraint, not a suggestion.

## Visual Direction

- Style: quiet, precise, desktop-product focused.
- Avoid: blue-purple gradients, decorative blobs, oversized marketing cards, random illustration styles.
- Prefer: white or near-white surfaces, thin hairline edges, soft shadows, clear hierarchy, product screenshots or product-like panels.

## Tokens

- Background: `#f7f9fc`
- Surface: `rgba(255,255,255,0.86)` or `#ffffff`
- Text: `#0f172a`
- Muted text: `#667085`
- Primary: `#1683ff`
- Primary hover: `#0b63ce`
- Success: `#29bf65`
- Warning: `#f59e0b`
- Danger: `#ff4f55`

## Typography

- Font stack: system UI, Inter-compatible sans-serif, Chinese fallback.
- Letter spacing: `0`; do not use negative letter spacing.
- Hero title: 46-74px, line-height near 1.
- Section title: 32-48px.
- Card title: 20px.
- Body: 15-19px, line-height 1.55-1.72.

## Components

- Large cards: 18-24px radius.
- Buttons: 12px radius, 46px min height.
- Badges: fully rounded pills.
- Edge treatment: use hairline `box-shadow` rings instead of heavy visible borders.
- CTA buttons: primary blue for one main action, secondary white for supporting actions.

## Layout

- Max content width: 1180px.
- Page gutters: 24px desktop, 14px mobile.
- Use grid layouts with stable columns; avoid nested cards unless the inner card is a real repeated item.
- Hero should expose product UI immediately and keep the next section reachable without visual clutter.

## Release Link Rule

All versioned download links in website and README must follow `package.json` version.
Run:

```bash
npm run version:sync
npm run version:check
```

before committing release or website changes.
