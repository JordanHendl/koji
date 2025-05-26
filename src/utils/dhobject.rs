// File: src/dh_object.rs

use dashi::utils::Handle;
use dashi::{Buffer, BufferInfo, BufferUsage, Context, MemoryVisibility};
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use std::ptr::NonNull;

use super::allocator::{Allocation, GpuAllocator};


#[derive(Clone, Copy)]
pub struct DHObject<T> {
    pub allocation: Allocation,
    ctx: *mut Context,
    ptr: NonNull<T>,
    _phantom: PhantomData<T>,
}

impl<T> DHObject<T> {
    pub fn new(
        ctx: &mut Context,
        allocator: &mut GpuAllocator,
        value: T,
    ) -> Result<Self, dashi::GPUError> {
        let alloc = allocator
            .allocate(std::mem::size_of::<T>() as u64)
            .ok_or("GPU allocation failed")
            .unwrap();

        let raw = ctx.map_buffer_mut::<u8>(alloc.buffer)?.as_mut_ptr();
        let ptr_offset = unsafe { raw.add(alloc.offset as usize) as *mut T };
        let ptr = NonNull::new(ptr_offset)
            .ok_or("Null pointer in mapped memory")
            .expect("Non null ptr!");

        unsafe {
            ptr.as_ptr().write(value);
        }

        Ok(Self {
            allocation: alloc,
            ptr,
            _phantom: PhantomData,
            ctx,
        })
    }

    pub fn get(&self) -> &T {
        return unsafe { self.ptr.as_ref() };
    }

    pub fn write(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    pub fn buffer(&self) -> Handle<Buffer> {
        self.allocation.buffer
    }

    pub fn offset(&self) -> u64 {
        self.allocation.offset
    }
}

impl<T> Drop for DHObject<T> {
    fn drop(&mut self) {
        unsafe { &(*self.ctx).unmap_buffer(self.buffer()) };
    }
}

#[cfg(test)]
mod test {
    use dashi::*;
    use serial_test::serial;
    use utils::*;

    use crate::utils::{allocator::GpuAllocator, dhobject::DHObject};
    #[repr(C)]
    #[derive(Debug, Default, Clone, Copy, PartialEq)]
    struct MyData {
        x: f32,
        y: f32,
    }

    #[test]
    #[serial]
    fn test_dhobject_allocation_and_write() {
        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = Context::new(&ContextInfo { device }).unwrap();

        let mut allocator = GpuAllocator::new(&mut ctx, 1024, BufferUsage::UNIFORM, 16).unwrap();

        let data = MyData { x: 3.14, y: 1.23 };
        let mut dh = DHObject::new(&mut ctx, &mut allocator, data).unwrap();

        assert_eq!(dh.write().x, 3.14);
        assert_eq!(dh.write().y, 1.23);

        // Mutate the value
        dh.write().x = 2.71;
        dh.write().y = 0.99;

        assert_eq!(dh.write().x, 2.71);
        assert_eq!(dh.write().y, 0.99);

        drop(dh);
        ctx.destroy();
    }

    #[test]
    #[serial]
    fn multiple_dh_objects_and_drop() {
        let device = DeviceSelector::new()
            .unwrap()
            .select(DeviceFilter::default().add_required_type(DeviceType::Dedicated))
            .unwrap_or_default();
        let mut ctx = Context::new(&ContextInfo { device }).unwrap();

        let mut allocator = GpuAllocator::new(&mut ctx, 1024, BufferUsage::UNIFORM, 16).unwrap();

        let dh1 = DHObject::new(&mut ctx, &mut allocator, 42u32).unwrap();
        let dh2 = DHObject::new(&mut ctx, &mut allocator, 7u32).unwrap();
        let dh3 = DHObject::new(&mut ctx, &mut allocator, 123u32).unwrap();

        assert_eq!(*dh1.get(), 42);
        assert_eq!(*dh2.get(), 7);
        assert_eq!(*dh3.get(), 123);

        // Drop all DHObjects explicitly
        drop(dh1);
        drop(dh2);
        drop(dh3);

        // Context will be destroyed after this test
        ctx.destroy();
    }
}
