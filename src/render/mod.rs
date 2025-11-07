#[derive(Clone)]
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

#[derive(Debug)]
pub enum RequestError {
    Adapter(#[allow(dead_code)] wgpu::RequestAdapterError),
    Device(#[allow(dead_code)] wgpu::RequestDeviceError),
}

impl GpuContext {
    pub async fn new() -> Result<Self, RequestError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await
            .map_err(RequestError::Adapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(RequestError::Device)?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}
