## ADDED Requirements

### Requirement: Typed immutable test catalog
UTIT SHALL expose stable compiled test cases grouped into `smoke`, `shell-parity`, and `full` suites, and SHALL reject duplicate IDs, invalid suite closure, arbitrary command strings, or paths outside approved roots.

#### Scenario: Catalog listing
- **WHEN** the user runs `superdesktop-utit list`
- **THEN** every case is printed with stable ID, tier, tags, timeout, prerequisite class, and recovery class without executing a case

#### Scenario: Invalid catalog entry
- **WHEN** a case has a duplicate ID, shell command string, escaping path, zero timeout, or Explorer-free tier without watchdog recovery
- **THEN** catalog validation fails before any child process or filesystem evidence mutation

#### Scenario: Suite closure
- **WHEN** `shell-parity` or `full` is selected
- **THEN** all mandatory lower-tier cases are included unless an explicit filter marks the run partial

### Requirement: Bounded safe execution
UTIT SHALL execute admitted cases serially with explicit argv, bounded deadlines, isolated logs, exact-child termination, and declared recovery handling.

#### Scenario: Passing case
- **WHEN** an admitted child exits zero within its deadline and produces every declared artifact
- **THEN** UTIT records passed with exit code, duration, argv, log hashes, and artifact hashes

#### Scenario: Failure or timeout
- **WHEN** a child exits non-zero, fails to spawn, exceeds its deadline, or omits an artifact
- **THEN** UTIT records failed, preserves stdout/stderr, terminates only the exact child when required, and does not relabel it blocked or passed

#### Scenario: Explorer-free recovery
- **WHEN** an Explorer-free case runs
- **THEN** its watchdog contract is verified before launch and its recovery disposition is recorded before the next GUI case begins

### Requirement: Truthful prerequisite and completeness semantics
UTIT SHALL record host facts and derive terminal states without treating unavailable mandatory physical, reboot, installer-mutation, or external-review evidence as passed.

#### Scenario: Missing physical prerequisite
- **WHEN** a mandatory full-suite case requires mixed-DPI monitors and the host exposes only one display
- **THEN** the case is blocked with monitor/DPI facts and the full run is incomplete

#### Scenario: Evidence-backed not applicable
- **WHEN** a case is conditionally irrelevant and its declared predicate proves that state
- **THEN** UTIT records not-applicable with the predicate evidence and may satisfy that conditional case

#### Scenario: Filtered run
- **WHEN** case or tag filters omit any mandatory selected-suite case
- **THEN** the run is marked partial even if every executed case passes

### Requirement: Canonical multi-format reports
UTIT SHALL write one authoritative JSON report and deterministic JUnit XML plus Markdown projections, and SHALL validate existing reports without executing tests.

#### Scenario: Successful report generation
- **WHEN** a run reaches a terminal decision
- **THEN** JSON, JUnit, Markdown, per-case logs, artifact hashes, host facts, source/tool hashes, and replay argv are written under one unique run directory

#### Scenario: Report validation rejects drift
- **WHEN** `validate-report` observes duplicate IDs, invalid counts/states, missing files, changed hashes, or a decision inconsistent with case results
- **THEN** validation exits non-zero and identifies every rejected field or artifact

#### Scenario: XML and Markdown safety
- **WHEN** case titles or logs contain Unicode and XML/Markdown control characters
- **THEN** projections escape them deterministically without corrupting the authoritative values

### Requirement: Current shell-parity catalog coverage
The initial catalog SHALL include current automated and GUI gates for taskbar, Start, desktop selection, Show desktop, notifications, system status/IME/flyouts, resize/lock, auto-hide, strict OpenSpec validation, release, and packaging boundaries.

#### Scenario: Smoke suite
- **WHEN** smoke runs on the supported Windows workspace
- **THEN** catalog validation, focused UTIT tests, workspace tests, Clippy, and strict applicable OpenSpec validation execute without Explorer mutation

#### Scenario: Shell-parity suite
- **WHEN** shell-parity runs on the interactive Windows host
- **THEN** admitted preview and Explorer-absent GUI cases execute serially, screenshots/reports are hashed, and Explorer is restored after every suppression case

#### Scenario: Full suite boundary
- **WHEN** full runs without all hardware/reboot/external prerequisites
- **THEN** safe actionable cases still execute, unavailable gates remain blocked, and the report does not claim complete Windows parity
