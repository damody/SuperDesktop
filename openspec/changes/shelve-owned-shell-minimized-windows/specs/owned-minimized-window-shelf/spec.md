## ADDED Requirements

### Requirement: Minimized application has no visible desktop tile

While SuperDesktop owns the Windows shell, it SHALL retain each eligible application's true minimized state and taskbar representation while making its legacy iconic representation invisible on the desktop.

#### Scenario: Taskbar command minimizes into the shelf
- **WHEN** the user minimizes an eligible foreground application through its SuperDesktop taskbar button or context command
- **THEN** the window remains iconic and present in the owned taskbar but its minimized title tile is hidden

#### Scenario: Application minimizes itself
- **WHEN** an eligible application invokes its own minimize command while Explorer is absent
- **THEN** SuperDesktop shelves its minimized representation by the next bounded task refresh without removing its taskbar entry

#### Scenario: Multiple minimized applications
- **WHEN** several eligible applications are minimized concurrently
- **THEN** every identity is independently shelved and no legacy iconic tiles are arranged along the desktop edge

#### Scenario: Preview mode retains host ownership
- **WHEN** SuperDesktop runs in preview mode with Explorer owning the shell
- **THEN** SuperDesktop performs no minimized-window hide or cached task-model mutation

### Requirement: Shelving preserves Windows restoration semantics

SuperDesktop SHALL asynchronously hide only an already-iconic current window representation and retain its exact identity in a bounded taskbar cache; it MUST preserve `WINDOWPLACEMENT.rcNormalPosition`, maximum position, size, styles, ownership, process identity, minimized state, and the application's ability to restore and show itself.

#### Scenario: Taskbar activation restores exact bounds
- **WHEN** the user activates a shelved taskbar item
- **THEN** Windows restores and foregrounds the application at its pre-minimize normal rectangle within DPI rounding

#### Scenario: Application restores itself
- **WHEN** a shelved application invokes its own restore operation
- **THEN** it returns to its preserved normal or maximized state without a SuperDesktop-specific inverse move

#### Scenario: Alt Tab restores a shelved window
- **WHEN** the user selects a shelved application through the owned Alt+Tab surface
- **THEN** the same existing restore-and-activate action restores the preserved window placement

### Requirement: Shelf mutation is identity-safe and bounded

Before hiding a minimized representation, SuperDesktop SHALL revalidate the live HWND, PID, stable identity, visibility, iconic state, and task eligibility, and SHALL use `ShowWindowAsync` without any placement or style mutation.

#### Scenario: Retired or reused HWND
- **WHEN** a snapshot identity is destroyed or its numeric HWND now belongs to a different PID or stable identity
- **THEN** SuperDesktop rejects the mutation before calling `ShowWindowAsync`

#### Scenario: Ineligible top-level window
- **WHEN** a window is hidden, restored, a tool window, cloaked, or an owned transient
- **THEN** SuperDesktop does not change its placement and does not track it as shelved

#### Scenario: Repeated snapshots are idempotent
- **WHEN** a successfully shelved identity appears in consecutive minimized snapshots
- **THEN** SuperDesktop does not repeatedly rewrite its placement

#### Scenario: State transition permits retry
- **WHEN** a failed or successful shelf identity restores, retires, hides, or changes identity and is later minimized again
- **THEN** the stale cache entry is removed and the new minimize episode is independently attempted

### Requirement: Shelf failures are observable without destabilizing the shell

SuperDesktop SHALL report a contextual console error once per continuous failing minimize episode and SHALL continue refreshing taskbar state without panic or visibility/style fallback.

#### Scenario: Shelf observation rejects the request
- **WHEN** live identity or visibility observation fails for an otherwise admitted identity
- **THEN** SuperDesktop writes the identity-scoped failure to the console once and remains running

#### Scenario: Repeated failure snapshot
- **WHEN** the same failing minimized identity remains unchanged across refreshes
- **THEN** SuperDesktop suppresses duplicate console messages until the identity leaves that minimize episode

### Requirement: Physical verification proves hide and restore

The Windows GUI gate SHALL exercise real taskbar-owned and application-owned minimization on the exact release candidate and SHALL restore the prior Explorer and Winlogon Shell state after success or failure.

#### Scenario: Final candidate passes twice
- **WHEN** the focused headful case runs twice from clean launch against the final candidate
- **THEN** both reports prove iconic hidden state, retained taskbar entry, exact restore bounds, process survival, and no runtime error signature

#### Scenario: Failure cleanup restores the host
- **WHEN** any launch, minimize, geometry, restore, assertion, or timeout step fails
- **THEN** `finally` cleanup restores the original Winlogon Shell value and Explorer availability and records the recovery result
