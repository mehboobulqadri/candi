# Candi GUI — Design Specification

Visual reference mockup: [`docs/design/mockups/reference-menu-open.png`](mockups/reference-menu-open.png).

## 1. Purpose

This document defines the intended visual and interaction design for the Candi GUI.

Candi is a minimal PDF/document reader. The GUI should feel:

- clean
- modern
- quiet
- focused on reading
- fast
- predictable
- configurable
- visually polished without becoming a dashboard

> **The document is the product. The UI should support reading rather than compete with it.**

---

## 2. Core Design Principles

### Minimalism

Expose only controls useful during reading. Avoid large toolbars, excessive buttons, permanent panels, decorative UI, and scattered settings.

### Reading first

The PDF/document occupies the largest visual area. The user should immediately understand the document, current location, navigation, and search.

### One design language

GUI, TUI, and Android should share the same conceptual language:

- semantic colors
- navigation
- chapters
- bookmarks
- search
- reading position
- themes
- settings

The exact visual implementation can differ by platform.

---

# 3. Overall Layout

```text
┌───────────────────────────────────────────────────────────────┐
│                           TOP BAR                             │
├───────────────┬───────────────────────────────────────────────┤
│               │                                               │
│   NAVIGATION  │              DOCUMENT VIEW                    │
│    SIDEBAR    │                                               │
│               │                                               │
├───────────────┴───────────────────────────────────────────────┤
│                    BOTTOM READER BAR                          │
└───────────────────────────────────────────────────────────────┘
```

There is **no permanent annotation panel**.

The right side should remain dedicated to the document unless a future contextual feature temporarily needs it.

---

# 4. Top Bar

The top bar contains global controls.

## Left

### Menu button

Provides:

- file operations
- recent files
- preferences
- help
- about
- advanced actions

It should not become a permanent settings panel.

### Candi branding

The `Candi` name uses the current **accent color**.

### Optional tagline

A subtle descriptor such as:

`Clean · Minimal · Distraction-Free · Fast`

is branding only and may disappear at smaller window sizes.

## Center

The document title is centered:

`The Art of Computer Programming, Vol. 1`

It should truncate gracefully.

**The document name should not be repeated in the sidebar.**

## Right

Desktop window controls remain at the far right.

---

# 5. Page Navigation

The main navigation control is:

```text
<       17 / 672       >
```

It represents:

- previous page
- current page
- total pages
- next page

Eventually the page indicator should also allow direct page entry.

---

# 6. Search

The search icon opens a compact search interface rather than permanently occupying screen space.

Required behavior:

- enter query
- highlight matches
- next result
- previous result
- result count
- jump to result

A persistent search-results sidebar can be available when useful.

---

# 7. Document/Contents Toggle

A document/contents icon can toggle the navigation sidebar.

When hidden, the document gets the additional width.

When shown, the sidebar provides navigation.

---

# 8. Fullscreen / Focus Mode

Fullscreen/focus mode removes unnecessary chrome.

Normal:

```text
top bar
sidebar
document
bottom bar
```

Focus:

```text
document
```

Controls can temporarily appear when needed.

Keyboard shortcuts must continue to work.

---

# 9. Zoom

The toolbar uses:

```text
−   100%   +
```

Required:

- zoom out
- zoom in
- current zoom
- reset zoom

Future:

- fit width
- fit page
- reading-width mode

---

# 10. Overflow Menu

The `⋮` menu contains secondary actions such as:

- Open
- Open Recent
- Export
- Print
- Document Information
- Settings
- Keyboard Shortcuts
- About

Only relevant actions should appear.

---

# 11. Left Navigation Sidebar

The sidebar is for **navigation**, not document metadata.

There should be **no document-name block at the top**.

The navigation icons start near the top.

Primary navigation:

```text
Contents
Bookmarks
Chapters
Search
```

Settings should preferably remain in the application menu.

---

# 12. Contents

Shows the PDF's existing table of contents/navigation structure.

Example:

```text
CONTENTS

Preface                              v

1. Basic Concepts                    1
    1.1 Introduction                 1
    1.2 Algorithms                   5
    1.3 Data Structures             17

2. Information Structures           31
3. Sorting and Searching             97
4. Combinatorial Algorithms         153
5. Graph Algorithms                 215
```

