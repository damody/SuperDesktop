## ADDED Requirements

### Requirement: Empty desktop space must start a live selection marquee
The desktop SHALL begin a transient selection gesture only from a primary-button press on empty desktop space and SHALL paint a normalized Windows-style rectangle after the movement threshold is crossed.

#### Scenario: Drag down and right
- **WHEN** the user presses empty space and drags the primary button down and right
- **THEN** a translucent blue rectangle follows the pointer and intersecting item hitboxes become selected

#### Scenario: Drag up and left
- **WHEN** the current pointer is above or left of the anchor
- **THEN** the same normalized rectangle and hit-test policy applies without negative size

#### Scenario: Empty click
- **WHEN** the pointer is released before crossing the threshold
- **THEN** no marquee remains visible and an ordinary click clears selection

### Requirement: Marquee selection must be deterministic and modifier-aware
Each move SHALL recompute hits from item bounds and the pointer-down baseline; ordinary marquee SHALL replace selection and Ctrl marquee SHALL union hits with the baseline.

#### Scenario: Drag reverses over an item
- **WHEN** an item enters and later leaves the current rectangle
- **THEN** it is removed again unless it was in the Ctrl baseline

#### Scenario: Ctrl-additive selection
- **WHEN** Ctrl is held at pointer-down with existing selected items
- **THEN** existing items remain selected and current rectangle hits are added

#### Scenario: Gesture completes
- **WHEN** the primary button is released or observed no longer pressed
- **THEN** transient capture is cleared while the final accessible selected state remains

### Requirement: Item interactions must not start a background marquee
Pointer input admitted by a desktop item SHALL retain item click, double-click, drag, drop, rename, and context-menu behavior without activating the empty-space gesture.

#### Scenario: User presses an item
- **WHEN** primary-button down occurs inside an item hitbox
- **THEN** the item handler stops background propagation and no marquee state is created
