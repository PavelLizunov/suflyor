---
name: manual-acceptance-html
description: Create or update a self-contained interactive HTML checklist for manual acceptance, owner review, release smoke testing, regression retesting, visual asset approval, or UI review from a goal, specification, changelog, or existing checklist. Use when Codex must turn implementation scope into a tester-facing HTML file that saves progress locally and produces a copyable plain-text result.
---

# Manual Acceptance HTML

Create one portable HTML file that a non-technical tester can open locally and complete without a server or build step.

## Workflow

1. Read the referenced goal/specification and the repository instructions.
2. Inspect the implemented diff, tests, release notes, assets, and existing acceptance HTML before writing claims.
3. Separate automated evidence from checks that require a human eye, ear, device, account, or real workflow.
4. Derive the shortest complete checklist. Group it into:
   - release smoke and blockers;
   - changed functionality;
   - visual/audio/asset decisions when relevant;
   - detailed scenarios embedded in the same page;
   - owner verdict and notes;
   - result export.
5. Create or update the HTML in the repository's documentation location. Use `acceptance-...html` for first acceptance and `retest-...html` only for regression retesting.
6. Open the file locally and exercise every interaction before handing it off.

## Integrity rules

- Never pre-check a manual item. State automated evidence beside it instead.
- Make every blocking item observable and binary. Include exact navigation, input, and expected result.
- Preserve the goal's acceptance criteria; do not replace them with implementation summaries.
- Mark unavailable or environment-dependent checks explicitly instead of claiming success.
- Require an explicit owner verdict such as `Release`, `Release with notes`, or `Stop`.
- Do not include secrets, live tokens, private URLs, machine-specific absolute paths, or diagnostic contents.

## HTML contract

Use plain HTML, CSS, and JavaScript in one file. Add no framework, package, CDN, font, or server dependency.

Include:

- project/version title, tester field, and date field;
- labelled checkboxes and comment fields;
- visible completed/total progress and blocker state;
- `localStorage` persistence with a file-specific versioned key;
- top and bottom `Copy report` buttons;
- a readonly report preview that can always be selected manually;
- Clipboard API copying with a hidden-textarea/`execCommand('copy')` fallback and visible success/failure status;
- UTF-8 TXT download using `Blob` and `URL.createObjectURL`;
- print/PDF action and print CSS;
- reset action guarded by confirmation;
- responsive layout, keyboard focus visibility, labels, alt text, and sufficient contrast.

Generate a concise plain-text report containing metadata, `[OK]`/`[ ]` checklist lines, asset verdicts with notes, detailed failures, and final owner verdict. Update the preview whenever a field changes.

Embed detailed scenarios with `<details>` in the same page. Avoid a collection of fragile companion links. If a relative link or image is unavoidable, verify that the target exists from the HTML file's directory and works through a local `file:///` open. Never use absolute local paths in `href` or `src`.

For visual assets, show the actual asset or screenshot with a unique label, an `Accept`/`Revise`/`Reject` selector, and a comment box. Do not treat Codex's visual opinion as owner approval.

## Validation

Open the generated file locally and verify:

1. no console errors or missing images;
2. all details, links, inputs, and verdict controls work;
3. progress and blocker state update correctly;
4. reload restores saved state;
5. copy produces the visible report, including the fallback path when clipboard permission is denied;
6. TXT download is readable UTF-8;
7. reset requires confirmation and clears only this checklist;
8. narrow-window and print layouts remain usable.

Report the HTML path and any checks that still require the owner. Do not claim release approval from generated or automated evidence alone.