The active section uses the accent color.

---

# 13. Bookmarks

Bookmarks are user-created reading markers.

Example:

```text
BOOKMARKS

Important theorem              p. 47
Read this later                p. 83
Exam material                  p. 129
```

Bookmarks belong to Candi's per-document state.

---

# 14. Custom Chapters

A user can select a page and define it as the beginning of a custom chapter.

Example:

```text
CHAPTERS

1. Introduction                 p. 1
2. Algorithms                   p. 17
3. Data Structures              p. 42
4. Advanced Topics              p. 108
```

This is useful when a PDF has poor or missing navigation metadata.

---

# 15. Search Results

Persistent search results can look like:

```text
SEARCH

algorithm

5 results

p. 5     algorithm is a finite...
p. 17    properties of an algorithm...
p. 21    analysis of algorithms...
p. 97    sorting algorithm...
```

Search should normally prefer an overlay/compact UI to preserve reading space.

---

# 16. Sidebar Behavior

The sidebar should:

- be collapsible
- remember its open/closed state
- resize gracefully
- avoid excessive width
- use subtle separators
- use accent color only for active states

It should never overpower the document.

---

# 17. Document View

The document is the central focus.

The application background and document surface should be visually distinct.

Conceptually:

```text
dark application background
        ↓
document/page surface
        ↓
PDF content
```

---

# 18. Dark Mode

Dark mode has two separate concepts.

### Dark UI

Darkens:

- toolbar
- sidebar
- controls
- surrounding background

### Dark document rendering

Transforms the PDF page itself.

These must be independent.

Possible combinations:

```text
Dark UI + normal PDF
Dark UI + dark PDF
Light UI + normal PDF
Warm UI + warm PDF
```

---

# 19. PDF Dark Transformation

Future dark document mode should intelligently transform colors.

Desired:

```text
white page
    ↓
dark page

black text
    ↓
light text

colored diagrams
    ↓
readable transformed colors
```

Simply placing a black overlay over the page is not sufficient.

Images and diagrams require special treatment.

This is deliberately outside the first text-only milestone.

---

# 20. Bottom Reader Bar

The bottom bar contains reading-specific controls.

Left:

- theme

Center:

- zoom

Right:

- page/view layout controls

---

# 21. Theme Control

The current design uses:

```text
☀  Light   ▼
```

The icon must use the current **accent color**.

Available modes should eventually include:

```text
Light
Dark
Warm
Custom
```

Potentially later:

```text
Dark + Warm
```

---

# 22. Theme Editing Philosophy

Candi should **not** create a huge GUI full of color pickers.

Instead:

```text
Theme menu
    ↓
Edit/Open theme configuration
    ↓
YAML/TOML configuration
    ↓
Save
    ↓
Candi reloads the theme
```

The GUI provides the entry point; the configuration file provides deep customization.

This keeps the interface minimal.

---

# 23. Semantic Theme System

Theme values should describe semantic roles, not widgets.

Example:

```toml
[colors]
background = "#0B0D10"
foreground = "#E6E6E6"
muted = "#8B8F98"
accent = "#9B7BFF"

[document]
background = "#12141A"
foreground = "#E8E6E3"

[selection]
background = "#2A2440"
foreground = "#FFFFFF"
```

The exact schema is a separate implementation decision.

---

# 24. Accent Color

The accent color is a central visual token.

It can control:

- Candi branding
- selected sidebar item
- active controls
- slider handles
- theme icon
- focus indicators
- active chapter
- selected page
- links where appropriate

It should not be sprayed across the entire interface.

It is a visual signal, not a background color for everything.

---

# 25. Warm Mode

Warm mode reduces the harshness of the display.

Conceptually:

```text
white → warm cream
```

and:

```text
dark → warm dark
```

Warm mode should be independent of light/dark UI mode.

---

# 26. Typography

Typography should support long reading sessions.

UI font:

- clean
- modern
- highly legible

For graphical PDF rendering, preserve the PDF's own typography.

For text-first mode, future reader settings can include:

