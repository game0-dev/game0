# file-mesh0

`file-mesh0` stores one mesh as a small header, a section table, and aligned
section bodies. All integer and float fields are little-endian.

## File Layout

```text
Mesh0 file
├─ Mesh0Header                         8 bytes
│  ├─ version: u32
│  └─ section_count: u32
│
├─ SectionTableItem[section_count]     16 bytes each
│  ├─ section_type: u32
│  ├─ file_id: u32                     0 means inline section body
│  ├─ offset: u32                      absolute file offset
│  └─ len: u32                         section body byte length
│
├─ padding                             zero bytes, aligned to 8
│
└─ section bodies                      each body starts at an 8-byte boundary
   ├─ MESH_INFO
   ├─ MATERIAL_SLOTS
   ├─ SKELETON
   ├─ ANIMATION
   └─ RENDER_VARIANT
```

`Mesh0Reader<R>` reads the file header and section table, then classifies table
entries into strongly typed fields. `MESH_INFO` is required, most metadata
sections are optional singletons, and `RENDER_VARIANT` may appear multiple
times. Duplicate singleton sections are rejected.

Opening a reader does not read section bodies. Each accessor reads and caches its
section on demand. Render variant vertex and index bytes remain lazy.

## Section Bodies

Most sections are simple fixed-stride tables or a single fixed-size
record:

```text
MESH_INFO
└─ MeshInfoHeader                      28 bytes
   ├─ bounding_box_min: [f32; 3]
   ├─ bounding_box_max: [f32; 3]
   └─ bounding_sphere_radius: f32

SKELETON
└─ skeleton0 bytes                     present only when table file_id == 0

ANIMATION
└─ anim0 bytes                         present only when table file_id == 0

```

MESH_INFO stores source model visual bounds. Render variants use a fixed M2-like
skinned vertex layout with position, normal, uv0, uv1, joints, and weights.

```text
RENDER_VARIANT section body
├─ RenderVariantHeader                 20 bytes
│  ├─ submesh_count: u32
│  ├─ draw_batch_count: u32
│  ├─ joint_palette_count: u32
│  ├─ vertex_buffer_size: u32
│  └─ index_buffer_size: u32
│
├─ Mesh0Submesh[]                      80 bytes each
├─ Mesh0DrawBatch[]                    76 bytes each
├─ Mesh0JointPaletteEntry[]            16 bytes each
├─ padding                             zero bytes, aligned to 8
├─ vertex bytes
├─ padding                             zero bytes, aligned to 8
└─ index bytes
```

## Runtime And Conversion

`Mesh0Reader<R>` is the runtime API. It keeps section bodies lazy:

```text
mesh.read_mesh_info().await?          reads only MeshInfoHeader
mesh.read_render_variant(0).await?    reads render variant metadata
variant.vertex_bytes().await?         reads render variant vertex bytes
variant.index_bytes().await?          reads render variant index bytes
```

`Mesh0Builder` is the converter API. It owns all bytes in memory and writes a
complete mesh0 file through `write_bytes` or `write_file`.
