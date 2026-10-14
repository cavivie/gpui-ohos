# GLES direct-window experiment, 2026-10-13

Direct default-framebuffer rendering works in this real-device prototype, but
it does not demonstrate a useful 120 Hz frame-time improvement. Two alternating
A/B pairs contain **13,007 paired draw/present records**. Whole-frame median
changes from **5.466 to 5.434 ms (0.6%)**, while the gpui-ohos renderer median
increases from **1.819 to 1.861 ms**. The remaining measurable benefit is another
**14,200,320 bytes / 13.542 MiB** of driver texture allocation avoided.

No production renderer or dependency change is retained from this experiment.
The phone is restored to the clean lazy-path release from the preceding pass.
The [prototype patches and structured measurements](experiments/direct-window-2026-10-13/README.md)
are archived for review; this does not imply production readiness or an upstream
commit/merge.

## Implementation and controls

Sources remain NearSend `aa8b0a36253a8135bfbc63d7f3015f575b4c6860`, gpui-ohos
`9ea8bc50f9dd3a03fcdf4622bddf591c4ab9e578` plus lazy path allocation, Ability
`eab9d11ad00ab8ab85aff5e5c3cfa86cdd6fb5bb`, and wgpu
`8b419de0906f37af69d728f1735132e427a3436f`. An isolated copy of all ten locked
wgpu packages is selected with Cargo path patches; other dependency versions
are unchanged. Neither the Cargo git cache nor production dependency manifests
are modified.

Both timing variants use the same release build, instrumentation, shader source,
and HAL source; `NSDIRECT` selects the presentation path and its uniform flag.
Device is ALN-AL00 / Maleoon 910, 1260 × 2720, GLES BGRA8/Opaque, FIFO, frame
latency 2. Page structure, GPUI layout, Ability code, XComponent scheduling,
AA, and resolution stay unchanged. Each trial warms settings, returns to the
top, and performs twelve alternating swipes. Battery temperatures are 33–35 C;
clock frequencies are not fixed.

Changes required for a valid direct-render test:

- Acquire a `DefaultRenderbuffer` texture tied to the native EGL surface, instead
  of allocating a full-screen offscreen renderbuffer and framebuffer.
- Bind that surface while executing its queue submission; ordinary resource
  operations retain the original context-lock/pbuffer behavior.
- Render to framebuffer zero with its default draw-buffer selection. Preserve
  viewport and scissor coordinate conventions for the direct target.
- Flip main-target vertex coordinates and fragment position interpretation
  together. Offscreen path rasterization keeps its original coordinates;
  path compositing and texture coordinates stay consistent.
- Present through EGL swap without the Y-flipping full-screen blit. Path MSAA
  resolves are retained when paths are drawn.