- font
- font size
- line height
- letter spacing
- maximum reading width

---

# 27. Reading Width

The document should not stretch across the entire monitor.

A maximum comfortable reading width should be considered.

This is particularly important for text-first rendering.

---

# 28. Borders and Shadows

Borders should be subtle.

Avoid outlines around every component.

Use borders mainly for:

- panel boundaries
- selected states
- input fields
- document boundaries

Shadows should be restrained.

---

# 29. Icons

Icons should be:

- simple
- consistent
- monochrome by default
- accent-colored when active

Avoid mixing icon styles.

All icons should have tooltips/accessibility labels.

---

# 30. Responsive Desktop Layout

### Large window

```text
sidebar + document + controls
```

### Medium window

```text
collapsed sidebar + document
```

### Small window

```text
document-first layout
secondary controls move into menus
```

The document always receives the highest priority for space.

---

# 31. Fullscreen Reading

Fullscreen should remove distractions:

```text
┌───────────────────────────────────────┐
│                                       │
│              DOCUMENT                 │
│                                       │
│                                       │
└───────────────────────────────────────┘
```

Controls can appear temporarily.

---

# 32. Accessibility

The design must not depend on color alone.

For example:

- selection should have shape/background differences
- errors should have icons or labels
- focus should be clearly visible

Future requirements:

- font scaling
- keyboard navigation
- screen-reader support
- touch-friendly targets
- reduced motion

---

# 33. Keyboard Interaction

The GUI should remain keyboard-friendly.

Possible shortcuts:

```text
Ctrl+O       Open
Ctrl+F       Search
Ctrl+G       Go to page
Ctrl+B       Bookmark
Ctrl+Shift+C Chapters
+ / -        Zoom
F11          Fullscreen
Esc          Close overlay
```

Exact bindings are not final.

Every common reading action should have a keyboard path.

---

# 34. Mouse Interaction

Mouse users should be able to:

- click TOC entries
- click pages
- select search results
- open menus
- scroll
- zoom
- resize supported panels

Mouse and keyboard should complement one another.

---

# 35. Drag and Drop

Eventually:

```text
Drag PDF
     ↓
Candi window
     ↓
Open document
```

This is a GUI/platform feature and should not affect the core architecture.

---

# 36. Android Adaptation

The desktop layout should **not** simply be shrunk onto Android.

Desktop:

```text
Sidebar | Document | Controls
```

Android:

```text
Document
   ↓
bottom/navigation controls
```

Potential Android UI:

- bottom navigation
- slide-out contents
- touch gestures
- floating search
- system file picker

The same Candi Core remains underneath.

---

# 37. No Permanent Annotation Panel

Annotations are intentionally absent from the current design.

Reasons:

- less clutter
- more document width
- simpler reading experience
- avoids turning Candi into a document-management dashboard

Annotations may become a future contextual feature.

---

# 38. No Document Name in Sidebar

The document title is already visible at the top.

Repeating it wastes vertical space.

Therefore:

```text
TOP BAR
└── document title

SIDEBAR
└── navigation
```

This is an explicit design decision.

---

# 39. Important UI States

### Normal reading

```text
sidebar + document + controls
```

### Sidebar hidden

```text
expanded document
```

### Search

```text
document + compact search UI
```

### Page jump

```text
page input
```

### Fullscreen

```text
document-first
```

### Theme editing

```text
normal UI
    ↓
theme menu
    ↓
open configuration file
```

---

# 40. Empty State

When no document is open:

```text
                 Candi

          Open a PDF to begin

             [ Open File ]

       or drag and drop a PDF here
```

Avoid promotional cards or unnecessary dashboard content.

---

# 41. Loading State

Opening a large PDF should not make the application appear frozen.

Example:

```text
Opening document…

Extracting text…
```

The UI should remain responsive whenever possible.

---

# 42. Error State

Errors should be understandable.

Bad:

```text
Error: -2147483647
```

Better:

```text
Unable to open this PDF.

The document appears to be corrupted or unsupported.

[Details] [Open another file]
```

Technical details can be available separately.

---

# 43. Performance

PDFs can be large.

Avoid:

