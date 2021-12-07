use winit::{
    event::*
};
use cgmath::Vector3;
use chunk::HeightChunk;
use winit_input_helper::WinitInputHelper;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
    pub fn uniform(&self) -> CameraUniform {
        CameraUniform {
            view_proj: self.build_view_projection_matrix().into()
        }
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}


pub struct CameraController {
    xrot: f32,
    yrot: f32,
    yvel: f32,
}

const EYE_OFFSET: Vector3<f32> = Vector3::new(0., -4., 0.);
const SC: f32 = 1.;

type K = VirtualKeyCode;

impl CameraController {
    pub fn new() -> Self {
        Self {
            xrot: 0.,
            yrot: 0.,
            yvel: 0.,
        }
    }

    pub fn update(&mut self, camera: &mut Camera, chunk: &HeightChunk, input: &WinitInputHelper, mouse_motion: (f32, f32), delta: f32) {
        const PI: f32 = 3.14159;

        let ef_fr = 10. * delta;
        let ef_si = 10. * delta;

        self.xrot += mouse_motion.0 * 10.;
        self.yrot += -mouse_motion.1 * 10.;

        if self.xrot > PI { self.xrot = -PI }
        if self.xrot < -PI { self.xrot = PI }

        self.yrot = self.yrot.clamp(-PI / 2., PI / 2.);

        // Update camera
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let move_norm = Vector3::<f32>::new(forward_norm.x, 0., forward_norm.z);
        let forward_mag = forward.magnitude();

        let feet = camera.eye + Vector3::new(0.,-1.,0.);

        if input.key_held(K::W) && chunk.is_empty(feet + move_norm * ef_fr) {
            camera.eye += move_norm * ef_fr;
        }
        if input.key_held(K::S) && chunk.is_empty(feet - move_norm * ef_fr) {
            camera.eye -= move_norm * ef_fr;
        }
        let right = move_norm.cross(camera.up);
        if input.key_held(K::D) && chunk.is_empty(feet + right * ef_si) {
            camera.eye += right * ef_si;
        }
        if input.key_held(K::A) && chunk.is_empty(feet - right * ef_si) {
            camera.eye -= right * ef_si;
        }
        // if input.key_held(K::Space) {
        //     camera.eye += camera.up * ef_up;
        // }
        // if input.key_held(K::LShift) {
        //     camera.eye -= camera.up * ef_up;
        // }
        if ! chunk.is_empty(camera.eye + Vector3::new(0.,-2.,0.)) {
            self.yvel = self.yvel.max(0.);
        } else {
            self.yvel -= 0.05;
            if self.yvel < -1. { self.yvel = -1. }
        }

        if input.key_pressed(K::Space) {
            self.yvel += 0.5;
        }
        camera.eye += camera.up * self.yvel;

        camera.target = camera.eye + Vector3::new(self.xrot.cos(), self.yrot.sin(), self.xrot.sin()).normalize() * forward_mag;
    }
}
