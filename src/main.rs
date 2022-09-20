use std::mem::size_of;
use pollster::block_on;
use wgpu::*;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

struct RenderContext {
    instance: Instance,
    device: Device,
    queue: Queue,

    window: Window,
    surface: Surface,
    format: TextureFormat,
}

impl RenderContext {
    async fn new(event_loop: &EventLoop<()>) -> Self {
        let window = Window::new(&event_loop).expect("failed to create window");
        let instance = Instance::new(Backends::DX12);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }).await.expect("failed to request adapter");
        let format = surface.get_supported_formats(&adapter)[0];

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor::default(),
            None,
        ).await.expect("failed to request device");

        let size = window.inner_size();
        surface.configure(&device, &SurfaceConfiguration {
            format,
            width: size.width,
            height: size.height,
            usage: TextureUsages::RENDER_ATTACHMENT,
            present_mode: PresentMode::AutoVsync,
        });
        Self {
            instance,
            device,
            queue,

            window,
            surface,
            format,
        }
    }

    fn resize(&self, width: u32, height: u32) {
        self.surface.configure(&self.device, &SurfaceConfiguration {
            format: self.format,
            width,
            height,
            usage: TextureUsages::RENDER_ATTACHMENT,
            present_mode: PresentMode::Fifo,
        });
        // required for MacOS
        self.window.request_redraw();
    }
}

#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct Vertex {
    pos: [f32; 2],
}

const VERTEX_SIZE: BufferAddress = size_of::<Vertex>() as BufferAddress;

struct Renderer {
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
}

impl Renderer {
    fn new(context: &RenderContext) -> Self {
        let shader_module = context.device.create_shader_module(include_wgsl!("shader.wgsl"));

        let pipeline_layout = context.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = context.device.create_render_pipeline(
            &RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    entry_point: "vertex",
                    module: &shader_module,
                    buffers: &[
                        VertexBufferLayout {
                            array_stride: VERTEX_SIZE,
                            step_mode: VertexStepMode::Vertex,
                            attributes: &vertex_attr_array![
                                0 => Float32x2,
                            ],
                        },
                    ],
                },
                fragment: Some(FragmentState {
                    entry_point: "fragment",
                    module: &shader_module,
                    targets: &[
                        Some(context.format.into())
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            }
        );

        let vertex_buffer = context.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&[
                Vertex { pos: [-1.0, -1.0] },
                Vertex { pos: [0.0, 1.0] },
                Vertex { pos: [1.0, -1.0] },
            ]),
        });

        Self {
            render_pipeline,
            vertex_buffer,
        }
    }

    async fn draw(&self, context: &RenderContext) -> Option<Error> {
        println!("draw");
        context.device.push_error_scope(ErrorFilter::Validation);

        let surface_texture = context.surface.get_current_texture().expect("couldn't get next surface texture");
        let surface_view = surface_texture.texture.create_view(&TextureViewDescriptor::default());

        let mut cmd = context.device.create_command_encoder(&CommandEncoderDescriptor::default());
        let mut render_cmd = cmd.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    ops: Operations {
                        load: LoadOp::Clear(Color::RED),
                        store: true,
                    },
                    view: &surface_view,
                    resolve_target: None,
                })
            ],
            depth_stencil_attachment: None,
        });
        render_cmd.set_pipeline(&self.render_pipeline);
        render_cmd.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_cmd.draw(0..3, 0..1);
        drop(render_cmd);
        let cmd = cmd.finish();
        context.queue.submit([cmd]);
        surface_texture.present();

        context.device.pop_error_scope().await
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let context = block_on(RenderContext::new(&event_loop));
    context.device.push_error_scope(ErrorFilter::Validation);
    let renderer = Renderer::new(&context);
    if let Some(error) = block_on(context.device.pop_error_scope()) {
        panic!("failed to create renderer: {error}");
    }

    event_loop.run(move |event, _event_loop, flow| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::Resized(size) => {
                        context.resize(size.width, size.height);
                    }
                    WindowEvent::CloseRequested => {
                        *flow = ControlFlow::ExitWithCode(0);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(..) => {
                if let Some(error) = block_on(renderer.draw(&context)) {
                    eprintln!("draw: {error}");
                }
            }
            _ => {}
        }
    });
}