These changes follow the distinction between an EGL context's draw/read surfaces
in the [EGL specification, section 3.7.3](https://registry.khronos.org/EGL/specs/eglspec.1.5.withchanges.pdf)
and the original [wgpu GLES presentation implementation](https://github.com/gfx-rs/wgpu/blob/8b419de0906f37af69d728f1735132e427a3436f/wgpu-hal/src/gles/egl.rs#L1149-L1192).
Removing only the blit would not produce a valid comparison.

## Results

Times below are milliseconds. “Whole” means GPUI CPU draw start through present
return; it does not measure GPU completion or physical display scanout.

| Metric | Offscreen + blit | Direct window |
| --- | ---: | ---: |
| Paired records | 6501 | 6506 |
| Whole median | 5.466 | 5.434 |
| Whole mean | 5.519 | 5.499 |
| Whole P95 | 6.633 | 6.699 |
| Whole P99 | 7.418 | 7.565 |
| Whole >8.333 ms | 9 | 18 |
| Platform median | 1.819 | 1.861 |
| Platform P95 | 2.230 | 2.495 |
| Submit median | 0.465 | 0.860 |
| Present median | 1.063 | 0.632 |
| Active draw interval median | 8.285 | 8.286 |

Individual whole medians are 5.466 / 5.466 ms for baseline A/B and 5.426 / 5.442 ms
for direct A/B. The tiny whole difference is not a consistent renderer gain:
GPUI CPU draw median also varies (3.607 → 3.521 ms), and platform median increases.
P95/P99 do not improve. These correlated frame samples and rare over-budget
counts do not establish a statistically reliable display-drop change. The
8.285/8.286 ms active callback intervals do not certify uninterrupted 120 Hz
panel output.

Driver memory after the same warmed settings sequence is identical in both pairs:

| Driver category | Offscreen + blit | Direct window |
| --- | ---: | ---: |
| Texture bytes | 16,586,368 | 2,386,048 |
| Texture MiB | 15.818 | 2.276 |
| Graph PSS, KiB | 54,400 | 54,400 |

The 13.542 MiB reduction removes the extra full-screen color attachment. Native
window/compositor buffers remain. This is not a total-process memory reduction
of the same percentage, nor an energy/power measurement.

## Where the time moved

A separate 40-second, 1000 Hz `hiperf` recording contains 107,896 samples with
zero lost records. It is excluded from the timing table. There are no
`glBlitFramebuffer` entries in the sampled direct call tree (settings contain
no paths); the compiled direct presentation branch also has no blit call.
The prominent window-buffer acquisition chain now sits under queue submission:

```
wgpu-hal GLES Queue::submit
  glClearBufferfv
    driver
      OH_NativeWindow_NativeWindowRequestBuffer
        ProducerSurface::RequestBuffer
          BufferClientProducer::RequestBuffer
            IPCObjectProxy::SendRequest
```

The preceding offscreen CPU trace put this buffer-request chain underneath the
present blit. `eglSwapBuffers` still reaches `NativeWindowFlushBuffer` and
`ProducerSurface::FlushBuffer`. Thus the reduction in present time is largely
offset by work paid during submit; it is not an equivalent reduction in the
frame budget. Samples identify the call path, not an exclusive GPU copy time.

## Functional validation and limitations

- Both settings A/B screenshot pairs match exactly over 3,074,400 application
  pixels, excluding system status/navigation bars.
- A separate debug scene draws three translucent curved paths interleaved with
  quads, using 4× path MSAA and a non-centered scissor. Baseline and direct
  images match exactly over the same area, exercising default/offscreen target
  transitions and continued main passes with `Load`.
- The direct debug scene survives Home/background then foreground resume in the
  same process. Its resumed image matches exactly as well.
- Debug/validation flags are enabled only in those synthetic-scene packages.
  The first post-submit GL error query reports **1280 in both variants**.
  Baseline also logs repeated invalid-capability debug messages; direct has no
  subsequent API-error callback in the captured probe. The cause of the initial
  error is not isolated. An initial strict zero-error probe was rejected, then
  both results were captured as a diagnostic limitation. Do not describe this
  as clean GL-driver validation or infer an error newly caused by direct drawing.
- OHOS `cargo clippy -p wgpu-hal --tests --target aarch64-unknown-linux-ohos`
  completes without warnings after using the SDK toolchain; formatter checks and
  both archived patches' `git apply --check` pass. The original local wgpu-core
  release build emits an unrelated unfulfilled lint-expectation warning.
- Full WebGPU CTS, arbitrary surface readback/copies, context loss, window resize,
  and simultaneous multiwindow submissions are not qualified. The prototype
  explicitly rejects submissions containing different native-window surfaces.
- All four HAPs are release `debug=false`; their allocated native ELF sections
  match the fresh native build artifacts. Renderer debug flags in the two
  synthetic probes are separate from the HAP `debug` setting.

## Restored phone and artifacts

Installed again: `nearsend/dist/NearSend-1.2.2-lazy-path-release.hap`, SHA-256
`22fa12c3962b615148406747ad8822d229aa6c252399ed508603dc00d604bdf2`.
It retains only the already verified lazy path allocation; it has no direct
HAL patch, profiler, backend override, synthetic scene, or diagnostic logs.
Navigation and scrolling smoke checks pass and the phone is returned to Receive.

Raw evidence, source snapshots, scripts and diagnostic HAPs were kept in a
local capture directory and are not archived in this repository. The archived
JSON and patches accompany this report. NearSend's pre-existing signing-file edit and all pre-existing
working-tree changes are preserved.
