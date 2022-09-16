use bevy::{
    core_pipeline::core_2d::graph::node::MAIN_PASS,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_graph::{self, RenderGraph},
        render_resource::encase::UniformBuffer,
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        RenderApp, RenderStage,
    },
};

pub struct JuliaPlugin;

impl Plugin for JuliaPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<JuliaData>()
            .add_plugin(ExtractComponentPlugin::<Handle<JuliaData>>::default())
            .add_plugin(RenderAssetPlugin::<JuliaData>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<JuliaPipeline>()
            .add_system_to_stage(RenderStage::Extract, extract_julia)
            .add_system_to_stage(RenderStage::Queue, queue_bind_group);

        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_node("julia", JuliaDispatch);
        render_graph.add_node_edge("julia", MAIN_PASS).unwrap();
    }
}

#[derive(Clone, Default, TypeUuid)]
#[uuid = "fe4bd1fe-10b1-4762-8507-446740817c63"]
pub struct JuliaData {
    pub c: Vec2,
    pub view_center: Vec2,
    pub view_scale: f32,
    pub view_aspect: f32,
    pub iters: u32,
    pub image: Handle<Image>,
}

#[derive(Clone, Default, ShaderType)]
struct JuliaBuffer {
    c: Vec2,
    w: f32,
    h: f32,
    view_center: Vec2,
    view_scale: f32,
    view_aspect: f32,
    iters: u32,
}

impl JuliaBuffer {
    fn new(data: &JuliaData, image: &GpuImage) -> Self {
        Self {
            c: data.c,
            w: image.size.x,
            h: image.size.y,
            view_center: data.view_center,
            view_scale: data.view_scale,
            view_aspect: data.view_aspect,
            iters: data.iters,
        }
    }
}

struct JuliaSize {
    w: u32,
    h: u32,
}

pub struct GpuJuliaData {
    params: Buffer,
}

impl RenderAsset for JuliaData {
    type ExtractedAsset = JuliaData;
    type PreparedAsset = GpuJuliaData;
    type Param = (SRes<RenderDevice>, SRes<RenderAssets<Image>>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        data: Self::ExtractedAsset,
        (render_device, images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        if let Some(image) = images.get(&data.image) {
            let buffer_data = JuliaBuffer::new(&data, image);
            let mut buffer = UniformBuffer::new(Vec::new());
            buffer.write(&buffer_data).unwrap();

            let params_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("mandelbrot_material_uniform_fs_buffer"),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                contents: buffer.as_ref(),
            });

            Ok(GpuJuliaData {
                params: params_buffer,
            })
        } else {
            Err(PrepareAssetError::RetryNextUpdate(data))
        }
    }
}

struct JuliaImage(pub Handle<Image>);
struct JuliaBindGroup(BindGroup);

fn extract_julia(
    mut commands: Commands,
    data: Res<Handle<JuliaData>>,
    params: Res<Assets<JuliaData>>,
    images: Res<Assets<Image>>,
) {
    commands.insert_resource(data.clone());
    let data = params.get(&data).unwrap();
    let image = images.get(&data.image).unwrap();
    let size = image.texture_descriptor.size;

    commands.insert_resource(JuliaSize {
        w: size.width,
        h: size.height,
    });
    commands.insert_resource(JuliaImage(data.image.clone()));
}

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<JuliaPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    params: Res<RenderAssets<JuliaData>>,
    julia_image: Res<JuliaImage>,
    data: Res<Handle<JuliaData>>,
    render_device: Res<RenderDevice>,
) {
    let view = &gpu_images[&julia_image.0];
    if let Some(data_buffer) = params.get(&data) {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("julia_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view.texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: data_buffer.params.as_entire_binding(),
                },
            ],
        });
        commands.insert_resource(JuliaBindGroup(bind_group));
    }
}

pub struct JuliaPipeline {
    pipeline: ComputePipeline,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for JuliaPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let shader_source = include_str!("../assets/shaders/julia.wgsl");
        let shader = render_device.create_shader_module(ShaderModuleDescriptor {
            label: Some("julia_shader"),
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let texture_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("julia_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::ReadWrite,
                            format: TextureFormat::R32Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("julia_pipline_layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = render_device.create_compute_pipeline(&RawComputePipelineDescriptor {
            label: Some("julia_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "julia",
        });

        JuliaPipeline {
            pipeline,
            bind_group_layout: texture_bind_group_layout,
        }
    }
}

struct JuliaDispatch;

impl render_graph::Node for JuliaDispatch {
    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline = world.get_resource::<JuliaPipeline>().unwrap();
        if let Some(texture_bind_group) = world.get_resource::<JuliaBindGroup>() {
            let size = &world.get_resource::<JuliaSize>().unwrap();

            let mut pass = render_context
                .command_encoder
                .begin_compute_pass(&ComputePassDescriptor::default());

            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &texture_bind_group.0, &[]);
            pass.dispatch_workgroups((size.w + 7) / 8, (size.h + 7) / 8, 1);
        }

        Ok(())
    }
}
