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

pub type DisksStructure = Vec<(DRIVE_LAYOUT_INFORMATION_EX, Vec<PARTITION_INFORMATION_EX>)>;

/// List all physical disks and their partitions
pub fn list_disks() -> Result<DisksStructure> {
    let mut disks = Vec::new();

    for disk_index in 0..16 { // Assuming up to 16 physical drives
        let list_disk_res = unsafe { try_list_disk(disk_index) };
        match  list_disk_res {
            Ok(None) => {
                // Disk does not exist. Assume the end
                break;
            },
            Err(e) => {
                return Err(e);
            },
            Ok(Some((layout, partitions))) => {
                disks.push((layout, partitions));
            }
        }
    }
    Ok(disks)
}

/// List the partitions of a physical disk by its index
/// If disk does not exist, returns Ok(None)
unsafe fn try_list_disk(disk_index: u32) -> Result<Option<(DRIVE_LAYOUT_INFORMATION_EX, Vec<PARTITION_INFORMATION_EX>)>> {
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

    CloseHandle(disk)?;

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

        // MBR disks always return 4 partitions. We check if they are valid here
        // See https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-drive_layout_information_ex#members
        if partition.PartitionStyle == PARTITION_STYLE_MBR
            && partition.Anonymous.Mbr.PartitionType == PARTITION_ENTRY_UNUSED as u8 {
            continue;
        }
        partitions.push(partition);
    }

    // Update partition count to match in case of MBR (see above)
    let mut layout_copy = *layout;
    layout_copy.PartitionCount = partitions.len() as u32;

    // Return the drive layout and partitions
    Ok(Some((layout_copy, partitions)))
}
