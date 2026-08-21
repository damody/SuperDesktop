## ADDED Requirements

### Requirement: Version-correct exact-icon callback delivery
The notification-area host SHALL validate the exact live icon generation, owner HWND/process identity, GUID or numeric ID, callback message, and negotiated version immediately before delivering left, keyboard, or context interaction.

#### Scenario: Modern left activation
- **WHEN** a current version-4 icon is invoked by pointer
- **THEN** the host delivers the documented `NIN_SELECT` callback payload to that exact owner once

#### Scenario: Keyboard activation
- **WHEN** a focused icon is invoked with Enter or Space
- **THEN** the host delivers the documented keyboard-select payload with interaction parity

#### Scenario: Right-click context request
- **WHEN** a current icon is right-clicked
- **THEN** the host establishes foreground context and sends the version-appropriate context callback with the interaction coordinates

#### Scenario: Stale or reused owner
- **WHEN** icon generation is stale or the owner HWND/process identity no longer matches
- **THEN** no callback is delivered to the reused target, the stale registration is reconciled away, and SuperDesktop remains alive

### Requirement: Interactive visible and overflow icon surfaces
Visible and overflow notification icons SHALL expose equivalent pointer, keyboard, and accessibility actions without opening a task preview or losing their popup solely because the callback is dispatched.

#### Scenario: Overflow interaction
- **WHEN** an overflow icon is invoked or context-clicked
- **THEN** the same exact-icon command path executes and the resulting application UI or menu can appear above SuperDesktop

