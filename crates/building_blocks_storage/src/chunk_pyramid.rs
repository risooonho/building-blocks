use crate::{prelude::*, ArrayIndexer, BytesCompression, ChunkDownsampler};

use building_blocks_core::prelude::*;

use core::hash::Hash;
use fnv::FnvHashMap;
use std::fmt::Debug;

/// A set of `ChunkMap`s used as storage for voxels with variable level of detail (LOD).
///
/// All chunks have the same shape, but the voxel size doubles every level of the pyramid.
///
/// There is no enforcement of a particular occupancy, allowing you to use this as a cache. Typically you will have some region
/// of highest detail close to a central point. Then as you get further from the center, the detail drops.
pub struct ChunkPyramid<N, T, Meta, Store> {
    levels: Vec<ChunkMap<N, T, Meta, Store>>,
}

impl<N, T, Meta, Store> ChunkPyramid<N, T, Meta, Store> {
    pub fn levels_slice(&self) -> &[ChunkMap<N, T, Meta, Store>] {
        &self.levels[..]
    }

    pub fn level(&self, lod: u8) -> &ChunkMap<N, T, Meta, Store> {
        &self.levels[lod as usize]
    }

    pub fn level_mut(&mut self, lod: u8) -> &mut ChunkMap<N, T, Meta, Store> {
        &mut self.levels[lod as usize]
    }
}

impl<N, T, Meta, Store> ChunkPyramid<N, T, Meta, Store>
where
    N: ArrayIndexer<N>,
    PointN<N>: IntegerPoint<N>,
    T: Copy,
    Meta: Clone,
    Store: ChunkWriteStorage<N, T, Meta>,
{
    pub fn downsample_chunk<Samp>(
        &mut self,
        sampler: &Samp,
        src_chunk_key: PointN<N>,
        src_lod: u8,
        dst_lod: u8,
    ) where
        Samp: ChunkDownsampler<N, T>,
        PointN<N>: Debug,
    {
        assert!(dst_lod > src_lod);
        let lod_delta = dst_lod - src_lod;

        // A trick to borrow mutably two different levels.
        let (head, tail) = self.levels.split_at_mut(dst_lod as usize);
        let src_map = &mut head[src_lod as usize];
        let dst_map = &mut tail[dst_lod as usize - src_lod as usize - 1];

        let chunk_shape = src_map.indexer.chunk_shape();

        let dst = DownsampleDestination::for_source_chunk(chunk_shape, src_chunk_key, lod_delta);
        let dst_chunk = dst_map.get_mut_chunk_or_insert_ambient(dst.dst_chunk_key);

        // HACK: only needs get_mut_chunk because of CompressibleChunkStorage
        if let Some(src_chunk) = src_map.get_mut_chunk(&src_chunk_key) {
            debug_assert_eq!(src_chunk.array.extent().shape, chunk_shape);

            sampler.downsample(
                &src_chunk.array,
                &mut dst_chunk.array,
                dst.dst_offset,
                lod_delta,
            );
        } else {
            let dst_extent = ExtentN::from_min_and_shape(
                dst_chunk.array.extent().minimum + dst.dst_offset.0,
                chunk_shape >> 1,
            );
            dst_chunk
                .array
                .fill_extent(&dst_extent, src_map.ambient_value());
        }
    }
}

/// A `ChunkMap` using `HashMap` as chunk storage.
pub type ChunkHashMapPyramid<N, T, Meta = ()> =
    ChunkPyramid<N, T, Meta, FnvHashMap<PointN<N>, Chunk<N, T, Meta>>>;
/// A 2-dimensional `ChunkHashMapPyramid`.
pub type ChunkHashMapPyramid2<T, Meta = ()> = ChunkHashMapPyramid<[i32; 2], T, Meta>;
/// A 3-dimensional `ChunkHashMapPyramid`.
pub type ChunkHashMapPyramid3<T, Meta = ()> = ChunkHashMapPyramid<[i32; 3], T, Meta>;

impl<N, T> ChunkHashMapPyramid<N, T>
where
    PointN<N>: Hash + IntegerPoint<N>,
{
    pub fn new(builder: ChunkMapBuilder<N, T>, num_levels: u8) -> Self
    where
        ChunkMapBuilder<N, T>: Copy,
    {
        let mut levels = Vec::with_capacity(num_levels as usize);
        levels.resize_with(num_levels as usize, || {
            builder.build_with_write_storage(FnvHashMap::default())
        });

        Self { levels }
    }
}

/// A `ChunkMap` using `CompressibleChunkStorage` as chunk storage.
pub type CompressibleChunkPyramid<N, T, Meta, B> =
    ChunkPyramid<N, T, Meta, CompressibleChunkStorage<N, T, Meta, B>>;

