# OHOS adaptation performance

The optimization preserves the existing platform APIs, raw touch and ArkUI Pan
streams, system velocity, animation frame rate, presentation mode, antialiasing,
window count, pixel formats and the existing capture size guard. GPUI core is
unchanged at `f45c7c22d0`.

## Runtime ownership

- `OhosDispatcher` reuses 2–8 base workers, with a hard limit of 64 live workers.
  If queued work stalls for 10 ms and OS scheduling state shows insufficient
  capacity, a sleeping monitor starts another worker below that limit.
  Observations without growth back off to 100 ms. Dequeues notify only once per
  backed-off wait; progress, an empty queue, or growth resets the interval to
  10 ms. Additional workers retire after 1 second without work. Below the
  limit, this preserves synchronous waits on child tasks and externally queued
  work. The queue uses GPUI's 60/30/10 priority weights and FIFO within each
  priority. Realtime work retains its dedicated thread. Neither the monitor nor
  timer thread polls an empty queue. Shutdown wakes them and releases queued
  tasks.
- One timer thread owns the deadline heap and executes expired timer runnables
  without holding the timer lock, in deadline order and FIFO for equal deadlines.
  Awakened futures resume through their own executors. Foreground tasks execute
  on the UI thread; each UI turn processes up to 64 tasks or 2 ms and posts a
  continuation for remaining work. Scheduling foreground work wakes the UI,
  including when the screen has no animation.
- Windows share `MainWake` to coalesce native wake events. Each window retains
  an independent atomic frame state and generation ticket. A callback from an
  earlier surface/visibility generation cannot clear a new request.
- Healthy native VSync uses Ability's `FrameInputDelivery::OnDemand` to remove
  the unused continuous XComponent display callback. All input callbacks remain
  registered. Missing/failed native VSync restores `Continuous`; this was tested
  with forced native VSync creation failure on phone and 2in1. Resume requests
  a fresh frame and retries native VSync after a failed render session.
- The window index routes borrowed input directly to its owner. Global events
  still reach every live window. `SmallVec` has four inline entries and grows
  on the heap; four is not a window limit. Resize callbacks are deduplicated
  only when full bounds, effective content size and scale are all unchanged.
- `frame_clock.rs` maps NativeVSync `CLOCK_MONOTONIC` timestamps to the
  `Instant` clock used for input dispatch. The main queue consumes only the
  latest pending tick; late or duplicate callbacks cannot complete another
  request. Momentum advances on the delivered frame time, so queue delay is not
  added to animation time.
- The renderer reports presented, retry and unavailable surface outcomes.
  Transient acquisition failures keep presentation demand and request the next
  VSync, which presents the latest scene without another layout. Presented and
  unavailable outcomes clear demand; unavailable surfaces wait for input or
  lifecycle events instead of spinning. Initial and lifecycle demand presents
  the latest scene even when GPUI is not dirty.
- Quads use general, solid-fill or borderless-rectangle pipelines selected per
  contiguous run, preserving scene order, clipping, blending and antialiasing.
  Resolved multisample path attachments use `StoreOp::Discard`; sample count
  and resolution are unchanged.
- Windows allocate path attachments on first path use, reuse them across frames,
  and invalidate them on resize. Windows that never draw paths avoid these
  full-size textures; see [memory measurements and first-use cost](path-target-memory-2026-10-13.md).
- `WgpuContext` owns common shader/layout handles and caches pipelines by surface
  format, alpha mode, path sample count and dual-source blending. Window buffers,
  uniforms and surface textures retain their existing ownership; atlas reuse
  uses compatible layouts from the same device. No rendering quality changes.
- Capture copies and converts one row at a time instead of zeroing and rescanning
  a complete frame. RGBA, RGBX, BGRA and BGRX retain their channel/alpha semantics.

Ability branch `feat/pr82-nearsend-integration` at `ab1d6c20` supplies frame
delivery selection and releases native consumers before dropping their ArkUI
root. The earlier reverse order caused a reproduced multiwindow close crash.
The SDK contains only OHOS implementations; target-platform fallback branches
were removed without changing the native frame callback body or device classes.

## Verification

```sh
cargo test --manifest-path tests/touch/Cargo.toml --locked
cargo clippy --manifest-path tests/touch/Cargo.toml --locked --all-targets --no-deps -- -D warnings
cargo fmt --manifest-path tests/touch/Cargo.toml --check
```

61 regressions cover gestures, frame generations, frame clock calibration,
presentation retry and lifecycle demand, unavailable surfaces without idle
retries, viewport deduplication, wake coalescing, timers, blocking background
tasks, worker limits, monitor backoff, priority fairness, shutdown, cache
variants, quad pipeline eligibility and pixel conversion. ARM64 Release
integration built successfully.

The same standalone GPUI example was measured before and after optimization:

| Case | Before | Optimized |
| --- | ---: | ---: |
| Phone idle native redraw events/s | 64.0 | 0 |
| 2in1 idle native redraw events/s | 63.291 | 0 |
| Phone idle CPU, one core | 1.098% | 0 observed ticks |
| 2in1 idle CPU, one core | 1.649% | 0 observed ticks |
| Compatible pipeline sets for main + 3 child windows | 4 | 1 |
| Worker thread IDs for 2,064 cooperative continuations (host) | 2,064 | 8 |
| Median dispatcher duration for that workload (host) | 31.893 ms | 2.616 ms |

Phone and 2in1 both passed foreground/background cycles. 2in1 passed concurrent
child-window creation, independent input, close and reopen, including fallback
mode. CPU samples lasted 9 seconds at procfs 100-tick/second resolution: these
measurements do not establish zero power or a real-device thermal result. Host
timings must not be interpreted as an OHOS speedup. Temporary counters and fault
injection are absent from production source.
