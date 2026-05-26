# ADR-002 — Slint license tier: royalty-free

**Status:** Decided — use the **royalty-free** Slint license for the
overlay-mvp Slint migration. Revisit if the project moves out of
pet-project / personal-use scope.

**Date:** 2026-05-27 (Phase 0 Day 1 of Slint migration)

## Context

Slint (the `slint` crate, SixtyFPS GmbH) is dual-licensed:

| Tier | Cost | Conditions |
|---|---|---|
| **Royalty-free** (default for OSS / personal use) | Free | Must display `AboutSlint` widget OR equivalent attribution somewhere reachable from the app; must include attribution on the download/release page; Slint AG retains marketing rights to mention the app as a user. |
| **Commercial** | Paid (per-developer subscription) | No attribution required, no marketing-mention rights, optional priority support. |

(Full terms: <https://slint.dev/pricing>.)

overlay-mvp is:

- A solo pet project (single contributor: x3d_mutant)
- Shipped to fewer than 100 users (currently personal use only)
- Not generating revenue
- Already MIT-licensed at the source level

## Decision

Adopt the **royalty-free** tier. Concretely:

1. Add a Slint attribution panel reachable from Settings (e.g. an
   "About" section listing third-party software including Slint). The
   simplest implementation is to embed `AboutSlint` from the
   `std-widgets.slint` library in the existing Settings → Updates or
   a new Settings → About panel during Phase 6.
2. Mention Slint in the GitHub release notes' "Built with" section
   starting v0.2.0.
3. No code changes required during Phase 0 pilot — the pilot binary
   stays internal; attribution lands in Phase 6 alongside the other
   final-polish work.

## Trade-offs

### Why not commercial

- The subscription cost (≥ €99/developer/month at time of writing) is
  unjustifiable for a project that does not generate revenue.
- Attribution is one widget call — negligible friction.
- Slint AG marketing-mention rights are acceptable for a pet project;
  no NDA or confidentiality concerns.

### Acceptable cost of royalty-free

- Must ship `AboutSlint` (or equivalent attribution). Negligible binary
  size + a panel the user can ignore.
- Cannot relicense the resulting Slint-built binary under terms more
  restrictive than the royalty-free license. We don't intend to.
- If overlay-mvp ever transitions to a commercial product (paid SaaS,
  paid downloads), this ADR must be reopened and an upgrade purchased
  BEFORE the first commercial release.

## Trigger to revisit

Re-open this ADR if **any** of:

1. overlay-mvp transitions to a commercial / paid offering.
2. A contributor objects to the Slint AG marketing-mention clause.
3. SixtyFPS GmbH changes the royalty-free terms in a way that imposes
   new obligations on existing builds.

In any of those cases, evaluate: (a) buy a commercial subscription,
(b) migrate to Iced (Variant A from [[suflyor-gui-strictness-spec]]),
or (c) stay on React + Tier 1-4 harness.

## Reference

- Slint pricing: <https://slint.dev/pricing>
- Stack decision history: [`ADR-001-stack.md`](ADR-001-stack.md)
- Migration plan: [`MIGRATION-PLAN-SLINT.md`](MIGRATION-PLAN-SLINT.md)
