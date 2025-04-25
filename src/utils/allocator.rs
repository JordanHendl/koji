// src/gpu_allocator.rs

use dashi::utils::*;
use dashi::*;

#[derive(Debug)]
pub struct Allocation {
    pub buffer: Handle<Buffer>,
    pub offset: u64,
    pub size: u64,
}

pub struct GpuAllocator {
    pub buffer: Handle<Buffer>,
    pub capacity: u64,
    pub alignment: u64,
    pub current_offset: u64,
    pub free_list: Vec<(u64, u64)>,
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
        })
    }

    pub fn free(&mut self, allocation: Allocation) {
        self.free_list.push((allocation.offset, allocation.size));
    }

    pub fn allocate(&mut self, size: u64) -> Option<Allocation> {
        // First, try from free list
        let aligned_size = Self::align_up(size, self.alignment);
        if let Some((index, (offset, free_size))) = self
            .free_list
            .iter()
            .enumerate()
            .find(|(_, (offset, free_size))| *free_size >= aligned_size)
        {
            let alloc = Allocation {
                buffer: self.buffer,
                offset: *offset,
                size: aligned_size,
            };
            if *free_size > aligned_size {
                self.free_list[index] = (offset + aligned_size, free_size - aligned_size);
            } else {
                self.free_list.swap_remove(index);
            }
            return Some(alloc);
        }
        let aligned_offset = Self::align_up(self.current_offset, self.alignment);
        let end = aligned_offset + size;

        if end > self.capacity {
            return None;
        }

        let alloc = Allocation {
            buffer: self.buffer,
            offset: aligned_offset,
            size,
        };

        self.current_offset = end;
        Some(alloc)
    }

    pub fn reset(&mut self) {
        self.current_offset = 0;
        self.free_list.clear();
    }

    fn align_up(offset: u64, alignment: u64) -> u64 {
        (offset + alignment - 1) & !(alignment - 1)
    }
}
