# Lazy path targets, 2026-10-13

Each window now allocates its intermediate path texture, optional MSAA texture
and texture bind group only when its first scene contains paths. `PathTargets`
owns the views and bind group, which retain their textures through wgpu.
Subsequent frames reuse the bundle. A drawable-size change drops it, and the
next path scene allocates attachments at the new size. Path quality, sample
count, draw order and resolve behavior remain unchanged.

The change was measured on an ALN-AL00 / Maleoon 910 phone using NearSend settings
with GLES. The experiment baseline was gpui-ohos `9ea8bc5`, NearSend `aa8b0a3`,
Ability `eab9d11`, and wgpu `8b419de0906f37af69d728f1735132e427a3436f`.
These measurements predate the independent migration to
`feat/pr82-ability-adaptation`; they are not a benchmark of that branch's other
rendering changes.

After identical settings warmup, both independent before/after pairs reported
the following driver texture allocation via `hidumper --mem PID --show-gpumem`:

| Allocation | Eager | Lazy |
| --- | ---: | ---: |
| Texture bytes | 87,587,968 | 16,586,368 |
| Texture MiB | 83.530 | 15.818 |

This saves **67.712 MiB** for windows that have not drawn paths. It is a texture
allocation measurement, not a total process-memory reduction. Once paths are
drawn, both versions allocate the same 87,587,968 texture bytes. Targets are
retained across path-free frames; this change does not add per-frame eviction.

The phone probe measured **5.533 ms** for the first 4× target allocation.
Allocation work moves from window construction to first path use and recurs
after resize. Settings renderer timings did not consistently improve, so this
is a memory optimization without a claimed frame-rate increase.

The standalone [GPU regression](../tests/path-targets/README.md) checks the
production owner with exact pixel readback for first use, reuse, resize, 1× and
4× sampling. A separate GLES phone scene with three translucent curved paths
interleaved with quads matched the eager baseline exactly over 3,074,400 app
pixels. Allocation was observed once and resources reused on subsequent frames.
