//! The GPU: the device and queue every pass draws with.

use crate::error::RenderError;

/// A wgpu device and its queue, plus the adapter's name for logs and errors.
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_name: String,
}

impl Gpu {
    /// A device with no window: a high-performance adapter if there is one, otherwise any (a CPU one too).
    pub fn headless() -> Result<Gpu, RenderError> {
        Gpu::headless_where(|_| true)
    }

    /// A device on the best adapter `f` accepts: a discrete card first, then an integrated one, then anything
    /// else (a virtual or CPU one such as lavapipe). Only the primary backends (Vulkan, DX12) are tried.
    pub fn headless_where(f: impl Fn(&wgpu::AdapterInfo) -> bool) -> Result<Gpu, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let mut adapters: Vec<wgpu::Adapter> = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY))
            .into_iter()
            .filter(|a| f(&a.get_info()))
            .collect();
        adapters.sort_by_key(|a| preference(a.get_info().device_type));
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| unusable("no Vulkan or DirectX 12 adapter was found"))?;
        let adapter_name = adapter.get_info().name;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bf2-render"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .map_err(|e| unusable(&format!("{adapter_name} refused a device ({e})")))?;
        Ok(Gpu {
            device,
            queue,
            adapter_name,
        })
    }

    /// Wraps a device someone else made (the app's, through eframe).
    pub fn from_device(device: wgpu::Device, queue: wgpu::Queue, adapter_name: String) -> Gpu {
        Gpu {
            device,
            queue,
            adapter_name,
        }
    }
}

/// Lower is better: real cards before virtual and CPU ones.
fn preference(t: wgpu::DeviceType) -> u8 {
    match t {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Cpu => 3,
        wgpu::DeviceType::Other => 4,
    }
}

fn unusable(why: &str) -> RenderError {
    RenderError(format!(
        "This PC's graphics card could not be used for Engine 2: {why}."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_finds_an_adapter() {
        let gpu = Gpu::headless().expect("an adapter");
        assert!(!gpu.adapter_name.is_empty());
    }

    #[test]
    fn no_adapter_is_a_sentence() {
        let err = Gpu::headless_where(|_| false).err().expect("no adapter matches");
        assert!(
            err.to_string()
                .starts_with("This PC's graphics card could not be used for Engine 2"),
            "{err}"
        );
    }
}
