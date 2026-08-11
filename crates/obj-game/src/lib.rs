use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::cell::RefCell;
use std::time::{Instant};

use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectDescriptor, MethodDesc, BasicObject, PropDesc, call_back_to_vm, ObjectRef};

use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Window},
    keyboard::{KeyCode, PhysicalKey},
};

pub struct TypeInfo {
    pub factory: fn(args: &[Value]) -> Result<ObjectRef>,
    pub descriptor: fn() -> ObjectDescriptor,
    pub constants: fn() -> Vec<(String, Value)>,
}

// --- Renderer ---

struct Renderer {
    #[allow(dead_code)]
    instance: wgpu::Instance,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
];

struct Texture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
}

impl Renderer {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(window.clone())
            .map_err(|e| BasilError(format!("Failed to create surface: {}", e)))?;
            
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.ok_or_else(|| BasilError("Failed to find an appropriate adapter".into()))?;

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ).await.map_err(|e| BasilError(format!("Failed to create device: {}", e)))?;

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

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
                cull_mode: None,
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

        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            surface,
            config,
            pipeline,
            bind_group_layout,
            vertex_buffer,
        })
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

// --- Engine State ---

struct QueuedSprite {
    key: String,
    #[allow(dead_code)]
    x: f32,
    #[allow(dead_code)]
    y: f32,
}

pub struct EngineInner {
    event_loop: Option<EventLoop<()>>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    assets: HashMap<String, Texture>,
    input: HashMap<KeyCode, bool>,
    last_frame: Instant,
    clear_color: wgpu::Color,
    draw_queue: Vec<QueuedSprite>,
}

impl EngineInner {
    fn new() -> Self {
        Self {
            event_loop: Some(EventLoop::new().expect("Failed to create EventLoop")),
            window: None,
            renderer: None,
            assets: HashMap::new(),
            input: HashMap::new(),
            last_frame: Instant::now(),
            clear_color: wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 },
            draw_queue: Vec::new(),
        }
    }

    fn load_texture(&mut self, key: &str, path: &str) -> Result<()> {
        let renderer = self.renderer.as_mut().ok_or_else(|| BasilError("Renderer not initialized".into()))?;
        
        let img = image::open(path)
            .map_err(|e| BasilError(format!("Failed to load image '{}': {}", path, e)))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(key),
            view_formats: &[],
        });

        renderer.queue.write_texture(
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
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &renderer.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        self.assets.insert(key.to_string(), Texture {
            texture,
            view,
            sampler,
            bind_group,
            width,
            height,
        });

        Ok(())
    }
}

// --- Basil Objects ---

pub struct Engine {
    inner: Rc<RefCell<EngineInner>>,
}

impl Engine {
    pub fn new() -> Self {
        Self { inner: Rc::new(RefCell::new(EngineInner::new())) }
    }
}

