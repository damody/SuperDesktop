## Why

The borderless owned taskbar settings window exposes neither a close button nor a visible scrollbar. Users cannot discover how to dismiss the surface or see and directly control their position in its long content.

## What Changes

- Add a fixed, accessible right-top close button that uses the existing settings dismissal route.
- Track the settings content scroll state and render a visible vertical scrollbar with synchronized wheel and pointer-drag behavior.
- Hide the scrollbar when the content fits and reserve content space when it is visible.
- Add focused and headful regression evidence for close invocation, scrollbar visibility, drag movement, accessibility, and high-contrast styling.

## Capabilities

### New Capabilities

- `taskbar-settings-window-chrome`: Defines close-button behavior and visible, synchronized scrollbar interaction for the owned taskbar settings window.

### Modified Capabilities

None. Related taskbar settings changes remain historical and no archived base capability currently owns this window-chrome contract.

## Impact

The change affects `taskbar-ui` settings rendering/state, its headful example, the settings layout capture script, focused tests, and evidence. It does not change settings persistence, public APIs, dependencies, window dimensions, or other surfaces.
