# TODO

## Issue: macOS app silently fails when configured port is already in use

### Background
- `AwareNote.app` is a local macOS menu bar wrapper around the Rust backend and web UI.
- The app is expected to feel like a desktop utility, even though the actual reading experience is still delivered through the browser.
- Because it is a local tool, startup feedback matters more than abstract server purity. If launch fails, the app should explain why.

### Current problem
- If the configured local port, such as `3001`, is already occupied, clicking `AwareNote.app` may appear to do nothing.
- This creates an especially bad experience because from the user's perspective there is no visible error, no running window, and no obvious recovery path.
- The failure may come from two very different situations:
- A previous AwareNote backend is already running.
- Another unrelated process is already using the configured port.

### Why this should be fixed
- Silent failure makes the app feel broken or incomplete.
- The actual issue is usually recoverable, but the current UX does not surface the recovery path.
- For a menu bar app, "no visible reaction" is one of the worst failure modes.

### Goal
- Make startup failure observable and actionable.
- Distinguish between "AwareNote is already running" and "another process is using the port".
- Preserve the current explicit port configuration model instead of introducing hidden behavior.

### Proposed startup flow
1. Launch app and attempt to start backend on the configured host and port.
2. If startup succeeds:
- Continue normal startup.
- Menu bar status remains the normal running state.
3. If bind fails because the port is occupied:
- Probe the configured local web endpoint.
- Perform a lightweight health or identity check against the existing process.
4. If the process already listening on that port is AwareNote itself:
- Treat this as "already running", not as an error state.
- Open the existing web UI, or otherwise foreground the existing running session.
- Avoid showing an error dialog in this branch.
5. If the process listening on that port is not AwareNote:
- Show a native macOS alert immediately.
- State the exact configured port that failed, for example `3001`.
- Explain that another process is already using the configured port.
- Offer clear next actions instead of failing silently.

### Proposed alert content
- Title: `AwareNote 无法启动服务`
- Body: `配置的端口 3001 已被其他进程占用，请修改端口后重试。`
- Buttons:
- `打开设置`
- `退出`

### Optional alert variants
- If desired, a third button could be added later:
- `重试`
- But retry is only useful if configuration has already changed, so it is not necessary for the first version.

### Detection details
- The "is this already AwareNote?" check should be based on an explicit backend identity signal, not a vague HTML match.
- Best options:
- A dedicated health endpoint response containing an app identifier.
- An existing JSON endpoint that reliably proves the server is AwareNote.
- The check should be lightweight and only run after bind failure, not on every successful startup path.

### Menu bar behavior
- Even if an alert is shown, the menu bar state should remain understandable.
- If startup fails, the menu could expose a temporary error state, such as:
- `启动失败：端口 3001 已被占用`
- And actions such as:
- `打开设置`
- `重试`
- `退出`
- This is a useful follow-up, but not required for the first fix.

### Settings integration
- `Open Settings` should take the user directly to the existing native settings UI.
- The relevant control is the configured port field.
- The fix should avoid making users hunt through the UI for the reason startup failed.

### Explicitly not preferred for now
- Do not silently switch to a random fallback port by default.
- Automatic port fallback would make local bookmarks and LAN access less predictable.
- Automatic fallback would also make debugging harder because the app would no longer be using the configured value the user expects.
- Do not hide the failure only in logs.
- For this kind of local desktop wrapper, a visible user-facing explanation is required.

### Edge cases to think through
- The configured port may be occupied by another AwareNote instance started from Terminal rather than Finder.
- The configured endpoint may respond slowly during startup race conditions.
- A stale browser tab may exist even though the backend is not currently available.
- The backend might fail for a reason other than port conflict; those cases should still surface a visible failure path instead of silently doing nothing.

### Implementation notes
- This should be handled in the native macOS launcher path, not just in backend logs.
- The backend error needs to be propagated to the native layer in a structured enough way to distinguish address-in-use from generic startup failure.
- If the backend process is spawned as a child, the launcher needs a short startup observation window so it can detect immediate failure instead of assuming launch succeeded.

### Acceptance criteria
- When port `3001` is occupied by another process, opening `AwareNote.app` produces a visible native error prompt.
- The prompt clearly states that the configured port is in use.
- The prompt allows the user to open settings immediately.
- When port `3001` is already being used by AwareNote itself, opening `AwareNote.app` does not show an error and instead opens or reuses the existing service.
- The app no longer appears to do nothing in either case.

## Issue: whether to surface unsupported archive files

### Background
- The project intentionally focuses on `PDF + native image folders`.
- It does not currently support direct reading of `zip` / `rar` / `7z` and similar archive formats.
- This is a deliberate scope decision rather than a missing bug fix.

