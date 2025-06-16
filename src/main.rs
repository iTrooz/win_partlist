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
        match list_disk(disk_index) {
            Ok(Some((layout, partitions))) => {
                println!("Disk {}: {} partition(s)", disk_index, layout.PartitionCount);
                
                // Display each partition
                for partition in &partitions {
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
            },
            Ok(None) => {
                // Disk doesn't exist or couldn't be queried - skip silently
            },
            Err(e) => {
                eprintln!("Error querying disk {}: {:?}", disk_index, e);
            }
        }
    }
    Ok(())
}

unsafe fn list_disk(disk_index: u32) -> Result<Option<(DRIVE_LAYOUT_INFORMATION_EX, Vec<PARTITION_INFORMATION_EX>)>> {
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
        Err(e) => {
            if e.code() == ERROR_FILE_NOT_FOUND.into() {
                return Ok(None);
            }
            return Err(e);
        }
    };

    // get partitions information
    let (buffer, bytes_returned) = match device_io_control_with_realloc(
        disk,
        IOCTL_DISK_GET_DRIVE_LAYOUT_EX,
        4096, // Initial buffer size
        &format!("disk {}", disk_index),
    ) {
        Ok((buf, bytes)) => (buf, bytes),
        Err(e) => {
            if let Err(err) = CloseHandle(disk) {
                eprintln!("Failed to close handle for {}: {:?}", path, err);
            }
            return Err(e);
        }
    };

    let _ = CloseHandle(disk);

    if bytes_returned < std::mem::size_of::<DRIVE_LAYOUT_INFORMATION_EX>() as u32 {
        return Ok(None);
    }

    // Parse the drive layout structure
    let layout = &*(buffer.as_ptr() as *const DRIVE_LAYOUT_INFORMATION_EX);
    
    // Extract partitions into a Vec
    let mut partitions = Vec::new();
    let partitions_ptr = layout.PartitionEntry.as_ptr();
    for partition_idx in 0..layout.PartitionCount {
        let partition = *partitions_ptr.add(partition_idx as usize);
        partitions.push(partition);
    }
    
    // Return the drive layout and partitions
    Ok(Some((*layout, partitions)))
}

/// Example function showing how to use the structured data returned by list_disk
unsafe fn analyze_disk_data() -> Result<()> {
    println!("\n=== Disk Analysis Example ===");
    
    if let Ok(Some((layout, partitions))) = list_disk(0) {
        println!("Disk 0 Analysis:");
        println!("  Partition Style: {}", layout.PartitionStyle);
        println!("  Total Partitions: {}", layout.PartitionCount);
        
        let mut total_size = 0i64;
        let mut valid_partitions = 0;
        
        for partition in &partitions {
            if partition.PartitionNumber > 0 && partition.PartitionLength > 0 {
                total_size += partition.PartitionLength;
                valid_partitions += 1;
            }
        }
        
        println!("  Valid Partitions: {}", valid_partitions);
        println!("  Total Used Space: {} GB", total_size / (1024 * 1024 * 1024));
    }
    
    Ok(())
}

/// Main entry point - displays logical drives and queries partition info for physical drives
fn main() -> Result<()> {
    println!("=== Physical Drives ===");
    unsafe {
        list_disks().expect("Failed to list disks");
        analyze_disk_data().expect("Failed to analyze disk data");
    }
    Ok(())
}
