use std::ops::Range;

use wgpu::{SurfaceTexture, CurrentSurfaceTexture, TextureView, CommandEncoder};
use crate::renderer::{
    graphics_context::GraphicsContext,
    pipeline::{PipelineManager, PipelineType},
    buffer::{BufferManager, BgType},
};

pub struct RenderFrame {
    surface_texture: SurfaceTexture,
    texture_view: TextureView,
    encoder: CommandEncoder,
}

pub enum CurrentRenderFrame {
    Success(RenderFrame),
    Suboptimal,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

impl RenderFrame {
    pub fn begin(gfx: &GraphicsContext) -> CurrentRenderFrame {
        let surface_texture = match gfx.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Suboptimal(_) => return CurrentRenderFrame::Suboptimal,
            CurrentSurfaceTexture::Timeout => return CurrentRenderFrame::Timeout,
            CurrentSurfaceTexture::Occluded => return CurrentRenderFrame::Occluded,
            CurrentSurfaceTexture::Outdated => return CurrentRenderFrame::Outdated,
            CurrentSurfaceTexture::Lost => return CurrentRenderFrame::Lost,
            CurrentSurfaceTexture::Validation => return CurrentRenderFrame::Validation,
        };
        
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(gfx.surface_caps.formats[0].add_srgb_suffix()),
                ..Default::default()
            });
        
        let encoder = gfx.device.create_command_encoder(&Default::default());
        
        CurrentRenderFrame::Success(Self {
            surface_texture,
            texture_view,
            encoder,
        })
    }
    
    pub fn begin_pass(&mut self, gfx: &GraphicsContext) -> RenderPass<'_> {
        let pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &gfx.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        
        RenderPass {
            pass,
        }
    }
    
    pub fn end(self, gfx: &GraphicsContext) {
        gfx.queue.submit(Some(self.encoder.finish()));
        self.surface_texture.present();
    }
}

pub struct RenderPass<'a> {
    pass: wgpu::RenderPass<'a>,
}

impl<'a> RenderPass<'a> {
    pub fn set_pipeline(&mut self, pipeline: &PipelineManager, pipeline_type: PipelineType) {
        match pipeline_type {
            PipelineType::Mesh => self.pass.set_pipeline(&pipeline.mesh_pipeline),
	    PipelineType::MeshLines => self.pass.set_pipeline(&pipeline.mesh_lines_pipeline),
        }
    }
    
    pub fn set_vertex_buffer(&mut self, buffer: &wgpu::Buffer) {
        self.pass.set_vertex_buffer(0, buffer.slice(..));
    }
    
    pub fn set_index_buffer(&mut self, buffer: &wgpu::Buffer) {
        self.pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
    }
    
    pub fn set_bind_group(&mut self, buffer: &BufferManager, bg_type: BgType) {
        match bg_type {
            BgType::Mesh => self.pass.set_bind_group(0, &buffer.mesh_bind_group, &[]),
        }
    }
    
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.pass.draw_indexed(indices, base_vertex, instances)
    }
}
