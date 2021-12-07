use wgpu::*;
use wgpu::util::DeviceExt;

pub fn fast_buffer<T: bytemuck::Pod>(device: &Device, data: &[T], usage: BufferUsages) -> Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Buffer", usage)),
            contents: bytemuck::cast_slice(data),
            usage: usage,
        }
    );
}