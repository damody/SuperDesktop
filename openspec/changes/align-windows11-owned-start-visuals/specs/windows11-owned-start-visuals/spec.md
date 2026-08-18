## ADDED Requirements

### Requirement: Start presentation follows the Windows locale
The owned Start SHALL present Traditional Chinese labels when the Windows user locale or deterministic override is `zh-TW`, and SHALL use bounded English fallback for all other unsupported locales.

#### Scenario: Traditional Chinese Start Home
- **WHEN** Start opens under `zh-TW`
- **THEN** Search, Pinned, Recommended, All apps, Settings, Power and empty-state labels are presented in Traditional Chinese with stable UIA roles

#### Scenario: Unsupported locale fallback
- **WHEN** the locale is neither Traditional Chinese nor English
- **THEN** Start renders the complete English table without missing or malformed labels

### Requirement: Start uses Windows 11 visual hierarchy
The owned Start SHALL retain its bounded 640×720 logical surface while rendering Windows 11 panel, search, section, grid, footer and Power-flyout hierarchy with consistent tokens.

#### Scenario: Home at 175 percent DPI
- **WHEN** Start Home opens on the reference host
- **THEN** the search field, six-column Pinned grid, two-column Recommended area and compact footer remain within the surface without overlap

#### Scenario: All apps and Power
- **WHEN** the user opens All apps or Power
- **THEN** the owned page/flyout preserves the Windows 11 hierarchy, localized labels and current confirmation contract

### Requirement: Start interaction states are accessible
Every owned Start action SHALL expose distinct hover, pressed and keyboard focus states and SHALL keep pointer, keyboard and UIA activation equivalent.

#### Scenario: High contrast keyboard navigation
- **WHEN** high contrast is active and focus moves through Start
- **THEN** focused controls remain visible by border geometry and expose stable names and roles

### Requirement: Start remains Explorer-free
SuperDesktop MUST render and operate Start without invoking Explorer, system Start hosts, SearchHost, ShellExperienceHost or system Settings presentation.

#### Scenario: Owned Start process observation
- **WHEN** Start opens and navigates Home, All apps and Power
- **THEN** the Start window belongs to SuperDesktop and no system Start process transition or delegated surface occurs
