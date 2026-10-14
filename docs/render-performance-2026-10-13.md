# Render cost follow-up, 2026-10-13

This pass compares the pinned GPUI backends and reduces OHOS renderer work. It
does not change NativeVSync, XComponent frame demand, animation stepping, GLES
FIFO presentation, resolution, antialiasing, or GPUI core.

The follow-up [Vulkan comparison](vulkan-comparison-2026-10-13.md) measures both
backends on this optimized code and records why GLES remains the default.

[Further renderer experiments](render-followup-experiments-2026-10-13.md) test
buffer rotation, smaller capacity, and reusable upload chunks within gpui-ohos.

[Path target and storage experiments](render-path-storage-experiments-2026-10-13.md)
verify lazy allocation savings and two Vulkan storage prototypes.

[Direct-window experiment](direct-window-experiment-2026-10-13.md) validates a
GLES default-framebuffer prototype and records where buffer acquisition moves.

## Source comparison

Sources are GPUI `ohos-rs/zed@f45c7c22d0`, wgpu `8b419de0906f37af69d728f1735132e427a3436f`,
and the OHOS baseline `fda4f2e`. Ability remains at `eab9d11` (keyboard, mouse and
drag features; released ArkUI/XComponent/UDMF bindings).

| Backend | Relevant implementation | OHOS implication |
| --- | --- | --- |
| macOS Metal | `gpui_apple/src/metal_renderer.rs`: writes instances into shared/managed buffers from a pool; returns buffers on command completion. `shaders.metal` uses hardware clip distances for quads, shadows and underlines. | Avoid repeated staging allocations and reject invisible geometry before fragment shading. Direct mapped-buffer reuse needs its own GPU-completion ownership scheme. |
| Windows D3D11 | `gpui_windows/src/directx_renderer.rs`: uploads scene buffers using `D3D11_MAP_WRITE_DISCARD`, draws ranges and presents the swap chain. `shaders.hlsl` uses `SV_ClipDistance`. | Preserve scene ordering while reducing upload overhead. D3D mapping and clip-distance support cannot be assumed on GLES. |
| Shared wgpu / Linux | `gpui_wgpu/src/wgpu_renderer.rs`: storage-buffer instance transport (texture transport for WebGL), per-scene bindings and cached atlas bindings. | The OHOS renderer targets GLES environments without vertex SSBOs; this pass retains its tested vertex-attribute transport. OHOS already caches atlas bindings, pipelines and stable uniforms. |
| OHOS wgpu/GLES | `wgpu-hal/src/gles/egl.rs::Surface::present` blits the rendered buffer to the EGL framebuffer with a Y flip, then swaps buffers. | This full-frame operation is visible in the CPU stack. Removing it needs a separate backend design preserving orientation, readback, transparency and lifecycle behavior; it is not a frame-scheduling fix. |

The tested Maleoon 910 GLES adapter does **not** expose wgpu `CLIP_DISTANCES` or
`MAPPABLE_PRIMARY_BUFFERS`. No unsupported feature is enabled.

## Retained changes

1. Quads, shadows and underlines with integral content masks shrink their vertex
   geometry to the mask. Original shape bounds, radii, colors and fragment clipping
   remain unchanged. Textured and transformed sprites retain their existing path.
   Fractional masks retain the original geometry: a prototype that also shortened
   fractional masks changed one pixel at a half-pixel boundary due to interpolation
   rounding. That prototype was rejected.
2. The six ordinary scene instance arrays share one `Queue::write_buffer_with`
   allocation and queued transfer. Their existing alignment, strides, primitive
   order and path-data offsets are preserved. Capacity is checked before writing,
   padding is initialized, and empty/paths-only scenes skip the transfer. Path
   rasterization uploads retain their existing handling.

An extra rounded-fill pipeline was tested and discarded because it did not show a
consistent improvement over the simpler candidate.

## Device measurements

Device: ALN-AL00 / Maleoon 910, 1260 x 2720. NearSend `aa8b0a3`, latest Ability
integration, release-optimized Rust, warmed settings page, 12 alternating scroll
gestures per run. All variants use the same temporary profiler; renderer stages
are sampled every 16 frames. Final comparisons stream hilog continuously so the
bounded device log ring cannot discard the beginning of a run. Earlier snapshot
captures were used only for exploration.

The final comparison order is optimized A, baseline, optimized B:

| Metric (ms) | Baseline | Optimized A | Optimized B |
| --- | ---: | ---: | ---: |
| Matched draw/present samples | 3217 | 3175 | 3172 |
| GPUI draw median | 3.542 | 3.591 | 3.500 |
| GPUI draw P95 | 4.405 | 4.437 | 4.248 |
| Platform draw/present median | 1.844 | 1.817 | 1.800 |
| Platform draw/present P95 | 2.447 | 2.196 | 2.297 |
| Instance upload median | 0.090 | 0.077 | 0.075 |
| Queue submission median | 0.477 | 0.449 | 0.435 |
| Draw start through present return P95 | 6.521 | 6.482 | 6.374 |
| Draw start through present return P99 | 7.135 | 7.152 | 6.823 |
| Draw start through present return maximum | 9.096 | 9.777 | 9.155 |
| Samples exceeding 8.333 ms | 4 | 5 | 4 |
| Active draw-start interval median | 8.285 | 8.287 | 8.284 |

The renderer gains are small but repeatable: instance upload median falls by
14–17%, and platform draw/present P95 by 6–10%. There is **no demonstrated
elimination of long frames**: 9 of 6347 optimized samples exceed 8.333 ms. The
frame-interval median is compatible with 120 Hz, but it does not prove every
refresh displayed a new frame. These CPU timings include submission/present-call
return, not GPU completion, compositor scheduling or panel scanout. Intervals
around gesture transitions are not classified as display drops.

A separate 40-second, 1000 Hz `hiperf` capture (55,477 CPU-cycle samples, zero
reported sample loss) resolves Taffy flex/block layout, style refinement, element
construction, and the GLES blit/swap/driver path. It supports two further areas:
reducing repeated layout/tree work, and eliminating the backend's full-frame
presentation copy. The sampling run is excluded from timing comparisons.

## Verification and artifacts

- 76 host regressions pass; host and OHOS ARM64 all-target/all-feature Clippy pass
  with warnings denied; formatting and whitespace checks pass.
- `tests/shader-compare` renders 576 reference/candidate image pairs through Metal:
  byte-identical RGBA8 output. It covers borders, rounded corners, gradients,
  shadow blur, wavy underlines, fractional/half-pixel masks and offscreen shapes.
- The phone settings screenshots match pixel-for-pixel over the application area
  (excluding status/navigation bars); device GLES scrolling is exercised in every
  run. This is complementary to the offscreen Metal check.
- The final NearSend HAP has the same signing certificate and version 1.2.2
  (1002002), uses release mode, and contains the freshly built native sections.
  Temporary profiler collection, stage logs, capability logs and FPS HUD are absent.
- Local raw captures, CPU stacks, scripts, native artifacts and package manifests:
  `/tmp/nearsend-render-20261013`. Final HAP:
  `nearsend/dist/NearSend-1.2.2-render-perf-release.hap`.

NearSend's ordinary checkout and signing configuration are preserved. Packaging
uses an isolated snapshot with the already verified dependency integration. The
renderer changes are in `perf/ohos-render-120hz`; this report does not imply they
have been merged into `feat/pr82-ability-adaptation`.
