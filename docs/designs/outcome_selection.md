# Design: Outcome Selection Pattern (2–N Outcomes)

## Objective
Create a unified, scalable, and mobile-safe outcome selection UI that provides unambiguous feedback for prediction market participants. It must adapt its layout based on the number of outcomes (N).

## Adaptive Layout Patterns

### 1. Binary Selection (N=2)
- **Pattern:** Side-by-side large cards (Full width on mobile, split columns on desktop).
- **Interaction:** Single tap selects the card, highlights it with high-contrast borders and an "Active" indicator.
- **Confirmation:** Vibrant upon selection.

### 2. Multi-Outcome Selection (3 ≤ N ≤ 6)
- **Pattern:** Grid cards (2 columns on mobile, 3–4 columns on desktop).
- **Content:** Includes outcome label, current percentage stake, and odds/multiplier.
- **Safety:** One tap highlights the outcome; second tap triggers confirmation.

### 3. Dense Multi-Outcome (N > 6)
- **Pattern:** Vertical list view (full width row items).
- **Scaling:** Supports long labels via multi-line text alignment.

## Visual Design & States

### State: Idle
- Subtle glassmorphism background.
- Semi-transparent border.

### State: Selected
- High-contrast border (amber/green).
- Pulse animation.
- Background becomes 10% more opaque.

### Mobile Optimization
- **Minimum Tap Target:** 48px height per row/card.
- **Spacing:** 12px gutter between outcomes.
- **Grip/Scroll:** List view implements a "Safe Scroll" gutter.

## Performance & Accessibility
- **Screen Readers:** Uses `role="radiogroup"`.
- **Keyboard:** Full tab navigation support.
- **Loading:** Skeleton states with distinct heights.
