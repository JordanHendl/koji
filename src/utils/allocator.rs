// src/gpu_allocator.rs

use dashi::utils::*;
use dashi::*;

#[derive(Debug, Clone, Copy)]
pub struct Allocation {
    pub buffer: Handle<Buffer>,
    pub offset: u64,
    pub size: u64,
}

pub struct GpuAllocator {
    ctx: *mut Context,
    pub buffer: Handle<Buffer>,
    pub capacity: u64,
    pub alignment: u64,
    pub current_offset: u64,
    pub free_list: Vec<(u64, u64)>,
}

impl Default for GpuAllocator {
    fn default() -> Self {
        Self {
            ctx: std::ptr::null_mut(),
            buffer: Default::default(),
            capacity: Default::default(),
            alignment: Default::default(),
            current_offset: Default::default(),
            free_list: Default::default(),
        }
    }
}
impl GpuAllocator {
    pub fn new(
        ctx: &mut Context,
        byte_size: u64,
        usage: BufferUsage,
        alignment: u64,
    ) -> Result<Self, GPUError> {
        let buffer = ctx.make_buffer(&BufferInfo {
            debug_name: "GpuAllocator_Buffer",
            byte_size: byte_size as u32,
            visibility: MemoryVisibility::CpuAndGpu,
            usage,
            initial_data: None,
        })?;

        Ok(Self {
            buffer,
            capacity: byte_size,
            alignment,
            current_offset: 0,
            free_list: Vec::new(),
            ctx,
        })
    }

    pub fn free(&mut self, allocation: Allocation) {
        self.free_list.push((allocation.offset, allocation.size));
        self.free_list.sort_by_key(|(offset, _)| *offset);

        let mut merged = Vec::new();
        let mut iter = self.free_list.iter().copied();

        if let Some(mut current) = iter.next() {
            for next in iter {
                if current.0 + current.1 == next.0 {
                    // adjacent blocks, merge
                    current.1 += next.1;
                } else {
                    merged.push(current);
                    current = next;
                }
            }
            merged.push(current);
        }

        self.free_list = merged;
    }

    pub fn allocate(&mut self, size: u64) -> Option<Allocation> {
        // First, try from free list
        let aligned_size = Self::align_up(size, self.alignment);
        if let Some((index, (_offset, free_size))) = self
            .free_list
            .iter()
            .enumerate()
            .find(|(_, (_offset, free_size))| *free_size >= aligned_size)
        {
            let alloc = Allocation {
                buffer: self.buffer,
                offset: *_offset,
                size: aligned_size,
            };
            if *free_size > aligned_size {
                self.free_list[index] = (_offset + aligned_size, free_size - aligned_size);
            } else {
                self.free_list.swap_remove(index);
            }
            return Some(alloc);
        }
        let aligned_offset = Self::align_up(self.current_offset, self.alignment);
        let end = aligned_offset + aligned_size;

        if end > self.capacity {
            return None;
        }

        let alloc = Allocation {
            buffer: unsafe { &mut *(self.ctx) }
                .suballoc_from(self.buffer, aligned_offset as u32, aligned_size as u32)?,
            offset: aligned_offset,
            size: aligned_size,
        };

        self.current_offset = end;
        Some(alloc)
    }
    
    pub fn destroy(self, ctx: &mut Context) {
        ctx.destroy_buffer(self.buffer);
    }
    pub fn reset(&mut self) {
        self.current_offset = 0;
        self.free_list.clear();
    }

    fn align_up(offset: u64, alignment: u64) -> u64 {
        (offset + alignment - 1) & !(alignment - 1)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use dashi::gpu;
    use serial_test::serial;

    fn init_ctx() -> gpu::Context {
        gpu::Context::headless(&Default::default()).unwrap()
    }

    #[test]
    #[serial]
    fn basic_allocation_and_free() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 64).unwrap();

        let a = alloc.allocate(128).expect("Failed to allocate A");
        let b = alloc.allocate(128).expect("Failed to allocate B");

        assert_ne!(a.offset, b.offset);

        alloc.free(a);
        alloc.free(b);

        assert_eq!(alloc.free_list.len(), 1); // should be merged
        assert_eq!(alloc.free_list[0], (0, 256));
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn reallocation_from_free_list() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 64).unwrap();

        let a = alloc.allocate(128).unwrap();
        let b = alloc.allocate(128).unwrap();
        let ao = a.offset;
        alloc.free(a);

        let c = alloc.allocate(64).unwrap();
        assert_eq!(c.offset, ao);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn allocation_over_capacity_fails() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 256, BufferUsage::STORAGE, 64).unwrap();

        alloc.allocate(128).unwrap();
        alloc.allocate(128).unwrap();
        let result = alloc.allocate(64);

        assert!(result.is_none());
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn merge_adjacent_blocks() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 64).unwrap();

        let a = alloc.allocate(128).unwrap();
        let b = alloc.allocate(128).unwrap();
        let c = alloc.allocate(128).unwrap();

        alloc.free(b);
        alloc.free(a);
        alloc.free(c);

        assert_eq!(alloc.free_list.len(), 1);
        assert_eq!(alloc.free_list[0], (0, 384));
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn reset_allocator() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 64).unwrap();

        let _ = alloc.allocate(128).unwrap();
        alloc.reset();

        assert_eq!(alloc.current_offset, 0);
        assert!(alloc.free_list.is_empty());
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn test_alignment_handling() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 64).unwrap();
        let a = alloc.allocate(1).unwrap();
        assert_eq!(a.offset % 64, 0);
        assert_eq!(a.size, 64);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn test_out_of_memory() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 128, BufferUsage::STORAGE, 64).unwrap();
        assert!(alloc.allocate(64).is_some());
        assert!(alloc.allocate(64).is_some());
        assert!(alloc.allocate(1).is_none()); // should not fit
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn test_large_allocation() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 4096, BufferUsage::STORAGE, 256).unwrap();
        let a1 = alloc.allocate(2048).unwrap();
        let a2 = alloc.allocate(2048).unwrap();
        assert!(alloc.allocate(1).is_none());
        assert_ne!(a1.offset, a2.offset);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn test_free_and_merge_multiple() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 32).unwrap();

        let a1 = alloc.allocate(128).unwrap();
        let a2 = alloc.allocate(128).unwrap();
        let a3 = alloc.allocate(128).unwrap();

        alloc.free(a1.clone());
        alloc.free(a3.clone());
        alloc.free(a2.clone()); // now they are out of order, test merge

        assert_eq!(alloc.free_list.len(), 1);
        let (offset, size) = alloc.free_list[0];
        assert_eq!(offset, 0);
        assert_eq!(size, 384);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn test_reset_allocator() {
        let mut ctx = init_ctx();
        let mut alloc = GpuAllocator::new(&mut ctx, 1024, BufferUsage::STORAGE, 64).unwrap();
        let _ = alloc.allocate(128).unwrap();
        let _ = alloc.allocate(128).unwrap();
        alloc.reset();
        assert_eq!(alloc.current_offset, 0);
        assert!(alloc.free_list.is_empty());
        assert!(alloc.allocate(1024).is_some());
        ctx.destroy();
    }
}
