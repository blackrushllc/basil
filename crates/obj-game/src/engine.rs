use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use winit::{
    event::{Event, WindowEvent, ElementState},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
    keyboard::{Key, NamedKey},
};
use basil_common::{Result, BasilError};
use basil_bytecode::{Value, call_back_to_vm};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0,
];

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SpriteUniform {
    pos: [f32; 2],
    size: [f32; 2],
}

pub struct Engine {
    width: u32,
    height: u32,
    title: String,
    keys_down: HashSet<String>,
    textures: HashMap<String, TextureHandle>,
    pending_textures: Vec<(String, String)>,
    draw_queue: Vec<QueuedSprite>,
    clear_color: wgpu::Color,
    should_quit: bool,
}

struct QueuedSprite {
    key: String,
    x: f32,
    y: f32,
}

struct TextureHandle {
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            width: 800,
            height: 600,
            title: "Basil Game".into(),
            keys_down: HashSet::new(),
            textures: HashMap::new(),
            pending_textures: Vec::new(),
            draw_queue: Vec::new(),
            clear_color: wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            should_quit: false,
        }
    }

    pub fn set_window_config(&mut self, w: u32, h: u32, title: String) {
        self.width = w;
        self.height = h;
        self.title = title;
    }

    pub fn is_key_down(&self, key: &str) -> bool {
        self.keys_down.contains(&key.to_ascii_uppercase())
    }

    pub fn clear(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = wgpu::Color {
            r: r as f64,
            g: g as f64,
            b: b as f64,
            a: a as f64,
        };
    }

    pub fn draw_sprite(&mut self, key: &str, x: f32, y: f32) -> Result<()> {
        self.draw_queue.push(QueuedSprite {
            key: key.to_string(),
            x,
            y,
        });
        Ok(())
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn load_texture(&mut self, key: &str, path: &str) -> Result<()> {
        self.pending_textures.push((key.to_string(), path.to_string()));
        Ok(())
    }

    fn process_pending_textures(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler) -> Result<()> {
        let pending: Vec<_> = self.pending_textures.drain(..).collect();
        for (key, path) in pending {
            let img = image::open(&path).map_err(|e| BasilError(format!("Failed to load texture '{}': {}", path, e)))?;
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();

            let texture_size = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some(&key),
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
                texture_size,
            );

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
                label: Some(&format!("{}_bind_group", key)),
            });

            self.textures.insert(key, TextureHandle {
                bind_group,
                width,
                height,
            });
        }
        Ok(())
    }

    pub fn run(&mut self, init_fn: Value, update_fn: Value, draw_fn: Value) -> Result<()> {
        let event_loop = EventLoop::new().map_err(|e| BasilError(e.to_string()))?;
        let window = Arc::new(WindowBuilder::new()
            .with_title(&self.title)
            .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height))
            .build(&event_loop)
            .map_err(|e| BasilError(e.to_string()))?);

        let mut state = pollster::block_on(WgpuState::new(window.clone()))?;
        
        // We might have deferred textures to load. 
        // For simplicity in V1, let's assume textures are loaded during or after initFn.
        // Actually, we need a way to load textures while the loop is running.
        
        let mut last_frame = std::time::Instant::now();

        // Call initFn once
        call_back_to_vm(init_fn, &[]).map_err(|e| BasilError(format!("Error in initFn: {}", e)))?;

        event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            state.resize(window.inner_size());
                        }
                        WindowEvent::KeyboardInput { event: input, .. } => {
                            if let Some(key_str) = map_key(&input.logical_key) {
                                if input.state == ElementState::Pressed {
                                    self.keys_down.insert(key_str);
                                } else {
                                    self.keys_down.remove(&key_str);
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            if self.should_quit {
                                elwt.exit();
                                return;
                            }

                            let now = std::time::Instant::now();
                            let dt = now.duration_since(last_frame).as_secs_f64();
                            last_frame = now;

                            // Update
                            if let Err(e) = self.process_pending_textures(&state.device, &state.queue, &state.texture_bind_group_layout, &state.sampler) {
                                eprintln!("{}", e);
                                elwt.exit();
                                return;
                            }
                            if let Err(e) = call_back_to_vm(update_fn.clone(), &[Value::Num(dt)]) {
                                eprintln!("Error in updateFn: {}", e);
                                elwt.exit();
                                return;
                            }

                            // Draw
                            self.draw_queue.clear();
                            if let Err(e) = call_back_to_vm(draw_fn.clone(), &[]) {
                                eprintln!("Error in drawFn: {}", e);
                                elwt.exit();
                                return;
                            }

                            // Render
                            match state.render(self) {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                                Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                                Err(e) => eprintln!("{:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        }).map_err(|e| BasilError(e.to_string()))
    }
}

fn map_key(key: &Key) -> Option<String> {
    match key {
        Key::Named(NamedKey::ArrowLeft) => Some("LEFT".into()),
        Key::Named(NamedKey::ArrowRight) => Some("RIGHT".into()),
        Key::Named(NamedKey::ArrowUp) => Some("UP".into()),
        Key::Named(NamedKey::ArrowDown) => Some("DOWN".into()),
        Key::Named(NamedKey::Space) => Some("SPACE".into()),
        Key::Character(s) => Some(s.to_uppercase()),
        _ => None,
    }
}

struct WgpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    sprite_uniform_buffer: wgpu::Buffer,
    sprite_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl WgpuState {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(window.clone()).map_err(|e| BasilError(e.to_string()))?;
        
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.ok_or_else(|| BasilError("Failed to find an appropriate adapter".into()))?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.map_err(|e| BasilError(e.to_string()))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let globals_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("globals_bind_group_layout"),
        });

        let sprite_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("sprite_bind_group_layout"),
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &globals_bind_group_layout,
                &sprite_bind_group_layout,
                &texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                    }
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let globals = Globals {
            proj: create_proj_matrix(size.width as f32, size.height as f32),
        };
        let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Globals Buffer"),
            contents: bytemuck::cast_slice(&[globals]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &globals_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                }
            ],
            label: Some("globals_bind_group"),
        });

        let sprite_uniform = SpriteUniform { pos: [0.0, 0.0], size: [100.0, 100.0] };
        let sprite_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Uniform Buffer"),
            contents: bytemuck::cast_slice(&[sprite_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &sprite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sprite_uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("sprite_bind_group"),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            globals_buffer,
            globals_bind_group,
            sprite_uniform_buffer,
            sprite_bind_group,
            texture_bind_group_layout,
            sampler,
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            
            let globals = Globals {
                proj: create_proj_matrix(new_size.width as f32, new_size.height as f32),
            };
            self.queue.write_buffer(&self.globals_buffer, 0, bytemuck::cast_slice(&[globals]));
        }
    }

    fn render(&mut self, engine: &mut Engine) -> core::result::Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(engine.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.globals_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            for sprite in &engine.draw_queue {
                if let Some(tex) = engine.textures.get(&sprite.key) {
                    let uniform = SpriteUniform {
                        pos: [sprite.x, sprite.y],
                        size: [tex.width as f32, tex.height as f32],
                    };
                    self.queue.write_buffer(&self.sprite_uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
                    
                    render_pass.set_bind_group(1, &self.sprite_bind_group, &[]);
                    render_pass.set_bind_group(2, &tex.bind_group, &[]);
                    render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn create_proj_matrix(width: f32, height: f32) -> [[f32; 4]; 4] {
    [
        [2.0 / width, 0.0, 0.0, 0.0],
        [0.0, -2.0 / height, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [-1.0, 1.0, 0.0, 1.0],
    ]
}
