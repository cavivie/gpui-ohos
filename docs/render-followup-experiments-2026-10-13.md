# Further rendering experiments, 2026-10-13

Scope is explicitly limited to gpui-ohos. NearSend layout, GPUI core, XComponent
frame demand, presentation mode, resolution, and rendering quality are unchanged.
No experimental renderer change from this pass is retained in production source.

The baseline is gpui-ohos `9ea8bc50f9dd3a03fcdf4622bddf591c4ab9e578`, NearSend
`aa8b0a36253a8135bfbc63d7f3015f575b4c6860`, Ability
`eab9d11ad00ab8ab85aff5e5c3cfa86cdd6fb5bb`, and wgpu
`8b419de0906f37af69d728f1735132e427a3436f`. All experiments use GLES on the same
ALN-AL00 / Maleoon 910 phone and retain the preceding Vulkan comparison's
instrumentation, release settings, FIFO, frame latency 2, and scroll procedure.

## Experiments and results

1. Rotate three separate 2 MiB instance buffers to test whether updating the same
   buffer as the preceding frame causes driver contention. This reserves two
   additional buffers; the prototype grows all three together on overflow.
2. Reduce the initial instance buffer from 2 MiB to 256 KiB, preserving existing
   growth behavior, to test whether driver work is sensitive to allocated size.
3. Reuse upload chunks with `wgpu::util::StagingBelt` (64 KiB chunks). Record the
   instance transfer in the frame encoder and schedule remapping on submission.
   Ordinary scene uploads change; path uploads retain their existing behavior.

Five successful trials contain 16,017 paired draw/present records. Each trial
warms settings, returns to the top, then performs twelve alternating swipes.
Logs are streamed. Times below are milliseconds, including CPU submission and
present return rather than GPU completion or panel scanout.

| Trial | Samples | Whole median | Whole P95 | Whole P99 | Platform median | Platform P95 | >8.333 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Baseline A | 3157 | 5.432 | 6.321 | 6.803 | 1.821 | 2.161 | 2 |
| Three buffers | 3311 | 5.494 | 6.514 | 7.060 | 1.843 | 2.233 | 2 |
| Baseline B | 3184 | 5.479 | 6.676 | 7.358 | 1.841 | 2.261 | 5 |
| 256 KiB buffer | 3197 | 5.457 | 6.468 | 7.020 | 1.842 | 2.250 | 3 |
| Staging belt | 3168 | 5.420 | 6.434 | 6.983 | 1.825 | 2.201 | 4 |

Buffer rotation and smaller capacity do not show a useful improvement beyond
baseline variation. The belt's sampled upload median falls from 0.077–0.078 to
0.023 ms, while the finish stage rises from 0.101–0.103 to 0.142 ms and submit
median is 0.467 ms versus baseline 0.443–0.451 ms. The finish stage includes the
belt's unmap/remap scheduling; command encoder creation also moves into its
upload interval. These are implementation-stage timings, not exclusive GPU
copy measurements. Total platform and whole-frame results show no convincing
improvement that justifies retaining the additional lifecycle complexity.

These are exploratory single candidate trials bracketed by two baseline
observations, not statistically independent per-frame experiments. Battery
temperatures were 28–31 C across the first four successful trials. Neither
thermal state nor clock frequency was held constant. Rare over-budget counts
must not be treated as a statistically reliable display-drop comparison.

## Diagnostic limitations

- The first baseline launch lost startup log lines (`write socket failed`); it
  was rejected before sampling. A fresh launch verified the backend and ran.
- The first staging-belt launch produced two process IDs, then no live app
  process, before valid capture began. The cause is unconfirmed. Protected
  system crash files were inaccessible. A diagnostic rebuild with a panic hook
  and adapter-capability log survived a ten-second probe and a subsequent fresh
  launch plus complete scroll capture without a panic. This does not establish
  the cause of the first exit or prove comprehensive lifecycle safety.
- Staging-belt overflow/retry and multiwindow behavior were not qualified for
  production. The prototype is rejected on lack of demonstrated overall gain,
  without making an unsupported claim that the wgpu API is unusable on GLES.

Follow-up verification is recorded in [path target and storage experiments](render-path-storage-experiments-2026-10-13.md).

## Remaining opportunities within gpui-ohos

### Allocate path render targets only when needed

`WgpuRenderer::new` and `update_drawable_size` currently create the full-size
path intermediate and optional MSAA textures unconditionally. All 1,006 sampled
renderer records across the five successful trials have zero path primitives.
The ordinary quad/sprite path does not use those targets.

A renderer-owned optional path-target bundle can be created for the first scene
with paths and invalidated on resize. At 1260 x 2720 RGBA8 with 4x MSAA, the
single-sample plus multisample logical payload is approximately 65.4 MiB per
window. This is a size calculation, not measured driver memory consumption;
compression, tiling, lazy driver allocation, and the actual sample count matter.
This is primarily a memory/startup/resize opportunity, not demonstrated savings
in steady settings-scroll frame time. It must preserve sample count and verify
first-path rendering, repeated paths, resize, recreation, and capture behavior.

### Backend-specific instance transport

The device's actual GLES capability log reports 16 storage buffers per stage but
does **not** advertise `DownlevelFlags::VERTEX_STORAGE`. The wgpu GLES adapter
code can derive the count from fragment storage when vertex storage is absent;
the count alone must not be interpreted as permission to use vertex SSBOs.
Earlier measurements also lack `CLIP_DISTANCES` and `MAPPABLE_PRIMARY_BUFFERS`.

The pinned upstream `gpui_wgpu` separates storage-buffer and texture instance
transport in `shaders_storage.wgsl` / `shaders_webgl.wgsl`. A future Vulkan
prototype can explicitly check vertex-storage capabilities, request suitable
limits, and evaluate storage-buffer transport rather than reusing OHOS's GLES
vertex-attribute transport. This needs renderer/context/layout/shader changes,
buffer-binding alignment and bounds checks, image comparisons, and device A/B
measurement. No speedup is claimed here. GLES texture-based transport is another
possible experiment, with extra upload/fetch costs that could outweigh savings.

### Surface presentation work

The earlier CPU stack shows native-window buffer acquisition/IPC underneath
`glBlitFramebuffer`, not just exclusive pixel-copy work. Therefore deleting the
flip copy cannot be credited with the whole present duration. Direct window
rendering would require a native GLES presentation path or a wgpu HAL change;
the latter is outside a strict gpui-ohos-only change. A native renderer is a
larger maintenance commitment and still pays native buffer acquisition costs.
Its benefit must be measured before replacing the current backend.

## Artifacts and verification

Raw data, scripts, candidate native libraries/HAPs, configuration and capability
logs, failed startup evidence, and per-trial analyses were kept in a local
capture directory and are not archived in this repository.

All candidate ARM64 release builds complete. Packaged allocated ELF sections
match their corresponding native outputs and HAPs have `debug=false`. Settings
screenshots for all three successful candidates match the baseline pixel for
pixel in `(0, 140, 1260, 2580)`, excluding system bars. No production Rust code
or NearSend UI source is modified. The previously verified clean GLES release
is restored after diagnostics; installation and smoke evidence are recorded
in the capture directory.
