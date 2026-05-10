use anyhow::Result;
use crate::profile::DeviceConfig;
use evdev::Device;

impl DeviceConfig {
    pub fn open(self) -> Result<Device> {
        find_device_by_id(&self.id)
            .ok_or_else(|| anyhow::anyhow!("Device '{}' with id {} not found", self.name, self.id))
    }
}

fn find_device_by_id(vendor_product: &str) -> Option<Device> {
    let (vendor_str, product_str) = vendor_product.split_once(':')?;
    let target_vendor = u16::from_str_radix(vendor_str, 16).ok()?;
    let target_product = u16::from_str_radix(product_str, 16).ok()?;

    for (_path, device) in evdev::enumerate() {
        let id = device.input_id();
        if id.vendor() == target_vendor && id.product() == target_product {
            return Some(device);
        }
    }
    None
}
