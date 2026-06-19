use crate::renderer::graphics_context::GraphicsContext;
use crate::renderer::buffer::BufferManager;
use wgpu::{RenderPipeline};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawMethod {
    Triangles,
    Lines,
}

pub struct PipelineManager {
    pub mesh_pipeline: RenderPipeline,
    pub mesh_lines_pipeline: RenderPipeline,
}

pub enum PipelineType {
    Mesh,
    MeshLines,
}

pub const VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: (size_of::<f32>() * 3) as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &[
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x3,
            offset: 0,
            shader_location: 0,
        },
    ],
};

impl PipelineManager {
    pub fn new(gfx: &GraphicsContext, buffer: &BufferManager) -> Self {
        let mesh_shader = gfx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mesh"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/Mesh.wgsl").into()),
        });
        
        let mesh_playout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mesh pipeline layout"),
            bind_group_layouts: &[Some(&buffer.mesh_bg_layout)],
            immediate_size: 0,
        });
        
        let mesh_pipeline = gfx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh pipeline"),
            layout: Some(&mesh_playout),
            
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[VERTEX_LAYOUT],
            },
            
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(gfx.surface_caps.formats[0].into())],
            }),
            
            primitive: wgpu::PrimitiveState::default(),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            cache: None,
        });

	let mut mesh_lines_primitive = wgpu::PrimitiveState::default();
	mesh_lines_primitive.topology = wgpu::PrimitiveTopology::LineList;

        let mesh_lines_pipeline = gfx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh pipeline"),
            layout: Some(&mesh_playout),
            
            vertex: wgpu::VertexState {
                module: &mesh_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[VERTEX_LAYOUT],
            },
            
            fragment: Some(wgpu::FragmentState {
                module: &mesh_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(gfx.surface_caps.formats[0].into())],
            }),
            
            primitive: mesh_lines_primitive,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            cache: None,
        });
        
        Self {
            mesh_pipeline,
	    mesh_lines_pipeline,
        }
    }
}