```text
load entire PDF
→ extract everything
→ render everything
→ display
```

Prefer:

```text
open document
    ↓
load metadata
    ↓
load current page
    ↓
load nearby content as needed
```

Search indexing can happen asynchronously.

---

# 44. Shared Core Architecture

The GUI must not implement its own PDF parser.

```text
                    Candi Core
                         │
          ┌──────────────┼──────────────┐
          │              │              │
         TUI             GUI          Android
          │              │              │
       terminal       desktop        mobile
```

Core owns:

- document
- pages
- extraction
- navigation
- search
- bookmarks
- chapters
- reading state
- themes

UI owns:

- rendering
- input
- layout
- platform interaction

---

# 45. Design Tokens

Implementation should define semantic tokens:

```text
color.background
color.surface
color.surfaceElevated
color.foreground
color.muted
color.accent
color.border
color.selection
color.error
color.warning
color.success

spacing.xs
spacing.sm
spacing.md
spacing.lg
spacing.xl

radius.sm
radius.md
radius.lg

font.ui
font.document
font.heading
```

This makes theme support and redesigns much easier.

---

# 46. Theme Architecture

Avoid hundreds of hard-coded colors.

Use:

```text
Theme file
     ↓
Theme parser
     ↓
Validated semantic tokens
     ↓
TUI / GUI
```

GUI and TUI should share the same semantic theme concepts.

---

# 47. Theme Editing Workflow

```text
Candi
 │
 ├── normal interface
 │
 └── Theme menu
       │
       ├── Light
       ├── Dark
       ├── Warm
       └── Edit theme
               │
               ▼
          theme.yaml/toml
               │
               ▼
             edit
               │
               ▼
             save
               │
               ▼
        Candi reloads theme
```

The user should not need to rebuild Candi to change colors.

---

# 48. What This Design Does NOT Solve

This design does not yet define:

- exact PDF rendering implementation
- OCR
- annotation engine
- PDF editing
- synchronization
- cloud storage
- document management
- multiple-document tabs
- plugin architecture

These should not complicate the first implementation.

---

# 49. Implementation Priority

## Phase 1

```text
Window
Top bar
Document view
Sidebar
Bottom bar
```

## Phase 2

```text
PDF loading
Page navigation
Zoom
Search
```

## Phase 3

```text
Contents
Bookmarks
Chapters
Reading position
```

## Phase 4

```text
Themes
Dark mode
Warm mode
Theme-file editing
```

## Phase 5

```text
Images
Diagrams
Full graphical PDF rendering
```

## Phase 6

```text
Android
Drag/drop
Touch
Platform-specific adaptation
```

---

# 50. Final Visual Hierarchy

The interface should communicate:

```text
1. DOCUMENT
   ↓
2. PAGE / READING POSITION
   ↓
3. NAVIGATION
   ↓
4. SEARCH / ZOOM
   ↓
5. CONFIGURATION
   ↓
6. EVERYTHING ELSE
```

The visual hierarchy should reflect this priority.

---

# 51. Final Design Statement

Candi's GUI should feel like a **quiet instrument for reading**.

It should not feel like:

- an office suite
- a browser
- a document-management system
- a settings application
- a dashboard

The ideal experience is:

```text
Open Candi
     ↓
Open PDF
     ↓
Immediately read
     ↓
Navigate when necessary
     ↓
Search when necessary
     ↓
Customize when desired
     ↓
Return to reading
```

The interface should disappear into the background while the document remains the focus.

---

# 52. Current Visual Reference Decisions

The current visual reference establishes these baseline choices:

- dark-first desktop UI
- purple/configurable accent
- minimal top bar
- centered document title
- compact page navigation
- left navigation sidebar
- no permanent annotation panel
- no repeated document title in sidebar
- contents/bookmarks/chapters/search navigation
- bottom theme control
- accent-colored theme icon
- bottom zoom controls
- bottom page-layout controls
- near-black application background
- soft-white typography
- restrained borders
- minimal rounded controls
- generous document reading area

These are the baseline design targets for the first Candi GUI prototype.

They can evolve after usability testing, but changes should preserve the core philosophy: **minimal UI, maximum reading focus.**
