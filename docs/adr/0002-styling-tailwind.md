# ADR 0002 — Styling: Tailwind v4 with the loaf.moe brand, replacing PicoCSS

- Status: Accepted
- Date: 2026-06-19

## Context

The first cut linked PicoCSS from a CDN and used classless semantic HTML. That
gave a generic look with no relationship to the loaf.moe brand (deep navy ink,
one warm-red accent, toasty cream, Lora/Outfit/IBM Plex Mono, rounded-everything
with soft navy-tinted shadows). We want the inviter dashboard and the public
invitee page to *be* loaf.moe, and a CDN dependency is undesirable for a
self-hosted account tool.

## Decision

Adopt **Tailwind CSS v4**, configured with the loaf.moe brand tokens.

- `tailwind.css` at the project root is the input: it imports the webfonts and
  `tailwindcss`, declares the brand palette/type/shadows in an `@theme` block
  (generating `bg-ink`, `text-accent`, `font-display`, `shadow-accent`, …), and
  defines reusable component classes (`.btn`, `.field-input`, `.card`, `.badge`,
  `.eyebrow`, `.alert-error`) via `@layer components`.
- Dioxus 0.7's `dx serve` / `dx bundle` auto-detect a root `tailwind.css` and run
  the Tailwind watcher themselves, compiling to `assets/tailwind.css`, which
  `main.rs` links via `asset!`.
- For standalone `cargo` builds (the `asset!` macro needs the file at compile
  time), a `mise run tailwind` task compiles the same output; `lint` and `test`
  depend on it.

## Tooling notes

- Tailwind is pinned through mise as `github:tailwindlabs/tailwindcss[exe=tailwindcss]`,
  **not** the `aqua:` backend: aqua serves the musl build, which is dynamically
  linked against a musl loader absent on glibc systems and fails to exec. The
  github backend selects the glibc binary.
- `assets/tailwind.css` is a build artifact and is gitignored.

## Alternatives considered

- **Keep PicoCSS / hand-written CSS file** — rejected. Pico fights bespoke
  branding, and a hand-rolled stylesheet duplicates what the design system
  already expresses as tokens.
- **Inline `<style>` / CSS modules** — rejected. No utility ergonomics, and the
  brand tokens would be re-typed per component.

## Consequences

- Components carry Tailwind utility classes in their `rsx!`; shared visual
  patterns live as `@layer components` classes so markup stays readable.
- The brand is centralized in one `@theme` block — palette/type changes happen
  in `tailwind.css`, not scattered across components.
- A build step now sits between source and styles; mise/dx own it, so no manual
  watcher juggling.
