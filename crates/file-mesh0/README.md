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
├─ SectionTableItem[section_count]     12 bytes each
│  ├─ section_type: u32
│  ├─ offset: u32                      absolute file offset
│  └─ len: u32                         section body byte length
│
├─ padding                             zero bytes, aligned to 8
│
└─ section bodies                      each body starts at an 8-byte boundary
   ├─ MESH_INFO
   ├─ MATERIAL_SLOTS
   ├─ SKINNING
   ├─ *_REFS
   ├─ SOURCE_FEATURES
   ├─ SOURCE_DEBUG
   └─ LOD
```

`Mesh0View<R>` stores the ordered section table. Each `SectionTableItem<R>`
stores its table metadata, an offset reader for its body, and a cached typed
`SectionView<R>`. Call `item.read_section_view()` to create or reuse the typed
view. Non-LOD sections are decoded immediately into their section structs; LOD
keeps vertex and index bytes lazy.

The section table is an ordered list. The container layer does not require any
specific section type and does not enforce uniqueness; every section is read as
`section_type + offset + len`. Higher-level code decides how to interpret zero,
one, or many sections of a type.

## Section Bodies

Most non-LOD sections are simple fixed-stride tables or a single fixed-size
record:

```text
MESH_INFO
└─ MeshInfoSection                     56 bytes

MATERIAL_SLOTS
└─ Mesh0MaterialSlot[]                 44 bytes each

SKINNING
└─ Mesh0Skinning                       36 bytes

SKELETON_REFS / ANIMATION_REFS /
EFFECT_REFS / COLLISION_REFS /
ATTACHMENT_REFS
└─ u64[]                               asset ids

SOURCE_FEATURES
└─ Mesh0SourceFeature[]                24 bytes each

SOURCE_DEBUG
└─ opaque bytes
```

LOD sections are self-contained. The layout is fixed: header, metadata arrays,
8-byte padding, vertex bytes, then index bytes. The header stores array counts
and blob sizes; offsets are derived from the fixed layout.

```text
LOD section body
├─ Mesh0LodHeader                      104 bytes
│  ├─ lod_level: u32
│  ├─ primitive/index/vertex metadata
│  ├─ bounds
│  ├─ submesh_count: u32
│  ├─ draw_batch_count: u32
│  ├─ joint_palette_count: u32
│  ├─ vertex_buffer_size: u32
│  └─ index_buffer_size: u32
│
├─ Mesh0Submesh[]                      76 bytes each
├─ Mesh0DrawBatch[]                    64 bytes each
├─ Mesh0JointPaletteEntry[]            16 bytes each
├─ padding                             zero bytes, aligned to 8
├─ vertex bytes
└─ index bytes
```

## Ownership Model

`Mesh0View<R>` reads only the file header and section table when opened.
`SectionTableItem<R>` does not expose raw byte reads. Reading a typed view
decodes non-LOD sections immediately. LOD section views read header and metadata
arrays up front, while vertex and index bytes remain lazy.

`Mesh0Owned` is the decoded in-memory form used for writing the whole file back
out. It preserves section order and allows repeated section types.
