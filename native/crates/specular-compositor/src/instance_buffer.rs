//! A growable per-instance vertex buffer.

use bytemuck::Pod;

/// Instance capacity of a new buffer; it doubles on demand.
const INITIAL_CAPACITY: usize = 64;

/// One vertex buffer of `T` instances, rewritten every frame.
#[derive(Debug)]
pub(crate) struct InstanceBuffer<T> {
    label: &'static str,
    buffer: wgpu::Buffer,
    capacity: usize,
    marker: std::marker::PhantomData<T>,
}

impl<T: Pod> InstanceBuffer<T> {
    pub(crate) fn new(device: &wgpu::Device, label: &'static str) -> Self {
        Self {
            label,
            buffer: create_buffer::<T>(device, label, INITIAL_CAPACITY),
            capacity: INITIAL_CAPACITY,
            marker: std::marker::PhantomData,
        }
    }

    /// Uploads `instances` from the start of the buffer, growing it first if
    /// they do not fit.
    pub(crate) fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, instances: &[T]) {
        if instances.len() > self.capacity {
            let capacity = instances.len().next_power_of_two();
            self.buffer = create_buffer::<T>(device, self.label, capacity);
            self.capacity = capacity;
        }
        if !instances.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(instances));
        }
    }

    pub(crate) fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }
}

fn create_buffer<T>(device: &wgpu::Device, label: &'static str, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (capacity * size_of::<T>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
