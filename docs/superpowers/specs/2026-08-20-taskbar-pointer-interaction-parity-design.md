# Taskbar Pointer Interaction Parity Design

## Intent

Make SuperDesktop's taskbar pointer behavior match Windows Explorer for input language, volume, notification-area icons, and application buttons, and make those behaviors mandatory UTIT gates.

## Interaction contract

- A left click performs only the control's primary action. Input and volume toggle their owned flyouts; a notification icon emits `Activate`; an application button activates, minimizes, restores, or opens its group preview from current window state.
- A right click performs only the control's contextual action. Input and volume open owned context menus; a notification icon emits `Context`; an application button opens its Jump List.
- Handled pointer events stop propagation so the taskbar background menu cannot replace a child control's menu. Opening one owned popup closes conflicting owned popups. Repeating the same invocation toggles that popup closed, and deactivation or Escape dismisses it.
- Keyboard/UIA activation remains equivalent to the left-click primary action. No caller-controlled executable or URI is introduced.

## Architecture

`taskbar-ui` owns button-specific input routing and small, testable helpers that classify primary versus contextual actions. It renders owned input/volume context menus using the existing taskbar popup styling and callback boundary. `superdesktop-app` owns popup lifetimes, positioning, fixed Windows actions, tracing, and mutual exclusion with Jump Lists, system flyouts, notification overflow, and the taskbar background menu. The notification compatibility host continues to translate `Activate` and `Context` to the negotiated NotifyIcon callback payload.

## Testing

Unit tests cover action classification, event propagation guards, exact notification event mapping, task state reduction, and popup exclusivity. UTIT adds dedicated headful cases for system controls, notification icons, and taskbar applications. Each case uses real pointer input, proves both buttons independently, asserts the unintended background menu stays absent, records trace tokens and UIA state, and writes a hashed JSON report. The existing system-status and notification compatibility captures are strengthened instead of accepting any callback as sufficient.

## Failure and safety

Unavailable audio/input providers produce truthful blocked or not-applicable evidence rather than synthetic success. Explorer-free scripts retain bounded watchdog recovery. Context actions use compile-time Windows Settings/control targets and do not accept arbitrary commands. Existing unrelated working-tree changes are preserved.

## Alternatives

Keeping only script-level assertions would preserve faulty routing. Rebuilding all taskbar controls as a new component framework would enlarge regression risk without being necessary. The selected approach centralizes semantics while retaining established view and popup infrastructure.