impl BasicObject for Engine {
    fn type_name(&self) -> &str { "GAME" }
    
    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "ASSETS" => Ok(Value::Object(Rc::new(RefCell::new(AssetsProxy { inner: self.inner.clone() })))),
            "INPUT" => Ok(Value::Object(Rc::new(RefCell::new(InputProxy { inner: self.inner.clone() })))),
            "DRAW" => Ok(Value::Object(Rc::new(RefCell::new(DrawProxy { inner: self.inner.clone() })))),
            _ => Err(BasilError(format!("Unknown property '{}' on GAME", name))),
        }
    }

    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> {
        Err(BasilError("GAME properties are read-only".into()))
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "WINDOW" => {
                let w = args.get(0).and_then(|v| v.as_int()).unwrap_or(800) as u32;
                let h = args.get(1).and_then(|v| v.as_int()).unwrap_or(600) as u32;
                let title = args.get(2).map(|v| v.to_string()).unwrap_or_else(|| "Basil Game".to_string());
                
                let mut inner = self.inner.borrow_mut();
                let event_loop = inner.event_loop.as_ref().ok_or_else(|| BasilError("Event loop not available".into()))?;
                
                let window = WindowBuilder::new()
                    .with_title(title)
                    .with_inner_size(winit::dpi::PhysicalSize::new(w, h))
                    .build(event_loop)
                    .map_err(|e| BasilError(format!("Failed to create window: {}", e)))?;
                
                let window_rc = Arc::new(window);
                let renderer = pollster::block_on(Renderer::new(window_rc.clone()))?;
                
                inner.window = Some(window_rc);
                inner.renderer = Some(renderer);
                
                Ok(Value::Null)
            }
            "RUN" => {
                let init_fn = args.get(0).cloned().unwrap_or(Value::Null);
                let update_fn = args.get(1).cloned().unwrap_or(Value::Null);
                let draw_fn = args.get(2).cloned().unwrap_or(Value::Null);
                
                let el = self.inner.borrow_mut().event_loop.take()
                    .ok_or_else(|| BasilError("GAME.Run can only be called once".into()))?;
                
                let inner_rc = self.inner.clone();
                
                println!("Starting game loop...");
                
                // Call init if provided
                if init_fn != Value::Null {
                    call_back_to_vm(init_fn, &[])?;
                }
                
                inner_rc.borrow_mut().last_frame = Instant::now();
                
                let _ = el.run(move |event, window_target| {
                    window_target.set_control_flow(ControlFlow::Poll);
                    match event {
                        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                            window_target.exit();
                        }
                        Event::WindowEvent { event: WindowEvent::Resized(size), .. } => {
                            if let Ok(mut inner) = inner_rc.try_borrow_mut() {
                                if let Some(renderer) = &mut inner.renderer {
                                    renderer.resize(size);
                                }
                            }
                        }
                        Event::WindowEvent { event: WindowEvent::KeyboardInput { event: KeyEvent { state, physical_key: PhysicalKey::Code(key), .. }, .. }, .. } => {
                            if let Ok(mut inner) = inner_rc.try_borrow_mut() {
                                match state {
                                    ElementState::Pressed => { inner.input.insert(key, true); }
                                    ElementState::Released => { inner.input.insert(key, false); }
                                }
                            }
                        }
                        Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                            let dt = inner_rc.borrow().last_frame.elapsed().as_secs_f64();
                            inner_rc.borrow_mut().last_frame = Instant::now();
                            inner_rc.borrow_mut().draw_queue.clear();
                            
                            // Update
                            if update_fn != Value::Null {
                                if let Err(e) = call_back_to_vm(update_fn.clone(), &[Value::Num(dt)]) {
                                    eprintln!("Error in update callback: {}", e);
                                    window_target.exit();
                                    return;
                                }
                            }
                            
                            // Draw callback
                            if draw_fn != Value::Null {
                                if let Err(e) = call_back_to_vm(draw_fn.clone(), &[]) {
                                    eprintln!("Error in draw callback: {}", e);
                                    window_target.exit();
                                    return;
                                }
                            }
                            
                            // Actual rendering
                            let mut inner = inner_rc.borrow_mut();
                            let clear_color = inner.clear_color;
                            if let Some(renderer) = inner.renderer.take() {
                                let output = match renderer.surface.get_current_texture() {
                                    Ok(t) => t,
                                    Err(_) => {
                                        inner.renderer = Some(renderer);
                                        return;
                                    }
                                };
                                let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                                let mut encoder = renderer.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") });
                                
                                {
                                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                        label: Some("Render Pass"),
                                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                            view: &view,
                                            resolve_target: None,
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(clear_color),
                                                store: wgpu::StoreOp::Store,
                                            },
                                        })],
                                        depth_stencil_attachment: None,
                                        occlusion_query_set: None,
                                        timestamp_writes: None,
                                    });
                                    
                                    render_pass.set_pipeline(&renderer.pipeline);
                                    render_pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
                                    
                                    for queued in &inner.draw_queue {
                                        if let Some(tex) = inner.assets.get(&queued.key) {
                                            render_pass.set_bind_group(0, &tex.bind_group, &[]);
                                            // Simple full screen draw for V1 (ignores position for now to ensure SOMETHING is drawn)
                                            render_pass.draw(0..4, 0..1);
                                        }
                                    }
                                }
                                
                                renderer.queue.submit(std::iter::once(encoder.finish()));
                                output.present();
                                inner.renderer = Some(renderer);
                            }
                        }
                        Event::AboutToWait => {
                            if let Ok(inner) = inner_rc.try_borrow() {
                                if let Some(window) = inner.window.as_ref() {
                                    window.request_redraw();
                                }
                            }
                        }
                        _ => {}
                    }
                });
                Ok(Value::Null)
            }
            "QUIT" => {
                std::process::exit(0);
            }
            _ => Err(BasilError(format!("Unknown method '{}' on GAME", method))),
        }
    }

    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "GAME".into(),
            version: "1.0".into(),
            summary: "Minimal 2D Game Engine".into(),
            properties: vec![
                PropDesc { name: "Assets".into(), type_name: "ASSETS".into(), readable: true, writable: false },
                PropDesc { name: "Input".into(), type_name: "INPUT".into(), readable: true, writable: false },
                PropDesc { name: "Draw".into(), type_name: "DRAW".into(), readable: true, writable: false },
            ],
            methods: vec![
                MethodDesc { name: "Window".into(), arity: 3, arg_names: vec!["w%".into(), "h%".into(), "title$".into()], return_type: "VOID".into() },
                MethodDesc { name: "Run".into(), arity: 3, arg_names: vec!["initFn".into(), "updateFn".into(), "drawFn".into()], return_type: "VOID".into() },
                MethodDesc { name: "Quit".into(), arity: 0, arg_names: vec![], return_type: "VOID".into() },
            ],
            examples: vec![],
        }
    }
}

