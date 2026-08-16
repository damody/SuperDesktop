## ADDED Requirements

### Requirement: Context menus are isolated
The system SHALL enumerate and invoke optional shell menu providers outside the GPUI process.

#### Scenario: Provider crashes during enumeration
- **WHEN** the provider host exits before enrichment completes
- **THEN** GPUI remains alive, displays built-in commands, and marks enrichment unavailable

### Requirement: Menu descriptors are sanitized
The system SHALL accept only owned, bounded command trees with stable tokens, labels, state, risk, and optional children.

#### Scenario: Provider returns excessive depth
- **WHEN** a menu tree exceeds the configured nesting limit
- **THEN** the provider response is rejected without rendering any unvalidated node

### Requirement: Invocation is bound to menu context
The system SHALL bind invocation tokens to host generation and selection fingerprint and reject stale or mismatched tokens.

#### Scenario: Selection changes before invocation
- **WHEN** a command token is invoked for a different selection fingerprint
- **THEN** invocation fails closed and no shell command runs

### Requirement: Built-in fallback remains functional
The system SHALL provide capability-filtered built-in commands within 250 milliseconds regardless of optional provider health.

#### Scenario: Native enrichment times out
- **WHEN** optional enumeration exceeds two seconds
- **THEN** the menu keeps its built-in commands and reports enrichment timeout

### Requirement: Keyboard and accessibility parity
The system SHALL expose menu roles, enabled state, focus order, shortcuts, submenus, and invocation through pointer, keyboard, and accessibility actions.

#### Scenario: Keyboard invokes a built-in command
- **WHEN** a focused enabled command receives Enter
- **THEN** it emits the same typed invocation as pointer activation
