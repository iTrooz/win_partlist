use win_partlist::{list_disks, DisksStructure};
/// Example function showing how to use the structured data returned by list_disk
fn analyze_disk_data(disks: DisksStructure) {
    println!("=== Disk Analysis Example ===");

    for (disk_index, (layout, partitions)) in disks.iter().enumerate() {
        println!("Disk {}: {} partition(s)", disk_index, layout.PartitionCount);
        
        // Display each partition
        for partition in partitions {
            // Only display valid partitions (partition number > 0 and reasonable size)
            println!("  Partition {}: {} MB (offset: {} bytes)", 
                partition.PartitionNumber,
                partition.PartitionLength / (1024 * 1024),
                partition.StartingOffset
            );
        }
    }
}

/// Main entry point - displays logical drives and queries partition info for physical drives
fn main() {
    let disks = list_disks().expect("Failed to list disks");
    analyze_disk_data(disks);
}
