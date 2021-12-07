#![allow(dead_code, unused_variables)]
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use noise::NoiseFn;
use wgpu::*;
use chunk::HeightChunk;
use cgmath::InnerSpace;
use winit_input_helper::WinitInputHelper;

use std::time::{SystemTime};
mod setup;
use setup::Ctx;

mod util;
use util::fast_buffer;

mod camera;
use camera::{CameraController, Camera};

mod texture;
use texture::Texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3]
}
impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            },
            VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                shader_location: 1,
                format: VertexFormat::Float32x3,
            }
            ]
        }
    }
}

const VERTICES: &[Vertex] = &[
Vertex { position: [0.5, 0.5, 0.0], color: [1.,1.,1.] }, // 0
Vertex { position: [0.5, -0.5, 0.0], color: [0.,0.,0.] }, // 1
Vertex { position: [-0.5, -0.5, 0.0], color: [0.,0.,0.] }, // 2
Vertex { position: [-0.5, 0.5, 0.0], color: [1.,1.,1.] }, // 3

Vertex { position: [0.5, 0.5, -1.], color: [1.,1.,1.] }, // 4
Vertex { position: [0.5, -0.5, -1.], color: [0.,0.,0.] }, // 5
Vertex { position: [-0.5, -0.5, -1.], color: [0.,0.,0.] }, // 6
Vertex { position: [-0.5, 0.5, -1.], color: [1.,1.,1.] }, // 7
];

const INDICES: &[u16] = &[
0,1,2,
0,2,3,

4,5,1,
4,1,0,

4,5,6,
4,6,7,

7,6,2,
7,2,3,

4,0,3,
4,3,7,

5,1,2,
5,2,6
];

struct Model {
    vert_buffer: Buffer,
    verts: u32,
    indx_buffer: Buffer,
    indxs: u32
}

struct State {
    ctx: Ctx,
    clear_color: Color,
    depth_texture: Texture,
    
    height: HeightChunk,
    height_changed: bool,
    
    block_render_pipe: RenderPipeline,
    block_model: Model,
    blocks_buffer: Buffer,
    
    time_buffer: Buffer,
    time_bind_group: BindGroup,
    secs: f64,
    
    cam: Camera,
    cam_control: CameraController,
    cam_buffer: Buffer,
    cam_bind_group: BindGroup,
    
