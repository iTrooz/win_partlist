use win_partlist::{
    list_disks,
    types::{Disk, PartitionExtra},
};

/// Example function showing how to use the structured data returned by list_disk
fn analyze_disk_data(disks: Vec<Disk>) {
    println!("=== Disk Analysis Example ===");

    for (disk_index, disk) in disks.into_iter().enumerate() {
        println!(
            "Disk {}: {} partition(s)",
            disk_index,
            disk.partitions.len()
        );

        // Display each partition
        for partition in disk.partitions {
            // Only display valid partitions (partition number > 0 and reasonable size)
            println!(
                "  Partition {}: {} MB (offset: {} bytes)",
                partition.partition_number,
                partition.partition_length / (1024 * 1024),
                partition.starting_offset
            );
            match partition.extra {
                PartitionExtra::Gpt(gpt) => {
                    let name: String = String::from_utf16(&gpt.name)
                        .unwrap_or_else(|_| "Invalid UTF-16".to_string());
                    println!(
                        "    GPT: GUID: {}, Type: {}, Name: {}",
                        gpt.partition_id, gpt.partition_type, name
                    );
                }
                PartitionExtra::Mbr(mbr) => {
                    println!(
                        "    MBR: Type: {}, ID: {}",
                        mbr.partition_type, mbr.partition_id
                    );
                }
            }
        }
    }
}

/// Main entry point - displays logical drives and queries partition info for physical drives
fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let disks = list_disks().expect("Failed to list disks");
    analyze_disk_data(disks);
}
