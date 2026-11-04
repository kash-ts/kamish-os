---
name: emil-kowalski-design
description: >-
  Apply Emil Kowalski's Pragmatic Design (Design for Developers) principles to build, review,
  and refactor UI/UX layouts. Focuses on reducing visual noise (borders), establishing clear
  typographical hierarchies, ensuring consistent spacing scales, utilizing depth/layering,
  and designing interactive states with high polish.
---

# Emil Kowalski's Pragmatic Design Skill

This skill contains UI/UX design principles inspired by Emil Kowalski's Pragmatic Design course for developers. Apply these principles when building new components, auditing existing screens, or refining visual styling.

## Core Principles

### 1. Reduce Borders & Visual Noise
Borders are often the first thing developers reach for to separate elements. Too many borders create visual clutter.
- **Rule of Thumb**: Prefer using background colors, spacing, or subtle shadows instead of borders to group or separate content.
- **Borders**: If borders are necessary, make them extremely subtle (e.g., 1px with low contrast/translucency, like `rgba(0,0,0,0.06)` or `rgba(255,255,255,0.08)`).
- **Groupings**: Use a slightly different background color (e.g., a card with a white background on a light-gray page background) to establish structure.

### 2. Typographical Hierarchy
Do not rely solely on font size to indicate hierarchy. Using size alone leads to giant, unbalanced text.
- **Font Weight**: Use weight (`font-weight: 500` / `600`) for headers, and standard weight (`400`) for body text.
- **Color Contrast**: Use text color to show importance.
  - Primary text: High contrast (e.g., slate-900 or white).
  - Secondary/supporting text: Lower contrast (e.g., slate-500 or gray-400).
- **Line Height**: Adjust `line-height` depending on size. Headings need tighter line-heights (`1.1` - `1.2`), while body text needs more breathing room (`1.5` - `1.6`).

### 3. Strict Spacing Scale
Layouts feel amateur when margins and paddings are random.
- **Grid System**: Use a consistent spacing scale (e.g., 4px, 8px, 12px, 16px, 24px, 32px, 48px, 64px). In CSS, map these to variables (like `--space-1`, `--space-2`, etc.).
- **Proximity Principle**: Elements that are related should be closer to each other than elements that are not.
  - E.g., a label and its input should have less space (4px - 8px) than the gap between two form fields (16px - 24px).

### 4. Utilize Depth & Elevation (Layering)
Use box shadows and background shade progression to indicate overlays and hierarchies.
- **Elevated Elements**: Dropdowns, modals, and tooltips should have noticeable shadows (`box-shadow`) to lift them off the page.
- **Dark Mode Elevation**: In dark mode, shadows are less visible. Instead, use lighter backgrounds for elements that are "closer" to the screen (e.g., page background is darkest, cards are lighter, modals are the lightest).

### 5. Interactive States & Accessibility
Interactive elements (buttons, inputs, links) must feel tactile and responsive.
- **Hover/Active**: Always style `:hover` and `:active` states.
- **Focus Rings**: Never remove `:focus` or `:focus-visible` outlines without replacing them with highly visible custom focus states.
- **Click Targets**: Ensure buttons and links have a minimum hit target size of `44x44px` (touch-friendly).

## Workflow

### 1. Audit
When building or reviewing a UI, check:
- Is there too much visual clutter? (Can we remove some borders or icons?)
- Are elements correctly aligned? (Do text baselines align? Are icons centered?)
- Is the typographic hierarchy clear?
- Is spacing consistent?

### 2. Remediate
Apply the following CSS / styling adjustments:
- Replace border separators with small background color offsets.
- Reduce font size of sub-headers but increase weight and lower contrast.
- Tighten margins to group related information.
- Increase padding inside cards/components to let them "breathe".
