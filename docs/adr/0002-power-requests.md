# 0002. Power requests instead of input injection

Date: 2026-11-07 · Status: accepted

## Context

The original script pressed Alt every fifteen minutes. That keeps the machine
awake but injects input into the focused application, cannot be inspected,
and keeps working even when the lid is closed or Sleep is chosen.

Windows offers two APIs for the purpose: `SetThreadExecutionState` (older,
thread-scoped, flags) and `PowerCreateRequest` + `PowerSetRequest` (Windows
7+, reference counted, carries a reason string).

## Decision

Power requests with a reason string ("KeepMeAwake is keeping this PC awake").
Hold `PowerRequestSystemRequired` while awake and additionally
`PowerRequestDisplayRequired` when "Keep the screen on" is set. Release on
toggle off, on pause, and on drop.

## Consequences

- `powercfg /requests` shows the app by name.
- Windows releases the request on lid close, power button and Start → Sleep,
  which is the behaviour a person expects.
- On Modern Standby laptops on battery, Windows terminates system requests
  five minutes after the sleep timeout. The battery rules make this mostly
  moot; the limitation is documented in the field guide, not worked around.
- Power requests do not reset the user-idle timer that chat applications
  read; see the roadmap for how that is treated.
