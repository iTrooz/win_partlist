use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::CloseHandle,
        Storage::FileSystem::{
            GetLogicalDrives, GetDriveTypeW, CreateFileW, 
            FILE_SHARE_READ, FILE_SHARE_WRITE,
            OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL,
        },
        System::IO::DeviceIoControl,
    },
};

// Constants that may not be available in the crate
const DRIVE_FIXED: u32 = 3;
const IOCTL_DISK_GET_DRIVE_LAYOUT_EX: u32 = 0x00070050;

/// Lists all logical drives (C:, D:, etc.) and identifies which ones are fixed drives (hard disks)
fn list_logical_drives() {
    let mask = unsafe { GetLogicalDrives() };
    for i in 0..26 {
        if mask & (1 << i) != 0 {
            let mut buf = [0u16; 4];
            buf[0] = (b'A' + i as u8) as u16;
            buf[1] = b':' as u16;
            buf[2] = b'\\' as u16;
            buf[3] = 0;
            let dt = unsafe { GetDriveTypeW(PCWSTR(buf.as_ptr())) };
            if dt == DRIVE_FIXED {
                println!("Fixed drive: {}:", (b'A' + i as u8) as char);
            }
        }
    }
}

/// Queries partition information for a specific physical drive by disk index
/// Uses Windows DeviceIoControl API to get drive layout information
fn list_partitions(disk_index: u32) {
    let path = format!(r"\\.\PhysicalDrive{}", disk_index);
    let wpath: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let h_result = unsafe {
        CreateFileW(
            PCWSTR(wpath.as_ptr()),
            FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
    };
    
    let h = match h_result {
        Ok(handle) => handle,
        Err(_) => {
            // Drive doesn't exist, skip it
            return;
        }
    };

    let mut bytes_returned = 0u32;
    let mut layout = vec![0u8; 1024]; // Fixed size buffer instead of using undefined type
    let ok = unsafe {
        DeviceIoControl(
            h,
            IOCTL_DISK_GET_DRIVE_LAYOUT_EX,
            None,
            0,
            Some(layout.as_mut_ptr() as _),
            layout.len() as _,
            Some(&mut bytes_returned),
            None,
        ).is_ok()
    };
    unsafe { let _ = CloseHandle(h); };
    if ok {
        // parse layout[..bytes_returned] as DRIVE_LAYOUT_INFORMATION_EX
        println!("Got partition layout for {} bytes", bytes_returned);
    }
}

/// Main entry point - displays logical drives and queries partition info for physical drives 0-3
fn main() {
    list_logical_drives();
    for disk in 0..4 {
        list_partitions(disk);
    }
}