// --- Assets Proxy ---

struct AssetsProxy {
    inner: Rc<RefCell<EngineInner>>,
}

impl BasicObject for AssetsProxy {
    fn type_name(&self) -> &str { "ASSETS" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("No properties on ASSETS".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("ASSETS properties are read-only".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "LOADTEXTURE" | "LOADTEXTURE%" => {
                let key = args.get(0).map(|v| v.to_string()).ok_or_else(|| BasilError("LOADTEXTURE expects key$".into()))?;
                let path = args.get(1).map(|v| v.to_string()).ok_or_else(|| BasilError("LOADTEXTURE expects path$".into()))?;
                self.inner.borrow_mut().load_texture(&key, &path)?;
                Ok(Value::Int(1))
            }
            _ => Err(BasilError(format!("Unknown method '{}' on ASSETS", method))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "ASSETS".into(), version: "1.0".into(), summary: "Asset manager".into(),
            properties: vec![],
            methods: vec![MethodDesc { name: "LoadTexture".into(), arity: 2, arg_names: vec!["key$".into(), "path$".into()], return_type: "INT".into() }],
            examples: vec![],
        }
    }
}

// --- Input Proxy ---

struct InputProxy {
    inner: Rc<RefCell<EngineInner>>,
}

impl BasicObject for InputProxy {
    fn type_name(&self) -> &str { "INPUT" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("No properties on INPUT".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("INPUT properties are read-only".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "KEYDOWN" | "KEYDOWN%" => {
                let key_name = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                let vk = match key_name.to_ascii_uppercase().as_str() {
                    "UP" => KeyCode::ArrowUp,
                    "DOWN" => KeyCode::ArrowDown,
                    "LEFT" => KeyCode::ArrowLeft,
                    "RIGHT" => KeyCode::ArrowRight,
                    "SPACE" => KeyCode::Space,
                    s if s.len() == 1 => {
                        let c = s.chars().next().unwrap();
                        match c {
                            'A' => KeyCode::KeyA, 'B' => KeyCode::KeyB, 'C' => KeyCode::KeyC, 'D' => KeyCode::KeyD,
                            'E' => KeyCode::KeyE, 'F' => KeyCode::KeyF, 'G' => KeyCode::KeyG, 'H' => KeyCode::KeyH,
                            'I' => KeyCode::KeyI, 'J' => KeyCode::KeyJ, 'K' => KeyCode::KeyK, 'L' => KeyCode::KeyL,
                            'M' => KeyCode::KeyM, 'N' => KeyCode::KeyN, 'O' => KeyCode::KeyO, 'P' => KeyCode::KeyP,
                            'Q' => KeyCode::KeyQ, 'R' => KeyCode::KeyR, 'S' => KeyCode::KeyS, 'T' => KeyCode::KeyT,
                            'U' => KeyCode::KeyU, 'V' => KeyCode::KeyV, 'W' => KeyCode::KeyW, 'X' => KeyCode::KeyX,
                            'Y' => KeyCode::KeyY, 'Z' => KeyCode::KeyZ,
                            _ => return Ok(Value::Int(0)),
                        }
                    }
                    _ => return Ok(Value::Int(0)),
                };
                let pressed = self.inner.borrow().input.get(&vk).copied().unwrap_or(false);
                Ok(Value::Int(if pressed { 1 } else { 0 }))
            }
            _ => Err(BasilError(format!("Unknown method '{}' on INPUT", method))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "INPUT".into(), version: "1.0".into(), summary: "Input state".into(),
            properties: vec![],
            methods: vec![MethodDesc { name: "KeyDown".into(), arity: 1, arg_names: vec!["key$".into()], return_type: "INT".into() }],
            examples: vec![],
        }
    }
}

// --- Draw Proxy ---

struct DrawProxy {
    inner: Rc<RefCell<EngineInner>>,
}

impl BasicObject for DrawProxy {
    fn type_name(&self) -> &str { "DRAW" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("No properties on DRAW".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("DRAW properties are read-only".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "CLEAR" => {
                let r = args.get(0).and_then(|v| v.as_num()).unwrap_or(0.1);
                let g = args.get(1).and_then(|v| v.as_num()).unwrap_or(0.2);
                let b = args.get(2).and_then(|v| v.as_num()).unwrap_or(0.3);
                let a = args.get(3).and_then(|v| v.as_num()).unwrap_or(1.0);
                self.inner.borrow_mut().clear_color = wgpu::Color { r, g, b, a };
                Ok(Value::Null)
            }
            "SPRITE" => {
                let key = args.get(0).map(|v| v.to_string()).ok_or_else(|| BasilError("SPRITE expects key$".into()))?;
                let x = args.get(1).and_then(|v| v.as_num()).unwrap_or(0.0) as f32;
                let y = args.get(2).and_then(|v| v.as_num()).unwrap_or(0.0) as f32;
                self.inner.borrow_mut().draw_queue.push(QueuedSprite { key, x, y });
                Ok(Value::Null)
            }
            _ => Err(BasilError(format!("Unknown method '{}' on DRAW", method))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "DRAW".into(), version: "1.0".into(), summary: "Drawing commands".into(),
            properties: vec![],
            methods: vec![
                MethodDesc { name: "Clear".into(), arity: 4, arg_names: vec!["r#".into(), "g#".into(), "b#".into(), "a#".into()], return_type: "VOID".into() },
                MethodDesc { name: "Sprite".into(), arity: 3, arg_names: vec!["key$".into(), "x#".into(), "y#".into()], return_type: "VOID".into() },
            ],
            examples: vec![],
        }
    }
}

pub fn register(add: &mut dyn FnMut(&str, TypeInfo)) {
    add("GAME", TypeInfo {
        factory: |_args| Ok(Rc::new(RefCell::new(Engine::new()))),
        descriptor: || Engine::new().descriptor(),
        constants: || vec![],
    });
}
