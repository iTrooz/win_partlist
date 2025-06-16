use windows::{
    core::*,
    Win32::Storage::FileSystem::*,
    Win32::System::IO::*,
    Win32::System::Ioctl::*,
    Win32::Foundation::*,
};

/// Performs DeviceIoControl with automatic buffer reallocation if needed
/// Returns (success_result, buffer, bytes_returned)
unsafe fn device_io_control_with_realloc(
    handle: HANDLE,
    ioctl_code: u32,
    initial_buffer_size: usize,
    debug_name: &str,
) -> Result<(Vec<u8>, u32)> {
    let mut buffer = vec![0u8; initial_buffer_size];
    let mut bytes_returned = 0u32;

    // First attempt with initial buffer size
    let mut result = DeviceIoControl(
        handle,
        ioctl_code,
        None,
        0,
        Some(buffer.as_mut_ptr() as *mut _),
        buffer.len() as u32,
        Some(&mut bytes_returned),
        None,
    );

    // If the buffer was too small, reallocate and try again
    if result.is_err() && bytes_returned > buffer.len() as u32 {
        println!("Buffer too small for {}, reallocating from {} to {} bytes", 
                debug_name, buffer.len(), bytes_returned);
        
        // Reallocate buffer with the required size
        buffer.resize(bytes_returned as usize, 0);
        
        // Try again with the larger buffer
        result = DeviceIoControl(
            handle,
            ioctl_code,
            None,
            0,
            Some(buffer.as_mut_ptr() as *mut _),
            buffer.len() as u32,
            Some(&mut bytes_returned),
            None,
        );
    }

    match result {
        Ok(_) => Ok((buffer, bytes_returned)),
        Err(e) => Err(e),
    }
}

unsafe fn list_disks() -> Result<()> {
    // Query the number of physical drives
    for disk_index in 0..16 { // Assuming up to 16 physical drives
        if let Err(e) = list_disk(disk_index) {
            eprintln!("Error querying disk {}: {:?}", disk_index, e);
        }
    }
    Ok(())
}

unsafe fn list_disk(disk_index: u32) -> Result<()> {
    let path = format!(r"\\.\PhysicalDrive{}", disk_index);
    let wpath: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();

    let disk = CreateFileW(
        PCWSTR(wpath.as_ptr()),
        FILE_GENERIC_READ.0,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        None,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        None,
    );

    let disk = match disk {
        Ok(handle) => handle,
        Err(_) => return Ok(()), // Skip drives that don't exist
    };

    // get partitions information
    let (buffer, bytes_returned) = match device_io_control_with_realloc(
        disk,
        IOCTL_DISK_GET_DRIVE_LAYOUT_EX,
        4096, // Initial buffer size
        &format!("disk {}", disk_index),
    ) {
        Ok((buf, bytes)) => (buf, bytes),
        Err(_) => {
            let _ = CloseHandle(disk);
            return Ok(()); // Skip drives that can't be queried
        }
    };

    let _ = CloseHandle(disk);

    if bytes_returned < std::mem::size_of::<DRIVE_LAYOUT_INFORMATION_EX>() as u32 {
        return Ok(());
    }

    // Parse the drive layout structure
    let layout = &*(buffer.as_ptr() as *const DRIVE_LAYOUT_INFORMATION_EX);
    
    println!("Disk {}: {} partition(s)", disk_index, layout.PartitionCount);
    
    // Loop over each partition in the PartitionEntry array
    let partitions_ptr = layout.PartitionEntry.as_ptr();
    for partition_idx in 0..layout.PartitionCount {
        let partition = *partitions_ptr.add(partition_idx as usize);
        // Only display valid partitions (partition number > 0 and reasonable size)
        if partition.PartitionNumber > 0 && 
            partition.PartitionNumber < 1000 && 
            partition.PartitionLength > 0 {
            println!("  Partition {}: {} MB (offset: {} bytes)", 
                partition.PartitionNumber,
                partition.PartitionLength / (1024 * 1024),
                partition.StartingOffset
            );
        }
    }
    Ok(())
}

/// Main entry point - displays logical drives and queries partition info for physical drives
fn main() -> Result<()> {
    println!("=== Physical Drives ===");
    unsafe {
        list_disks().expect("Failed to list disks");
    }
    Ok(())
}
