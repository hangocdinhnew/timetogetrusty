use crate::renderer::graphics_context::GraphicsContext;
use wgpu::{BufferDescriptor, BufferUsages};

use crate::renderer::{MeshInstance, ViewProjection};

pub enum BufferType {
    Model,
    Camera
}

pub enum BgType {
    Mesh,
}

pub struct BufferManager {
    pub model_sbuf: wgpu::Buffer,
    pub model_sbuf_size: usize,
    pub camera_ubuf: wgpu::Buffer,
    pub mesh_bg_layout: wgpu::BindGroupLayout,
    pub mesh_bind_group: wgpu::BindGroup,
}

impl BufferManager {
    pub fn new(gfx: &GraphicsContext) -> Self {
        let model_sbuf = gfx.device.create_buffer(&BufferDescriptor {
            label: None,
            size: (size_of::<MeshInstance>() as u64) * 128,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        
        let mesh_instance = MeshInstance {
            model: glam::Mat4::IDENTITY,
        };
        
        gfx.queue.write_buffer(&model_sbuf, 0, bytemuck::bytes_of(&mesh_instance));
        
        let camera_ubuf = gfx.device.create_buffer(&BufferDescriptor {
            label: None,
            size: size_of::<ViewProjection>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        
        let view_projection = ViewProjection {
            view: glam::Mat4::IDENTITY,
            projection: glam::Mat4::IDENTITY,
        };
        
        gfx.queue.write_buffer(&camera_ubuf, 0, bytemuck::bytes_of(&view_projection));
        
        let mesh_bg_layout = gfx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage {
                                read_only: true,
                            },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }
        );
        
        let mesh_bind_group = gfx.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Bind Group"),
                layout: &mesh_bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: model_sbuf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: camera_ubuf.as_entire_binding(),
                    },
                ],
            }
        );
        
        Self {
            model_sbuf,
            mesh_bg_layout,
            mesh_bind_group,
            model_sbuf_size: size_of::<MeshInstance>() * 128,
            camera_ubuf,
        }
    }
    
    pub fn recreate_model_sbuf(&mut self, gfx: &GraphicsContext) {
        self.model_sbuf = gfx.device.create_buffer(&BufferDescriptor {
            label: None,
            size: self.model_sbuf_size as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        
        self.mesh_bind_group = gfx.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Bind Group"),
                layout: &self.mesh_bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.model_sbuf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.camera_ubuf.as_entire_binding(),
                    },
                ],
            },
        );
    }
    
    pub fn write_buf(&mut self, gfx: &GraphicsContext, buffer_type: BufferType, data: &[u8]) {
        match buffer_type {
            BufferType::Model => gfx.write_buf(&self.model_sbuf, data),
            BufferType::Camera => gfx.write_buf(&self.camera_ubuf, data),
        }
    }
}
