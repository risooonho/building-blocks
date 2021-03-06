# building-blocks

[![Crates.io](https://img.shields.io/crates/v/building-blocks.svg)](https://crates.io/crates/building-blocks)
[![Docs.rs](https://docs.rs/building-blocks/badge.svg)](https://docs.rs/building-blocks)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/d/building-blocks.svg)](https://crates.io/crates/building-blocks)
[![Discord](https://img.shields.io/discord/770726405557321778.svg?logo=discord&colorB=7289DA)](https://discord.gg/CnTNjwb)

Building Blocks is a voxel library for real-time applications.

![Meshing](https://i.imgur.com/IZwfRHc.gif)

The primary focus is core data structures and algorithms. Features include:

- memory-efficient storage of voxel maps
  - a `ChunkMap` with generic chunk storage
  - LRU-cached storage of compressed chunks
  - compressed serialization format
  - `OctreeSet` bitset structure
- mesh generation
  - isosurface
  - cubic / blocky
  - height map
- accelerated spatial queries
  - ray casting and sphere casting
  - range queries
- procedural generation
  - sampling signed distance fields
  - constructive solid geometry (TODO)
- pathfinding on voxel maps

## Short Code Example

The code below samples a [signed distance field](https://en.wikipedia.org/wiki/Signed_distance_function) and generates a
mesh from it.

```rust
use building_blocks::{
    prelude::*,
    mesh::{SurfaceNetsBuffer, surface_nets},
    procgen::signed_distance_fields::sphere,
};

let center = PointN([25.0; 3]);
let radius = 10.0;
let sphere_sdf = sphere(center, radius);

let extent = Extent3i::from_min_and_shape(PointN([0; 3]), PointN([50; 3]));
let mut samples = Array3::fill_with(extent, &sphere_sdf);

let mut mesh_buffer = SurfaceNetsBuffer::default();
surface_nets(&samples, samples.extent(), &mut mesh_buffer);
```

## Learning

### Docs and Examples

The current best way to learn about the library is to read the documentation and examples. For the latest stable docs, look
[here](https://docs.rs/building_blocks/latest/building_blocks). For the latest unstable docs, clone the repo and run

```sh
cargo doc --open
```

There is plentiful documentation with examples. Take a look in the `examples/` directory to see how Building Blocks can be
used in real applications.

### Getting Started

This library is organized into several crates. The most fundamental are:

- **core**: lattice point and extent data types
- **storage**: storage for lattice maps, i.e. functions defined on `Z^2` and `Z^3`

Then you get extra bits of functionality from the others:

- **mesh**: 3D mesh generation algorithms
- **procgen**: procedural generation of lattice maps
- **search**: search algorithms on lattice maps

To learn the basics about lattice maps, start with these doc pages:

- [points](https://docs.rs/building_blocks_core/latest/building_blocks_core/point/struct.PointN.html)
- [extents](https://docs.rs/building_blocks_core/latest/building_blocks_core/extent/struct.ExtentN.html)
- [arrays](https://docs.rs/building_blocks_storage/latest/building_blocks_storage/array/index.html)
- [access traits](https://docs.rs/building_blocks_storage/latest/building_blocks_storage/access/index.html)
- [chunk maps](https://docs.rs/building_blocks_storage/latest/building_blocks_storage/chunk_map/index.html)
- [transform maps](https://docs.rs/building_blocks_storage/latest/building_blocks_storage/transform_map/index.html)
- [fn maps](https://docs.rs/building_blocks_storage/latest/building_blocks_storage/func/index.html)

### Benchmarks

To run the benchmarks (using the "criterion" crate), go to the root of a crate and run `cargo bench`.

## Configuration

### LTO

It is highly recommended that you enable link-time optimization when using building-blocks. It will improve the performance
of critical algorithms like meshing by up to 2x. Just add this to your Cargo.toml:

```toml
[profile.release]
lto = true
```

### Cargo Features

Building Blocks is organized into several crates, some of which are hidden behind features, and some have features
themselves, which get re-exported by the top-level crate.

#### Math Type Conversions

The `PointN` types have conversions to/from `glam`, `nalgebra`, and `mint` types by enabling the corresponding feature.

#### Compression Backends and WASM

Chunk compression supports two backends out of the box: `Lz4` and `Snappy`. They are enabled with the "lz4" and "snappy"
features. "lz4" is the default, but it relies on a C++ library, so it's not compatible with WASM. But Snappy is pure Rust,
so it can! Just use `default-features = false` and add "snappy" to you `features` list, like so:

```toml
[dependencies.building-blocks]
version = "0.4.1"
default-features = false
features = ["snappy"]
```

#### VOX Files

".VOX" files are supported via the `dot_vox` crate. Enable the `dot_vox` feature to expose the generic `encode_vox` function
and `Array3::decode_vox` constructor.

#### Images

Arrays can be converted to `ImageBuffer`s and constructed from `GenericImageView`s from the `images` crate. Enable the
`images` feature to expose the generic `encode_image` function and `From<Im> where Im: GenericImageView` impl.

## Development

We prioritize work according to the [project board](https://github.com/bonsairobo/building-blocks/projects/1).

If you'd like to make a contribution, please first read the **[design philosophy](DESIGN.md)** and **[contribution
guidelines](CONTRIBUTING.md)**.

License: MIT
