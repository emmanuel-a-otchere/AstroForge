//! wgpu display path — float-texture rendering painted into egui via
//! `egui_wgpu::CallbackTrait` custom paint hooks.
//!
//! Slice P1b. The pipeline is deliberately minimal: a fullscreen
//! triangle sampling an rgba16float texture with linear filtering
//! (the wgpu counterpart of `image-canvas-webgl.ts`). Tone mapping
//! stays CPU-side (core's canonical math); the wgpu compute shaders
//! from `src/lib/webgpu-*.ts` land with the Compare workspace (P3e).

use std::borrow::Cow;

use astroforge_core::image::F32Image;
use egui_wgpu::wgpu;

/// Device handles cloned from eframe's shared wgpu render state.
/// Cloning is cheap (Arc-backed) and shares the same device egui
/// renders with, so the callback can bind resources directly.
#[derive(Clone)]
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub target_format: wgpu::TextureFormat,
    pub adapter_info: String,
}

impl GpuContext {
    pub fn from_render_state(rs: &egui_wgpu::RenderState) -> Self {
        Self {
            device: rs.device.clone(),
            queue: rs.queue.clone(),
            target_format: rs.target_format,
            adapter_info: rs.adapter.get_info().name,
        }
    }
}

/// Display pipeline for one image canvas.
pub struct ImagePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
    /// RGBA f32 source kept for CPU fallback / re-upload.
    dims: [u32; 2],
}

const DISPLAY_SHADER: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    // Fullscreen triangle.
    var p = array(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    let pos = p[i];
    var out: VsOut;
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>((pos.x + 1.0) * 0.5, 1.0 - (pos.y + 1.0) * 0.5);
    return out;
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    // Tone-mapped display values straight into the sRGB target; no
    // extra encode (matches the CPU path and the web GLSL layer).
    return textureSample(tex, samp, in.uv);
}
"#;

impl ImagePipeline {
    pub fn new(ctx: &GpuContext) -> Self {
        let module = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("astroforge.display"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(DISPLAY_SHADER)),
            });

        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("astroforge.display.bgl"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
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
                });

        let layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("astroforge.display.pl"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("astroforge.display.pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.target_format,
                        // Full replace inside the viewport; egui's
                        // own content outside is untouched.
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });

        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("astroforge.display.sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            bind_group_layout,
            sampler,
            bind_group: None,
            dims: [0, 0],
        }
    }

    /// Upload linear-light RGBA (4 floats per pixel) as rgba16float.
    /// The tone map already ran CPU-side; this is display-ready
    /// signal, matching what `to_color_image` would quantize.
    pub fn upload_rgba_f32(&mut self, ctx: &GpuContext, width: u32, height: u32, rgba: &[f32]) {
        debug_assert_eq!(rgba.len(), (width * height * 4) as usize);
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("astroforge.display.texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let halves: Vec<u16> = rgba
            .iter()
            .map(|v| half::f16::from_f32(v.clamp(0.0, 1.0)).to_bits())
            .collect();
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&halves),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4 * 2),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.bind_group = Some(ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("astroforge.display.bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
        self.dims = [width, height];
    }

    /// Upload a tone-mapped `F32Image` (1 or 3 channels), expanding
    /// to RGBA with alpha 1.
    pub fn upload_f32_image(&mut self, ctx: &GpuContext, image: &F32Image) {
        let (w, h, c) = (image.width(), image.height(), image.channels());
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let ch = |i: usize| image[(i.min(c - 1), y, x)].clamp(0.0, 1.0);
                rgba.push(ch(0));
                rgba.push(ch(1));
                rgba.push(ch(2));
                rgba.push(1.0);
            }
        }
        self.upload_rgba_f32(ctx, w as u32, h as u32, &rgba);
    }

    pub fn dims(&self) -> [u32; 2] {
        self.dims
    }

    pub fn has_texture(&self) -> bool {
        self.bind_group.is_some()
    }
}

/// Build the egui paint callback covering `rect`. Clones the bind
/// group + pipeline into the callback so they outlive the frame
/// (the render pass requires 'static resources).
pub fn paint_callback(pipeline: &ImagePipeline, rect: egui::Rect) -> egui::PaintCallback {
    let draw = DrawCallback {
        pipeline: pipeline.pipeline.clone(),
        bind_group: pipeline
            .bind_group
            .clone()
            .expect("paint_callback called before upload"),
    };
    egui_wgpu::Callback::new_paint_callback(rect, draw)
}

struct DrawCallback {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

impl egui_wgpu::CallbackTrait for DrawCallback {
    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let vp = info.viewport_in_pixels();
        if vp.width_px == 0 || vp.height_px == 0 {
            return;
        }
        render_pass.set_viewport(
            vp.left_px as f32,
            vp.top_px as f32,
            vp.width_px as f32,
            vp.height_px as f32,
            0.0,
            1.0,
        );
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
        // Hand the full-screen viewport back to egui's renderer.
        let [w, h] = info.screen_size_px;
        render_pass.set_viewport(0.0, 0.0, w as f32, h as f32, 0.0, 1.0);
    }
}
