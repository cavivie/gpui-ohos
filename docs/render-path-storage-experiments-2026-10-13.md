# Path target allocation and Vulkan storage experiments, 2026-10-13

Only gpui-ohos production code changes. NearSend pages, GPUI core, Ability,
XComponent frame scheduling, 1260 × 2720 resolution, FIFO presentation, maximum
frame latency 2, and antialiasing remain unchanged. GLES remains the default.

Baseline: gpui-ohos `9ea8bc50f9dd3a03fcdf4622bddf591c4ab9e578`, NearSend
`aa8b0a36253a8135bfbc63d7f3015f575b4c6860`, Ability
`eab9d11ad00ab8ab85aff5e5c3cfa86cdd6fb5bb`, wgpu
`8b419de0906f37af69d728f1735132e427a3436f`. Same ALN-AL00 / Maleoon 910 phone.

## Retained change: lazy path targets

`WgpuRenderer` owns an optional `PathTargets` bundle. A window creates neither
path attachment nor its texture bind group until its first scene containing
paths. The resources are reused thereafter. A drawable-size change drops the
bundle; the next path scene creates targets at the new dimensions. Views and
bind groups retain their underlying textures; wgpu manages in-flight lifetimes.
Path shaders, rasterization order, resolve behavior, and sample count are unchanged.

Memory was read with `hidumper --mem PID --show-gpumem` after the same settings
warmup and return to the top. Both independent baseline/lazy pairs reported:

| Texture allocation reported by driver | Baseline | Lazy |
| --- | ---: | ---: |
| Bytes | 87,587,968 | 16,586,368 |
| MiB | 83.530 | 15.818 |

The reduction is **71,001,600 bytes / 67.712 MiB**, or 81.1% of this texture
category. This is measured driver texture allocation, not a claim that total
process memory falls 81.1%. The unpadded logical estimate for the single-sample
and 4× MSAA attachments is 65.37 MiB; driver allocations are larger.
The final clean release also reports 16,586,368 bytes in the texture category.

This saves memory for windows that have not drawn paths. Once the synthetic path
scene runs, baseline and lazy both report 87,587,968 texture bytes. No eviction
on each path-free frame is introduced. First use pays allocation latency: the
phone probe measured **5.533 ms** to create the 4× path targets once, then reused
them. This moves work out of window construction; it does not remove that work
when paths are actually needed. Resizing a path-using window also requires new
allocation on its next path frame. A cold synthetic path frame was 23.922 ms in
the baseline and 34.025 ms in lazy; those single startup observations include
shader/driver warmup and are not controlled steady-state comparisons.

## Vulkan-only storage prototypes (not retained)

The device's Vulkan adapter advertises `VERTEX_STORAGE`, storage-buffer alignment
256, and maximum binding size 1 GiB. Its GLES adapter lacks `VERTEX_STORAGE` even
though its aggregate storage-buffer limit is nonzero; the GLES path is not forced
to use this capability.

The first Vulkan prototype keeps the coalesced upload and existing vertex/fragment
math, but loads ordinary primitive attributes from a read-only storage buffer.
Raw u32 loads and bitcasts follow the original Rust vertex offsets, avoiding WGSL
structure-padding differences. Six per-array global/storage bind groups use the
same instance buffer, with aligned offsets. Quad, shadow, underline, and sprite
vertices use storage; path rasterization and path sprites retain vertex buffers.
No claim of complete dual-source text or multiwindow production qualification is
made for this discarded prototype.

The second prototype caches the six bindings by offset, binds the remaining
buffer range, and invalidates the cache on buffer growth. It removes the obvious
per-frame bind-group allocation cost before assessing this direction.

First prototype upload median is 0.120–0.125 ms versus Vulkan baseline
0.050–0.051 ms in A/B. With caching it falls to 0.053–0.054 ms. Nevertheless,
cached platform medians are 1.668 / 1.644 ms versus baseline 1.636 / 1.631 /
1.607 ms. There is no repeatable platform or whole-frame improvement that
justifies adding this backend-specific transport and cache lifecycle.

## Timing trials

All eleven runs use identical profiler/stage instrumentation and twelve warmed,
alternating settings swipes; they contain **35,318 paired draw/present records**.
Order below is chronological. “Whole” is CPU draw start through present return;
it is not GPU completion, compositor deadline compliance, or physical panel
scanout. Platform measures the gpui-ohos renderer call. Times are milliseconds.

