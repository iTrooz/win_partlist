use windows::{
    core::*,
    Win32::Storage::FileSystem::*,
    Win32::System::IO::*,
    Win32::System::Ioctl::*,
    Win32::Foundation::*,
};

#[repr(C)]
#[derive(Debug)]
struct PartitionInformationEx {
    partition_style: u32,
    starting_offset: i64,
    partition_length: i64,
    partition_number: u32,
    rewrite_partition: u8, // BOOLEAN is u8
    recognized: u8, // BOOLEAN is u8
    padding: [u8; 2], // Alignment padding
    // Union field - we'll treat as raw bytes for simplicity
    partition_info: [u8; 32],
}

#[repr(C)]
#[derive(Debug)]
struct DriveLayoutInformationEx {
    partition_style: u32,
    partition_count: u32,
    // Union field for different partition styles  
    drive_layout_info: [u8; 40],
    // Variable-length array of partitions follows
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

    let mut buffer = vec![0u8; 4096];
    let mut bytes_returned = 0u32;

    let result = DeviceIoControl(
        disk,
        0x00070050, // IOCTL_DISK_GET_DRIVE_LAYOUT_EX
        None,
        0,
        Some(buffer.as_mut_ptr() as *mut _),
        buffer.len() as u32,
        Some(&mut bytes_returned),
        None,
    );

    let _ = CloseHandle(disk);

    if result.is_err() {
        return Ok(()); // Skip drives that can't be queried
    }

    if bytes_returned < std::mem::size_of::<DriveLayoutInformationEx>() as u32 {
        return Ok(());
    }

    // Parse the drive layout structure
    // let layout = &*(buffer.as_ptr() as *const DriveLayoutInformationEx);
    let layout = &*(buffer.as_ptr() as *const DRIVE_LAYOUT_INFORMATION_EX);
    
    println!("Disk {}: {} partition(s)", disk_index, layout.PartitionCount);
    
    // // Calculate offset to partition array
    // let partition_array_offset = std::mem::size_of::<DriveLayoutInformationEx>();
    // let partition_size = std::mem::size_of::<PartitionInformationEx>();
    
    // // Loop over each partition
    // for partition_idx in 0..layout.partition_count {
    //     let partition_offset = partition_array_offset + (partition_idx as usize * partition_size);
        
    //     if partition_offset + partition_size <= bytes_returned as usize {
    //         let partition = &*(buffer.as_ptr().add(partition_offset) as *const PartitionInformationEx);
            
    //         // Only display valid partitions (partition number > 0 and reasonable size)
    //         if partition.partition_number > 0 && 
    //             partition.partition_number < 1000 && 
    //             partition.partition_length > 0 {
    //             println!("  Partition {}: {} MB (offset: {} bytes)", 
    //                 partition.partition_number,
    //                 partition.partition_length / (1024 * 1024),
    //                 partition.starting_offset
    //             );
    //         }
    //     }
    // }
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
