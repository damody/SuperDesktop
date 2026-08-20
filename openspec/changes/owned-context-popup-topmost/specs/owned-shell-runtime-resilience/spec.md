## ADDED Requirements

### Requirement: Asynchronous UI updates are fallible and non-panicking
SuperDesktop SHALL use a fallible GPUI application update path for callbacks that resume after asynchronous waits.

#### Scenario: Application context is already borrowed
- **WHEN** an asynchronous SuperDesktop callback resumes while GPUI's application context is mutably borrowed
- **THEN** the update returns an error instead of panicking
- **AND** SuperDesktop writes a contextual console/trace error
- **AND** the process remains alive

#### Scenario: Repeating refresh tick is rejected
- **WHEN** borrow contention rejects an auto-hide, transfer, or refresh tick
- **THEN** that tick makes no partial UI mutation
- **AND** the loop remains available for its next scheduled tick

#### Scenario: One-shot timer is rejected
- **WHEN** borrow contention rejects a preview or shutdown timer update
- **THEN** SuperDesktop reports rejection without `RefCell already borrowed` panic output

### Requirement: AppBar unavailability is recoverable
Failure to register the owned taskbar as an AppBar SHALL NOT terminate SuperDesktop.

#### Scenario: Initial AppBar registration fails
- **WHEN** the shell rejects `ABM_NEW` for the owned taskbar
- **THEN** SuperDesktop records `taskbar:appbar-unavailable-owned-shell`
- **AND** retains the owned taskbar using bounded monitor geometry
- **AND** continues processing pointer and refresh events

#### Scenario: AppBar-unavailable stress interval
- **WHEN** SuperDesktop runs through repeated refresh and popup activity after AppBar registration failure
- **THEN** the process remains alive for the entire bounded test interval
- **AND** stdout/stderr contains no `RefCell already borrowed` panic

### Requirement: Runtime resilience evidence
Headful UTIT SHALL prove popup topmost behavior and owned-shell survival under the reported failure conditions.

#### Scenario: Focused runtime UTIT passes
- **WHEN** the popup/runtime resilience case completes
- **THEN** its report records topmost state for every supported independent context popup
- **AND** records successful focus-loss dismissal
- **AND** records process survival after AppBar-unavailable trace
- **AND** records no RefCell borrow panic