| Trial | Frames | Whole median | Whole P95 | Whole P99 | Platform median | Platform P95 | >8.333 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| gles-a | 3385 | 5.482 | 6.745 | 7.728 | 1.841 | 2.254 | 10 |
| lazy-a | 3178 | 5.427 | 6.486 | 7.125 | 1.836 | 2.222 | 6 |
| gles-b | 3259 | 5.487 | 6.783 | 7.877 | 1.821 | 2.305 | 12 |
| lazy-b | 3184 | 5.457 | 6.583 | 7.316 | 1.844 | 2.153 | 9 |
| vulkan-a | 3181 | 5.243 | 6.463 | 7.046 | 1.636 | 2.413 | 4 |
| ssbo-a | 3171 | 5.260 | 6.453 | 7.162 | 1.660 | 2.384 | 6 |
| vulkan-b | 3178 | 5.222 | 6.448 | 7.132 | 1.631 | 2.279 | 7 |
| ssbo-b | 3206 | 5.061 | 6.215 | 6.708 | 1.640 | 2.333 | 3 |
| cached-a | 3226 | 5.245 | 6.549 | 7.286 | 1.668 | 2.347 | 7 |
| vulkan-c | 3160 | 5.204 | 6.259 | 6.673 | 1.607 | 2.348 | 6 |
| cached-b | 3190 | 5.217 | 6.304 | 6.719 | 1.644 | 2.273 | 9 |

The lazy GLES medians differ by less than 1.1% from their preceding baselines;
platform time does not consistently improve. Treat it as a memory optimization,
not proof of a frame-rate increase. SSBO B's lower whole median coincides with
lower GPUI CPU draw time (3.302 ms versus preceding baseline 3.502 ms), while
platform time is 1.640 versus 1.631 ms. It cannot be attributed to faster SSBO
rendering. Battery temperatures range from 33 to 36 C; clocks are not fixed.
Frames within one gesture are correlated and the small over-budget counts do
not establish a statistically reliable display-drop change. These results do
not certify uninterrupted 120 Hz presentation.

## Functional and artifact verification

- Production path-target GPU test passes on Metal: first use, repeated clear /
  resolve / texture sampling, resize, 1× and 4× sampling; exact pixel readback.
  The harness now lives in its own crate:
  `cargo test --manifest-path tests/path-targets/Cargo.toml --locked`.
- Separate real-device GLES scene renders three overlapping translucent curved
  paths interleaved with quads. Baseline and lazy match exactly over 3,074,400
  application pixels. Lazy allocation logs occur once, with sample count 4;
  24 sampled renderer records exercise subsequent frames.
- All eleven settings screenshots match exactly over the same application area.
- An isolated storage version of the shader harness compares 576 quad, shadow,
  and underline cases on Metal with zero pixel differences. Phone screenshots
  additionally exercise ordinary text sprites. This validates the experiment,
  not every possible storage-backend workload.
- All 76 existing host regressions pass. OHOS all-target/all-feature Clippy and
  shader-harness all-target Clippy pass with `-D warnings`; formatting and diff
  whitespace checks pass. OHOS Clippy uses the SDK clang/clang++/llvm-ar and
  target sysroot, not the host compiler.
- All eight HAP artifacts have native allocated ELF sections identical to their
  fresh build outputs. The final HAP signature verifies against the same device
  certificate, version 1.2.2 (1002002), release mode, `debug=false`.
- One preliminary synthetic-path probe lost startup lines before its late log
  snapshot and was rejected. The rerun captures startup before interaction and
  verifies GLES selection. No unsupported claim of a renderer crash is inferred.

## Direct native-window rendering boundary

The pinned [wgpu GLES presentation implementation](https://github.com/gfx-rs/wgpu/blob/8b419de0906f37af69d728f1735132e427a3436f/wgpu-hal/src/gles/egl.rs#L1149-L1192)
binds the offscreen framebuffer for reading and default framebuffer for drawing,
then performs the Y-flipping blit and swaps EGL buffers. Removing that operation
without changing render-target ownership and coordinate conventions is not a
valid gpui-ohos setting. A dedicated GLES renderer or a coordinated HAL change
would be a separate experiment. Neither was implemented or measured in this
pass. The preceding Vulkan experiment already removes this GLES presentation
path, without demonstrating a large steady-state frame-time improvement.

The subsequent [direct-window experiment](direct-window-experiment-2026-10-13.md)
implements and measures that isolated HAL/renderer prototype.

## Installed result

Only lazy path allocation is retained in production source on
`perf/ohos-render-120hz`; the storage prototypes remain isolated under `/tmp`.
No merge into `feat/pr82-ability-adaptation` or push is implied.

`nearsend/dist/NearSend-1.2.2-lazy-path-release.hap` is installed on the phone.
SHA-256: `22fa12c3962b615148406747ad8822d229aa6c252399ed508603dc00d604bdf2`.
The final package uses GLES, has no profiler, FPS HUD, synthetic path scene,
backend override or diagnostic markers. Navigation and settings scrolling pass;
the phone is returned to Receive with its existing preferences. NearSend's
ordinary checkout and signing configuration remain unchanged; packaging uses
an isolated snapshot with the already verified dependency integration.

Raw logs, measurements, prototype sources, native builds, screenshots, artifact
checks and the source-hash manifest were kept in a local capture directory and
are not archived in this repository.