    switch: bool
}
impl State {
    async fn new(window: &Window) -> State {
        let ctx = Ctx::new(window).await;
        
        // Time buffer
        let time_bind_group_layout = ctx.device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
                entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                }
                ],
                label: Some("bind group layout")
            }
        );
        let time_buffer = fast_buffer(&ctx.device, &[0. as f32, 0. as f32], BufferUsages::UNIFORM | BufferUsages::COPY_DST);
        let time_bind_group = ctx.device.create_bind_group(
            &BindGroupDescriptor {
                layout: &time_bind_group_layout,
                entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: time_buffer.as_entire_binding()
                },
                BindGroupEntry {
                    binding: 1,
                    resource: time_buffer.as_entire_binding()
                }
                ],
                label: None
            }
        );
        
        // Camera buffer
        let cam = Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (10.,30.,10.).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: ctx.config.width as f32 / ctx.config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,
        };
        
        let cam_buffer = fast_buffer(&ctx.device, &[cam.uniform()], BufferUsages::UNIFORM | BufferUsages::COPY_DST);
        
        let cam_bind_group_layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
            ],
            label: Some("camera_bind_group_layout"),
        });
        
        let cam_bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &cam_bind_group_layout,
            entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: cam_buffer.as_entire_binding(),
            }
            ],
            label: Some("camera_bind_group"),
        });
        
        // Positions
        
        let ns = noise::SuperSimplex::new();
        let height = HeightChunk::for_each(|x,y,z| {
            let v = ns.get([x as f64 / 50.,z as f64 / 50.]) as f32 * 10. + 5.;
            return (y as f32) < v;
        });
        let pos_buffer = fast_buffer(&ctx.device, &height.positions(), BufferUsages::VERTEX | BufferUsages::COPY_DST);
        
        let pos_desc = VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 3]>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &[
            VertexAttribute {
                offset: 0,
                shader_location: 2,
                format: VertexFormat::Float32x3
            }
            ]
        };
        
        // Shader
        let path = "src/shader.wgsl";
        let shader = ctx.device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        
        // Layout
        let render_pipeline_layout =
        ctx.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
            &time_bind_group_layout,
            &cam_bind_group_layout
            ],
            push_constant_ranges: &[],
        });
        
        let depth_texture = Texture::create_depth_texture(&ctx.device, &ctx.config, "depth texture");
        
        // Pipeline
        let render_pipeline = ctx.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                Vertex::desc(),
                pos_desc
                ],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[ColorTargetState {
                    format: ctx.config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                ..PrimitiveState::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default()
            }),
            multisample: MultisampleState::default()
        });
        
        // Vertex buffer stuff
        let vert_buffer = fast_buffer(&ctx.device, VERTICES, BufferUsages::VERTEX);
        let indx_buffer = fast_buffer(&ctx.device, INDICES, BufferUsages::INDEX);
        
        Self {
            ctx,
            clear_color: Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            depth_texture,
            height,
            height_changed: false,
            
            block_render_pipe: render_pipeline,
            block_model: Model {
                vert_buffer,
                verts: VERTICES.len() as u32,
                indx_buffer,
                indxs: INDICES.len() as u32
            },
            blocks_buffer: pos_buffer,
            
            time_buffer,
            time_bind_group,
            secs: 0.,
            
            cam,
            cam_control: CameraController::new(),
            cam_buffer,
            cam_bind_group,
            
            switch: false
        }
    }
    
    fn update(&mut self, delta: f64, window: &Window, input: &WinitInputHelper, mouse_motion: (f64, f64)) {
        let rel_mouse_motion = (
            (mouse_motion.0 / window.inner_size().width as f64) as f32,
            (mouse_motion.1 / window.inner_size().width as f64) as f32
        );

        self.cam_control.update(
            &mut self.cam, 
            &self.height, 
            input,
            rel_mouse_motion,
            delta as f32
        );
        
        self.ctx.queue.write_buffer(&self.cam_buffer, 0, bytemuck::cast_slice(&[self.cam.uniform()]));

        type K = VirtualKeyCode;
        if input.key_pressed(K::E) {
            let target = self.height.ray(self.cam.eye, (self.cam.target - self.cam.eye).normalize() * 5.);

            match target {
                Some((ix, b)) => {
                    *self.height.get_mut(ix).unwrap() = false;
                    self.height_changed = true;
                },
                None => {}
            };
        }

        self.secs += delta;
        self.ctx.queue.write_buffer(&self.time_buffer, 0, bytemuck::cast_slice(&[self.secs as f32,self.secs as f32]));
        
        if self.height_changed {
            self.ctx.queue.write_buffer(&self.blocks_buffer, 0, bytemuck::cast_slice(&self.height.positions()));
        }
    }
    
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.ctx.resize(new_size);
        self.depth_texture = texture::Texture::create_depth_texture(&self.ctx.device, &self.ctx.config, "depth_texture");
    }
    
    fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.ctx.surface.get_current_texture()?;
        let view = output.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.ctx.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            rpass.set_pipeline(&self.block_render_pipe);
            
            rpass.set_vertex_buffer(0, self.block_model.vert_buffer.slice(..));
            rpass.set_vertex_buffer(1, self.blocks_buffer.slice(..));
            
            rpass.set_index_buffer(self.block_model.indx_buffer.slice(..), IndexFormat::Uint16);
            
            rpass.set_bind_group(0, &self.time_bind_group, &[]);
            rpass.set_bind_group(1, &self.cam_bind_group, &[]);
            
            rpass.draw_indexed(
                0..self.block_model.indxs, 
                0, 
                0..self.height.positions().len() as u32);
            }
            
            self.ctx.queue.submit(std::iter::once(encoder.finish()));
            output.present();
            
            Ok(())
        }
    }
    
    fn main() {
        env_logger::init();
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
        .with_title("Copyright Friedrich Hohensee")
        .build(&event_loop).unwrap();
        window.set_cursor_position(winit::dpi::LogicalPosition::new(window.inner_size().width / 4, window.inner_size().height / 4)).unwrap();
        window.set_cursor_visible(true);
        window.set_cursor_grab(true).unwrap();
        
        let mut state = pollster::block_on(State::new(&window));
        let mut input = WinitInputHelper::new();
        
        let mut prev = SystemTime::now();
        let mut mouse_motion: (f64, f64) = (0.,0.);
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                    mouse_motion.0 += delta.0;
                    mouse_motion.1 += delta.1;
                }
                
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == window.id() => {
                    match event {
                        // Handle quitting
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        
                        // Handle resizing
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so we have to dereference it twice
                            state.resize(**new_inner_size);
                        },
                        
                        _ => {}
                    }
                },
                Event::RedrawRequested(_) => {
                    
                    match state.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if lost
                        Err(SurfaceError::Lost) => state.ctx.resize(state.ctx.size),
                        // The system is out of memory, we should probably quit
                        Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        // All other errors (Outdated, Timeout) should be resolved by the next frame
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                Event::MainEventsCleared => {
                    // RedrawRequested will only trigger once, unless we manually
                    // request it.
                    window.request_redraw();
                },
                _ => {}
            };

            if input.update(&event) {
                let duration = SystemTime::now().duration_since(prev).unwrap();
                let delta = duration.as_secs() as f64
                    + duration.subsec_nanos() as f64 * 1e-9;
                state.update(
                    delta, &window, &input, mouse_motion
                );
                prev = SystemTime::now();

                mouse_motion = (0.,0.);
            }
        });
    }