# AwareNote

[中文 README](README.md)

AwareNote is a local-first library browser built to manage and preview `PDFs + native image folders`.

It is not a public web service and not a general-purpose media manager. It is a personal tool that respects your existing directory structure, builds indexes and caches around it, and never modifies the source files.

## Screenshots

### Home / Library

![Home / Library](sreenshot/index.png)

### Book Detail

![Book Detail](sreenshot/detail.png)

### Image Preview

![Image Preview](sreenshot/image.png)

### Page Viewer

![Page Viewer](sreenshot/swiper.png)

## Why this project exists

This project came from a very specific frustration: existing tools did not fit the way I actually organize my files.

- Many image managers flatten everything and destroy meaningful folder structure.
- Many comic readers focus on `zip/cbz`-style packages and do not handle native image folders well.
- Some tools support images, but have weak `PDF` support.
- Some comic tools can read pages, but do not provide a good cover-grid browsing experience for mixed libraries.
- Too many apps assume they should reorganize your files instead of respecting your layout.

The actual need was simple:

- folders are already part of the organization
- image packs should stay as normal folders
- `PDF` and image folders should be managed together
- the software should not rename, move, or rewrite source files

AwareNote was built around that boundary.

## What problems it solves

- Manages both `PDF` books and native image-folder books in one place
- Preserves the original directory structure instead of forcing import or flattening
- Improves browsing with cover cache and page cache
- Supports category-style browsing instead of a single flat list
- Can be accessed from the local machine or LAN devices like iPad and phone
- Never modifies the original files, only generates sidecar cache data

## Project principles

- Solve real personal pain points, not theoretical completeness
- Do not add formats just to look more universal
- Never touch source files; all optimization goes through cache
- Prefer clear and stable behavior over over-engineered abstraction

That is why the project is intentionally focused on:

- `PDF`
- native image folders

And intentionally does not focus on:

- `zip`
- `rar`
- `7z`
- automatic source-file organization or renaming

## Current capabilities

- Scan multiple library paths
- Detect categories and subcategories
- Support image-folder books and PDF books
- Generate cover cache
- Generate preview cache for oversized image-folder pages
- Generate SVG cache for PDF pages
- Reveal source files in Finder from the book detail page
- Provide a native macOS menu bar app for launching the service and opening settings
- Use the web UI for browsing and previewing

These capabilities roughly map to:

- library home with category switching and cover grid
- detail page with metadata, path, and cache status
- reader pages for single-page preview and paging
- native settings window for local service and cache configuration

## What it does not try to do

AwareNote is intentionally opinionated about what it will not do:

- it does not modify source files
- it does not take over your directory structure
- it does not auto-organize your library
- it does not force an import-into-library workflow
- it does not prioritize archive formats you do not actually use
- it is not trying to become a public-facing server product

## Best fit

AwareNote fits best if your library looks like this:

- lots of PDFs
- lots of image packs already organized as folders
- you want to preserve the original Finder structure
- you want cover-grid browsing
- you occasionally want to browse from iPad or phone over local network

## How to run

### Option 1: run the Rust backend directly

You need a working Rust toolchain.

```bash
cargo run --bin awarenotes
```

The service starts from the configuration file and the web UI is opened in a browser.

### Option 2: build the macOS app

The project includes a macOS packaging script:

```bash
./scripts/build-macos-app.sh
```

The build output is:

```text
native-macos/dist/AwareNote.app
```

This app is a macOS menu bar app:

- it starts the local service
- it provides a native settings entry
- the actual library browsing and reading still happens in the web UI

## Basic usage

1. Add your library folders in the configuration.
2. Start the service.
3. Open the local web address in a browser.
4. Browse categories, covers, and detail pages.
5. Read through the web preview pages.

If you use the packaged macOS app, you can also access settings and service controls from the menu bar icon.

## Configuration

The project uses `app_config.toml`.

Main configuration areas include:

- server host and port
- log level
- scan paths
- image-count threshold
- cache size and rendering strategy
- internal concurrency settings

This is mainly a personal local tool, so configuration is more direct and less abstracted than a typical consumer app.

## Tech stack

- Backend: Rust + Axum
- Database: SQLite
- ORM: SeaORM
- PDF rendering: MuPDF
- Native macOS menu bar: `tao` + `tray-icon` + SwiftUI settings window
- Frontend: currently a legacy static web UI

## Current status

The project has already reached its main goal, so it is likely to stay in low-maintenance mode rather than continue expanding.

The reason is simple:

- it already solves the core pain point
- adding more features has lower payoff now
- for a personal tool, maintenance cost matters more than feature completeness

So AwareNote should be understood as a practical, bounded local tool, not a large general-purpose library platform.
