use bytemuck::{Pod, Zeroable};
use std::io::Cursor;
use std::time::Instant;
use wgpu::*;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

#[repr(C)]
#[derive(Copy, Clone)]
struct TimeUniform {
    time: f32,
}
unsafe impl Zeroable for TimeUniform {}
unsafe impl Pod for TimeUniform {}

#[repr(C)]
#[derive(Copy, Clone)]
struct ScreenSizeUniform {
    screen_size: [f32; 2],
}
unsafe impl Zeroable for ScreenSizeUniform {}
unsafe impl Pod for ScreenSizeUniform {}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    pollster::block_on(run(event_loop, window));
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let surface = Surface::create(&window);
    let adapter = Adapter::request(
        &RequestAdapterOptions {
            power_preference: PowerPreference::Default,
            compatible_surface: Some(&surface),
        },
        BackendBit::PRIMARY,
    )
    .await
    .unwrap();
    let (device, queue) = adapter
        .request_device(&DeviceDescriptor {
            extensions: Extensions {
                anisotropic_filtering: false,
            },
            limits: Limits::default(),
        })
        .await;
    let mut screen_size = window.inner_size();

    let mut swap_chain_descriptor = SwapChainDescriptor {
        usage: TextureUsage::OUTPUT_ATTACHMENT,
        format: TextureFormat::Bgra8Unorm,
        width: screen_size.width,
        height: screen_size.height,
        present_mode: PresentMode::Mailbox,
    };
    let mut swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

    let mut msaa_texture_descriptor = TextureDescriptor {
        size: Extent3d {
            width: screen_size.width,
            height: screen_size.height,
            depth: 1,
        },
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: 16,
        dimension: TextureDimension::D2,
        format: swap_chain_descriptor.format,
        usage: TextureUsage::OUTPUT_ATTACHMENT,
        label: None,
    };
    let mut msaa_texture = device
        .create_texture(&msaa_texture_descriptor)
        .create_default_view();

    let vertex_shader = include_bytes!("../metaballs_vert.spv");
    let vertex_shader = read_spirv(Cursor::new(&vertex_shader[..])).unwrap();
    let vertex_shader = device.create_shader_module(&vertex_shader);

    let fragment_shader = include_bytes!("../metaballs_frag.spv");
    let fragment_shader = read_spirv(Cursor::new(&fragment_shader[..])).unwrap();
    let fragment_shader = device.create_shader_module(&fragment_shader);

    let time = Instant::now();
    let mut time_uniform = TimeUniform {
        time: time.elapsed().as_secs_f32(),
    };
    let time_uniform_buffer = device.create_buffer_with_data(
        bytemuck::cast_slice(&[time_uniform]),
        BufferUsage::UNIFORM | BufferUsage::COPY_DST,
    );

    let mut screen_size_uniform = ScreenSizeUniform {
        screen_size: [screen_size.width as f32, screen_size.height as f32],
    };
    let screen_size_uniform_buffer = device.create_buffer_with_data(
        bytemuck::cast_slice(&[screen_size_uniform]),
        BufferUsage::UNIFORM | BufferUsage::COPY_DST,
    );

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        bindings: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::UniformBuffer { dynamic: false },
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::UniformBuffer { dynamic: false },
            },
        ],
        label: None,
    });
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        layout: &bind_group_layout,
        bindings: &[
            Binding {
                binding: 0,
                resource: BindingResource::Buffer {
                    buffer: &time_uniform_buffer,
                    range: 0..std::mem::size_of_val(&time_uniform) as BufferAddress,
                },
            },
            Binding {
                binding: 1,
                resource: BindingResource::Buffer {
                    buffer: &screen_size_uniform_buffer,
                    range: 0..std::mem::size_of_val(&screen_size_uniform) as BufferAddress,
                },
            },
        ],
        label: None,
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: ProgrammableStageDescriptor {
            module: &vertex_shader,
            entry_point: "main",
        },
        fragment_stage: Some(ProgrammableStageDescriptor {
            module: &fragment_shader,
            entry_point: "main",
        }),
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: PrimitiveTopology::TriangleList,
        color_states: &[ColorStateDescriptor {
            format: swap_chain_descriptor.format,
            color_blend: BlendDescriptor::REPLACE,
            alpha_blend: BlendDescriptor::REPLACE,
            write_mask: ColorWrite::ALL,
        }],
        depth_stencil_state: None,
        vertex_state: VertexStateDescriptor {
            index_format: IndexFormat::Uint16,
            vertex_buffers: &[],
        },
        sample_count: msaa_texture_descriptor.sample_count,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                screen_size = new_size;

                swap_chain_descriptor.width = screen_size.width;
                swap_chain_descriptor.height = screen_size.height;
                swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

                msaa_texture_descriptor.size = Extent3d {
                    width: screen_size.width,
                    height: screen_size.height,
                    depth: 1,
                };
                msaa_texture = device
                    .create_texture(&msaa_texture_descriptor)
                    .create_default_view();

                let mut encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor { label: None });
                screen_size_uniform.screen_size =
                    [screen_size.width as f32, screen_size.height as f32];
                let staging_buffer = device.create_buffer_with_data(
                    bytemuck::cast_slice(&[screen_size_uniform]),
                    BufferUsage::COPY_SRC,
                );
                encoder.copy_buffer_to_buffer(
                    &staging_buffer,
                    0,
                    &screen_size_uniform_buffer,
                    0,
                    std::mem::size_of::<ScreenSizeUniform>() as BufferAddress,
                );
                queue.submit(&[encoder.finish()]);
            }
            Event::RedrawRequested(_) => {
                let display_texture = &swap_chain.get_next_texture().unwrap().view;
                let mut encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor { label: None });
                {
                    time_uniform.time = time.elapsed().as_secs_f32();
                    let staging_buffer = device.create_buffer_with_data(
                        bytemuck::cast_slice(&[time_uniform]),
                        BufferUsage::COPY_SRC,
                    );
                    encoder.copy_buffer_to_buffer(
                        &staging_buffer,
                        0,
                        &time_uniform_buffer,
                        0,
                        std::mem::size_of::<TimeUniform>() as BufferAddress,
                    );

                    let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                        color_attachments: &[RenderPassColorAttachmentDescriptor {
                            attachment: &msaa_texture,
                            resolve_target: Some(display_texture),
                            load_op: LoadOp::Clear,
                            store_op: StoreOp::Store,
                            clear_color: Color::BLACK,
                        }],
                        depth_stencil_attachment: None,
                    });
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.set_pipeline(&pipeline);
                    render_pass.draw(0..6, 0..1);
                }
                queue.submit(&[encoder.finish()]);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
