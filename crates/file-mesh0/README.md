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
section on demand. Shared vertex bytes and render variant index bytes remain
lazy.

## Section Bodies

Most sections are simple fixed-stride tables or a single fixed-size
record:

```text
MESH_INFO
├─ MeshInfoHeader                      80 bytes
│  ├─ mesh flags/default lod
│  ├─ bounds
│  ├─ source format/version
│  ├─ primitive/vertex metadata
│  └─ vertex_buffer_size: u32
├─ padding                             zero bytes, aligned to 8
└─ shared vertex bytes

MATERIAL_SLOTS
└─ Mesh0MaterialSlot[]                 44 bytes each

SKELETON
└─ skeleton0 bytes                     present only when table file_id == 0

ANIMATION
└─ anim0 bytes                         present only when table file_id == 0

```

MESH_INFO stores the required mesh metadata and shared parent vertex buffer.
Render variants reference that shared vertex buffer directly through their index
stream. A render variant may optionally carry a `lod_level`; `NO_LOD_LEVEL`
means the variant is just a skin/render profile rather than distance LOD data.

```text
RENDER_VARIANT section body
├─ RenderVariantHeader                 88 bytes
│  ├─ render_variant_index: u32
│  ├─ lod_level: u32                   u32::MAX means none
│  ├─ primitive/index metadata
│  ├─ bounds
│  ├─ submesh_count: u32
│  ├─ draw_batch_count: u32
│  ├─ joint_palette_count: u32
│  └─ index_buffer_size: u32
│
├─ Mesh0Submesh[]                      68 bytes each
├─ Mesh0DrawBatch[]                    56 bytes each
├─ Mesh0JointPaletteEntry[]            16 bytes each
├─ padding                             zero bytes, aligned to 8
└─ index bytes
```

## Runtime And Conversion

`Mesh0Reader<R>` is the runtime API. It keeps section bodies lazy:

```text
mesh.mesh_info().await?          reads only MeshInfoHeader
info.vertex_bytes().await?       reads shared vertex bytes
mesh.render_variant(0).await?    reads render variant metadata
variant.index_bytes().await?     reads render variant index bytes
```

`Mesh0Builder` is the converter API. It owns all bytes in memory and writes a
complete mesh0 file through `write_bytes` or `write_file`.
