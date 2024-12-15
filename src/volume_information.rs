pub struct VolumeInformation {
    pub volume_name: String,
    pub volume_serial_number: u32,
    pub maximum_component_length: u32,
    pub file_system_flags: u32,
    pub file_system_name: String,
}

impl VolumeInformation {
    // fn new(
    //     volume_name: String,
    //     volume_serial_number: u32,
    //     maximum_component_length: u32,
    //     file_system_flags: u32,
    //     file_system_name: String,
    // ) -> Self {
    //     Self {
    //         volume_name,
    //         volume_serial_number,
    //         maximum_component_length,
    //         file_system_flags,
    //         file_system_name,
    //     }
    // }
}