### Why this came up
- In real libraries, unsupported archive files may sit beside normal books.
- If scanning ignores them completely, they become invisible in the UI.
- That invisibility can make it feel like scanning missed files.

### Main design tension
- If archives are shown, the model becomes more complex.
- If archives are ignored, the model stays cleaner but loses visibility.
- The complexity increases further when an archive is not at category root, but inside what is otherwise treated as a normal image-folder book.

### Options discussed

### Option A: keep ignoring archives completely
- Pros:
- Keeps the current model clean.
- No extra scan, DB, API, or UI complexity.
- Matches the current project boundary.
- Cons:
- Users cannot tell whether an archive was skipped or never existed.

### Option B: show archives as unsupported placeholder resources
- Pros:
- Makes skipped files visible.
- Better explains scan results.
- Cons:
- Requires a parallel "non-book resource" model.
- Raises questions about where these placeholders should appear:
- category level
- inside a book
- separate unsupported section
- This quickly becomes broader than a small UX tweak.

### Option C: treat archives as a special book type
- Pros:
- Simplest implementation path if visibility is required.
- Can reuse much of the existing book list, category, search, and storage flow.
- Archives could appear in the UI with basic metadata and an action like `Open in Finder`.
- Cons:
- Pollutes the meaning of "book", because these entries are not actually readable.
- Would require special-case behavior in list, detail, and action logic.

### Current conclusion
- Do not implement archive support for now.
- If this is ever revisited, the most practical route is probably Option C:
- treat archives as a special non-readable book type
- allow basic visibility
- do not provide reader/detail parity with normal books
- For the current stage of the project, continuing to ignore unsupported archives is the cleaner choice.

### Explicit non-goals
- Do not add direct archive reading just for completeness.
- Do not introduce auto-extraction or temporary unpacking logic.
- Do not mutate source files or reorganize archive contents.

### Revisit trigger
- Only revisit this if unsupported archives become a frequent real-world nuisance in day-to-day use.
- Do not expand the scope based on theoretical completeness alone.

## Issue: future web frontend rewrite

### Background
- The current frontend is still the old HTML + JavaScript implementation embedded into the Rust binary.
- It is usable, but maintainability is weak and mobile adaptation is not good enough.
- A previous attempt to move toward `Vue + Quasar` was stopped midway because the result still carried too much old-page structure and did not really use Quasar as a component system.

### Why this matters
- The project only has a few core pages:
- home/library page
- book detail page
- settings page
- In theory this is small enough to rewrite cleanly.
- In practice, if the rewrite keeps dragging old DOM structure and imperative page logic forward, it loses most of the benefit.

### Current conclusion
- Do not continue incremental frontend migration inside the current old-page structure.
- If a rewrite is done in the future, it should be treated as a genuinely new frontend project rather than a patchwork port.

### Rewrite goals
- Improve maintainability through componentized structure.
- Improve mobile and tablet usability without needing separate pages.
- Preserve the current backend API-first architecture.
- Keep reading/preview behavior working for both desktop and mobile browsers.

### Framework direction
- `Quasar` remains the preferred direction if the frontend is rewritten.
- The reason is practical rather than fashionable:
- it already provides responsive UI primitives
- it fits both desktop and mobile use
- it reduces the need to hand-roll layout and interaction details

### Important constraint for a future rewrite
- Do not let the old frontend's DOM structure dictate the new implementation.
- The rewrite should be designed from backend capabilities and actual user flows, not copied from existing HTML fragments.
- Avoid large amounts of raw `div`-driven layout when Quasar already provides an appropriate component.

### Expected page structure
- Home:
- libraries
- categories
- book grid
- favorite view
- Book detail:
- metadata
- cover
- path and local-file actions
- links into reader/preview
- Settings:
- configuration editing
- cache actions
- scan actions

### Component expectations
- Prefer Quasar components for page scaffolding, navigation, lists, cards, forms, dialogs, and actions.
- Avoid recreating generic UI pieces manually with plain HTML containers unless there is a clear reason.
- Build from reusable components instead of page-level script blobs.

### API guidance
- The rewrite should treat the Rust backend as the source of truth.
- Frontend data flow should be designed around backend endpoints and domain concepts, not around preserving old ad hoc page scripts.
- Avoid repeated, fragmented fetching when a single full response is already available and appropriate for the local-tool use case.

### Explicit non-goals
- Do not mix half-rewritten Vue pages with large leftover blocks of legacy static HTML.
- Do not optimize for public SaaS patterns; this is still a local-first tool.
- Do not force feature expansion just because a rewrite is happening.

### Revisit trigger
- Revisit only when there is enough time and energy to build a clean replacement in one dedicated effort.
- Do not restart frontend rewriting as a side quest during backend maintenance.