macro_rules! define_conditional_aliases {
    ($backend:ident) => {
        use crate::$backend;

        /// 2-dimensional `CompressibleChunkPyramid`.
        pub type CompressibleChunkPyramid2<T, Meta = (), B = $backend> =
            CompressibleChunkPyramid<[i32; 2], T, Meta, B>;
        /// 3-dimensional `CompressibleChunkPyramid`.
        pub type CompressibleChunkPyramid3<T, Meta = (), B = $backend> =
            CompressibleChunkPyramid<[i32; 3], T, Meta, B>;
    };
}

// LZ4 and Snappy are not mutually exclusive, but if you only use one, then you want to have these aliases refer to the choice
// you made.
#[cfg(all(feature = "lz4", not(feature = "snap")))]
define_conditional_aliases!(Lz4);
#[cfg(all(not(feature = "lz4"), feature = "snap"))]
define_conditional_aliases!(Snappy);

impl<N, T, Meta, B> CompressibleChunkPyramid<N, T, Meta, B>
where
    PointN<N>: Hash + IntegerPoint<N>,
    T: Copy,
    Meta: Clone,
    B: BytesCompression,
{
    pub fn new(builder: ChunkMapBuilder<N, T, Meta>, num_levels: u8, compression: B) -> Self
    where
        B: Copy,
        ChunkMapBuilder<N, T, Meta>: Copy,
    {
        let mut levels = Vec::with_capacity(num_levels as usize);
        levels.resize_with(num_levels as usize, || {
            builder.build_with_write_storage(CompressibleChunkStorage::new(compression))
        });

        Self { levels }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DownsampleDestination<N> {
    pub dst_chunk_key: PointN<N>,
    pub dst_offset: Local<N>,
}

pub type DownsampleDestination2 = DownsampleDestination<[i32; 2]>;
pub type DownsampleDestination3 = DownsampleDestination<[i32; 3]>;

impl<N> DownsampleDestination<N>
where
    PointN<N>: IntegerPoint<N>,
{
    /// When downsampling a chunk at level `N`, the samples are used at the returned destination within level `N + level_delta`
    /// in the clipmap.
    pub fn for_source_chunk(
        chunk_shape: PointN<N>,
        src_chunk_key: PointN<N>,
        lod_delta: u8,
    ) -> Self {
        let lod_delta = lod_delta as i32;
        let chunk_shape_log2 = chunk_shape.map_components_unary(|c| c.trailing_zeros() as i32);
        let level_up_log2 = chunk_shape_log2 + PointN::ONES * lod_delta;
        let level_up_shape = chunk_shape << lod_delta;
        let dst_chunk_key = (src_chunk_key >> level_up_log2) << chunk_shape_log2;
        let offset = src_chunk_key % level_up_shape;
        let dst_offset = Local(offset >> lod_delta);

        Self {
            dst_chunk_key,
            dst_offset,
        }
    }
}

// ████████╗███████╗███████╗████████╗
// ╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝
//    ██║   █████╗  ███████╗   ██║
//    ██║   ██╔══╝  ╚════██║   ██║
//    ██║   ███████╗███████║   ██║
//    ╚═╝   ╚══════╝╚══════╝   ╚═╝

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_destination_for_one_level_up() {
        let chunk_shape = PointN([16; 3]);
        let level_delta = 1;

        let src_key = chunk_shape;
        let dst = DownsampleDestination3::for_source_chunk(chunk_shape, src_key, level_delta);
        assert_eq!(
            dst,
            DownsampleDestination3 {
                dst_chunk_key: PointN([0; 3]),
                dst_offset: Local(chunk_shape / 2),
            }
        );

        let src_key = 2 * chunk_shape;
        let dst = DownsampleDestination3::for_source_chunk(chunk_shape, src_key, level_delta);
        assert_eq!(
            dst,
            DownsampleDestination3 {
                dst_chunk_key: chunk_shape,
                dst_offset: Local(Point3i::ZERO),
            }
        );
    }

    #[test]
    fn downsample_destination_for_two_levels_up() {
        let chunk_shape = PointN([16; 3]);
        let level_delta = 2;

        let src_key = 3 * chunk_shape;
        let dst = DownsampleDestination3::for_source_chunk(chunk_shape, src_key, level_delta);
        assert_eq!(
            dst,
            DownsampleDestination3 {
                dst_chunk_key: PointN([0; 3]),
                dst_offset: Local(3 * chunk_shape / 4),
            }
        );

        let src_key = 4 * chunk_shape;
        let dst = DownsampleDestination3::for_source_chunk(chunk_shape, src_key, level_delta);
        assert_eq!(
            dst,
            DownsampleDestination3 {
                dst_chunk_key: chunk_shape,
                dst_offset: Local(Point3i::ZERO),
            }
        );
    }
}