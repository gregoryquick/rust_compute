#[allow(dead_code)]
pub struct PipelineManager {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    in_buffer_a: wgpu::Buffer,
    in_buffer_b: wgpu:: Buffer,
    output_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    compute_pipeline: wgpu::ComputePipeline,
}

pub const F32_SIZE: wgpu::BufferAddress = std::mem::size_of::<f32>() as wgpu::BufferAddress;
pub const SIZE: usize = 5;

impl PipelineManager {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
            },
        ).await.unwrap();
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ).await.unwrap();

        //Create buffers
        let buffer_size = (F32_SIZE * SIZE as u64) as wgpu::BufferAddress;
        let input_buffer_desc = wgpu::BufferDescriptor {
            label: Some("Input buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        };
        let output_buffer_desc = wgpu::BufferDescriptor {
            label: Some("Output buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::COPY_SRC,
            mapped_at_creation: false,
        };

        let in_buffer_a = device.create_buffer(&input_buffer_desc);
        let in_buffer_b = device.create_buffer(&input_buffer_desc);
        let output_buffer = device.create_buffer(&output_buffer_desc);

        //Create bind group for pipeline
        let bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: false,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(4),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(4),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(4),
                    },
                    count: None,
                }],
            }
        );
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                 entries: &[wgpu::BindGroupEntry {
                     binding: 0,
                     resource: output_buffer.as_entire_binding(),
                 },
                 wgpu::BindGroupEntry {
                     binding: 1,
                     resource: in_buffer_a.as_entire_binding(),
                 },
                 wgpu::BindGroupEntry {
                     binding: 2,
                     resource: in_buffer_b.as_entire_binding(),
                 }],
            }
        );
        
        //Create compute pipeline
        let cs_src = include_str!("shaders/shader.comp");
        let mut compiler = shaderc::Compiler::new().unwrap();
        let cs_spirv = compiler.compile_into_spirv(cs_src, shaderc::ShaderKind::Compute, "shader.comp", "main", None).unwrap();
        let cs_module = device.create_shader_module(
            &wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::util::make_spirv(cs_spirv.as_binary_u8()),
                flags: wgpu::ShaderFlags::empty(),
            }
        );

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }
        );
        let compute_pipeline = device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &cs_module,
                entry_point: "main",
            }
        );

        PipelineManager {
            adapter,
            device,
            queue,
            in_buffer_a,
            in_buffer_b,
            output_buffer,
            bind_group,
            compute_pipeline,
        }
    }

    pub async fn get_result(&mut self, input_a: &[f32; SIZE], input_b: &[f32; SIZE]) -> Option<Vec<f32>> {
        let size = (F32_SIZE * SIZE as u64) as wgpu::BufferAddress;

        //Create command encoder
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: None 
            }
        );

        //Load input data into gpu
        use wgpu::util::{BufferInitDescriptor, DeviceExt};
        let data_buffer_a = self.device.create_buffer_init(
            &BufferInitDescriptor {
                label: Some("Staging Buffer A"),
                contents: bytemuck::cast_slice(input_a),
                usage: wgpu::BufferUsage::COPY_SRC,
            }
        );
        encoder.copy_buffer_to_buffer(&data_buffer_a, 0, &self.in_buffer_a, 0, size);
        
        let data_buffer_b= self.device.create_buffer_init(
            &BufferInitDescriptor {
                label: Some("Staging Buffer B"),
                contents: bytemuck::cast_slice(input_b),
                usage: wgpu::BufferUsage::COPY_SRC,
            }
        );
        encoder.copy_buffer_to_buffer(&data_buffer_b, 0, &self.in_buffer_b, 0, size);

        //Create the compute pass (Mutably borrows encoder)
        let mut compute_pass = encoder.begin_compute_pass(
            &wgpu::ComputePassDescriptor { 
                label: None 
            }
        );

        //Add compute pass
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);
        //Work groups of x=SIZE, Y = 1, Z = 1
        compute_pass.dispatch(SIZE as u32, 1, 1);

        //Encoder borrow is gone now!
        drop(compute_pass);

        //Copy from gpu buffer to staging buffer on cpu
        let staging_buffer = self.device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Staging buffer"),
                size: size,
                usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false,
            }
        );
        encoder.copy_buffer_to_buffer(&self.output_buffer, 0, &staging_buffer, 0, size);

        //Submit command encoder
        self.queue.submit(Some(encoder.finish()));

        //Creates future for computation
        let buffer_slice = staging_buffer.slice(..);
        let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

        //Wait for computation
        self.device.poll(wgpu::Maintain::Wait);

        //Get computation result
        match buffer_future.await {
            Ok(()) => {
                use std::convert::TryInto;
                //Get buffer contents
                let data = buffer_slice.get_mapped_range();
                //Convert from bytes to f32. f32 is 4 bytes.
                let result: Vec<f32> = data.chunks_exact(4).map(|b| f32::from_ne_bytes(b.try_into().unwrap())).collect();

                //Drop mapped view
                drop(data);
                //Unmap buffer
                staging_buffer.unmap();

                Some(result)
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                None
            }
        }
    }
}

//fn create_pipeline(
//    device: &wgpu::Device,
//    pipeline_desc: &wgpu::ComputePipelineDescriptor
//    cs_module_desc: wgpu::ShaderModuleDescriptor,
//) {}

