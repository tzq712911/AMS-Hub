Upstream: yanshay/esp-hal-app tag 0.6.1, commit 43daad9.
Local runtime changes:
- touch.rs: active-contact fallback 200 ms -> 16 ms; repeated identical contacts are ignored, I2C errors propagate without panic, empty IRQ reads yield rather than spin.
- touch_events.rs: hardware-independent contact state and bounded FIFO; press/release order and the full drag excursion are preserved. Overflow cancels incomplete input rather than replaying partial clicks.
- wt32_sc01_plus.rs: sample touch at 8 ms intervals between synchronous render lines. Dispatch after drawing, never re-enter Slint during rendering. Drain queued input before the next redraw. Record slow frames for diagnosis.
- lib.rs: export the pure touch_events module.
Manifest changes for reproducible offline builds: framework_macros uses original Git tag; esp-mbedtls uses the same existing local wrapper as the root patch. No TLS behavior changes.
- Rendering: retain line rendering (the PSRAM full-frame experiment was reverted after 500–680 ms measurements). Keep per-stage timing, drain buffered input before redraw, and yield once per event-loop pass.
- Build: i-slint-core and framework use speed optimisation. No custom framebuffer/experimental renderer feature is required.
