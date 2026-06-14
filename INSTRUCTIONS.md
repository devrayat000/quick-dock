# Project Specification: QuickDock (Contextual Drag-and-Drop Shelf)

## 1. Executive Summary & Tech Stack

You are building **QuickDock**, a high-performance, lightweight, "always-on-top" temporary staging area utility app for Windows 11. It allows developers and power users to temporarily park files, text clips, and images when moving them between deep folders, browsers, and IDEs.

* **Framework:** Tauri (v2 preferred for modular architecture)
* **Backend:** Rust (Handles low-level OS listeners, mouse positions, native file drops, and Windows API calls)
* **Frontend:** React + TypeScript + Tailwind CSS (Handles layout animations, image thumbnails, syntax highlighting, and state management)
* **Core Goals:** Near-zero idle resource utilization (less than 30MB RAM target), buttery-smooth UX, native Windows 11 Fluent design alignment.

---

## 2. Windows-Specific Architecture & Crucial Constraints

*Please pay extreme attention to these constraints, as standard web APIs or naive Tauri setups will fail on Windows:*

### Constraint A: The Drag-and-Drop Conflict

On Windows WebView2, when Tauri's native OS file drop listener is enabled (`"dragDropEnabled": true` in configuration), it completely hijacks the window system. This blocks the webview from detecting standard HTML5 `dragover` and `drop` events for internal web elements (like moving things inside the frontend UI).

* **Solution Strategy:** Keep Tauri's file drop listener active to capture file paths dropping in from external apps (File Explorer, Chrome). Manage all internal shelf UI layouts and item re-ordering using raw React Mouse Events (`onMouseDown`, `onMouseMove`, `onMouseUp`) instead of standard HTML5 drag-and-drop libraries to prevent cross-talk or cursor block (`🚫`) bugs.

### Constraint B: Native "Drag-Out" Action

Dragging an asset *out* of a webview and forcing the Windows OS to recognize it as a real physical file (to drop into Slack, Discord, or VS Code) is impossible via JavaScript.

* **Solution Strategy:** Use Tauri Inter-Process Communication (IPC). When the frontend detects a drag intent on a file chip, it must invoke a custom Rust command. The Rust backend must explicitly trigger native Windows user interface bindings (`DoDragDrop` API patterns) passing the cached absolute path of the local file.

### Constraint C: Window Vibrancy (Mica / Acrylic)

Do not attempt to emulate Windows 11 backdrop effects via pure CSS filters.

* **Solution Strategy:** Use the official `window-vibrancy` Rust crate (`apply_mica` or `apply_acrylic`). Configure the Tauri window to be explicitly transparent (`"transparent": true` in `tauri.conf.json`) and strip HTML/body backgrounds to `transparent` in CSS so the native operating system blur shines through.

---

## 3. Core Features & Implementation Guide

### Feature 1: Screen-Edge Detection & Animated Shelf

* **Behavior:** The app runs silently in the background as a system tray utility. When a user picks up a file and moves their cursor near the right/left boundary of the primary monitor screen, a sleek vertical panel slides out.
* **Rust Logic:** Utilize global hooks (via system level window event handlers or crates like `rdev`) to track cursor positioning only during active drag states, or keep an ultra-narrow (e.g., 2px wide), transparent click-through window at the screen edge that registers native OS `DragEnter` signals.
* **Frontend Logic:** Use Tailwind transitions to animate the shelf sliding cleanly on/off screen along the X-axis (`transform translate-x`).

### Feature 2: Smart Assets Intake & Rendering

The shelf must intercept and cleanly format three distinct item types:

1. **Physical Files (Images/Assets/Archives):** Capture the absolute path. If the item is an image (`.png`, `.jpg`, `.webp`), the app should create an optimized thumbnail preview.
2. **Raw Text / Code Snippets:** If code or text is highlighted and dragged directly out of a browser tab, parse the data payload.
3. **URLs:** Retain links cleanly for quick browser drops.

### Feature 3: Smart Developer Tooling (The Code Highlighter)

* **Integration:** Bundle a lightweight editor engine or highlighting compiler (e.g., Prism.js or Monaco Core) into the React frontend.
* **Behavior:** When text is dropped into the shelf, auto-detect common programming structures (JavaScript/TypeScript, Rust, Python, Go, HTML/CSS). Render the item container as a polished, read-only code card reflecting syntactic tokens.

### Feature 4: Memory Management & Auto-Eviction

To prevent cluttering, the tool needs a strict cleanup pipeline:

* Clear specific blocks upon registering successful system paste completions (`Ctrl+V` tracking hooks) or via an adjustable expiration timer loop configured in milliseconds (e.g., auto-purge after 15 minutes).

---

## 4. Suggested Execution Phases (Prompt Sequence)

*Instruct the AI to work through the implementation sequentially using this roadmap:*

```
Phase 1: Environment Setup & Transparent Fluent Window
├── Configure tauri.conf.json with zero-borders, transparency, and always-on-top rules.
└── Set up cargo dependencies for `window-vibrancy` to apply active Mica styling.

Phase 2: The Native Inbound Drag Pipeline
├── Map Tauri's file drop listener pipeline to bridge file paths to React state.
└── Build the React layout utilizing custom mouse-event tracking for safe UI dragging.

Phase 3: The InterProcess "Drag-Out" Architecture
├── Author the Rust backend module capable of instigating Windows native `DoDragDrop`.
└── Connect frontend file thumbnail drag-triggers to invoke the Rust IPC method.

Phase 4: Developer Polish
├── Inject the automatic programming language detection and syntax highlighting engine.
└── Code the background eviction clock and system clipboard synchronization protocols.

```
