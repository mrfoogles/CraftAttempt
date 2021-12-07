use wgpu::*;
use wgpu::util::DeviceExt;
use winit::event::*;

mod camera;
use camera::*;

pub mod util;
use util::{fast_buffer};

pub trait Plugin {
    fn update(&mut self, delta: f64, queue: &Queue);
    fn input(&mut self, event: &WindowEvent, queue: &Queue) -> bool;
}

pub struct CameraPlugin {
    camera: Camera,
    controller: CameraController,
    buffer: Buffer
}
impl CameraPlugin {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> (CameraPlugin, BindGroup, BindGroupLayout) {
        

        return (CameraPlugin {
            camera,
            controller: CameraController::new(2.),
            buffer
        },bind_group, bind_group_layout)
    }
}
impl Plugin for CameraPlugin {
    fn update(&mut self, delta: f64, queue: &Queue) {
        self.controller.update_camera(&mut self.camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.camera.uniform()]));
    }
    fn input(&mut self, event: &WindowEvent, queue: &Queue) -> bool {
        return self.controller.process_events(event)
    }
}

impl Plugin for TimePlugin {
    fn update(&mut self, delta: f64, queue: &Queue) {
        self.secs += delta;
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.secs as f32,self.secs as f32]));
    }
    fn input(&mut self, event: &WindowEvent, queue: &Queue) -> bool {
        false
    }
}