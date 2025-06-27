use windows::Win32::System::Ioctl::*;

pub struct Disk {
    pub partition_style: u32,
    pub partitions: Vec<Partition>,
    pub extra: DiskExtra,
}

pub enum DiskExtra {
    Mbr(DiskExtraMbr),
    Gpt(DiskExtraGpt),
}

pub struct DiskExtraMbr {
    pub signature: u32,
    pub checksum: u32,
}

pub struct DiskExtraGpt {
    pub disk_id: uuid::Uuid,
    pub starting_usable_offset: i64,
    pub usable_length: i64,
    pub max_partition_count: u32,
}

pub struct Partition {
    pub partition_style: PartitionStyle,
    pub starting_offset: i64,
    pub partition_length: i64,
    pub partition_number: u32,
    pub rewrite_partition: bool,
    pub is_service_partition: bool,
    pub extra: PartitionExtra,
}

pub enum PartitionExtra {
    Mbr(PartitionExtraMbr),
    Gpt(PartitionExtraGpt),
}

pub struct PartitionExtraMbr {
    pub partition_type: u8,
    pub boot_indicator: bool,
    pub recognized_partition: bool,
    pub hidden_sectors: u32,
    pub partition_id: uuid::Uuid,
}

pub struct PartitionExtraGpt {
    pub partition_type: uuid::Uuid,
    pub partition_id: uuid::Uuid,
    pub attributes: u64,
    pub name: [u16; 36],
}

#[repr(i32)]
pub enum PartitionStyle {
    GPT = 1i32,
    MBR = 0i32,
    RAW = 2i32,
}

impl From<(DRIVE_LAYOUT_INFORMATION_EX, Vec<PARTITION_INFORMATION_EX>)> for Disk {
    fn from(
        (layout, win_partitions): (DRIVE_LAYOUT_INFORMATION_EX, Vec<PARTITION_INFORMATION_EX>),
    ) -> Self {
        let partitions = win_partitions.into_iter().map(|p| p.into()).collect();

        let extra = match PARTITION_STYLE(layout.PartitionStyle as i32) {
            PARTITION_STYLE_MBR => unsafe {
                DiskExtra::Mbr(DiskExtraMbr {
                    signature: layout.Anonymous.Mbr.Signature,
                    checksum: layout.Anonymous.Mbr.CheckSum,
                })
            },
            PARTITION_STYLE_GPT => {
                unsafe {
                    // Convert GUID to UUID
                    let guid = layout.Anonymous.Gpt.DiskId;
                    let mut bytes = [0u8; 16];
                    bytes[0..4].copy_from_slice(&guid.data1.to_le_bytes());
                    bytes[4..6].copy_from_slice(&guid.data2.to_le_bytes());
                    bytes[6..8].copy_from_slice(&guid.data3.to_le_bytes());
                    bytes[8..16].copy_from_slice(&guid.data4);

                    DiskExtra::Gpt(DiskExtraGpt {
                        disk_id: uuid::Uuid::from_bytes_le(bytes),
                        starting_usable_offset: layout.Anonymous.Gpt.StartingUsableOffset,
                        usable_length: layout.Anonymous.Gpt.UsableLength,
                        max_partition_count: layout.Anonymous.Gpt.MaxPartitionCount,
                    })
                }
            }
            _ => {
                // RAW or unknown, use default MBR
                DiskExtra::Mbr(DiskExtraMbr {
                    signature: 0,
                    checksum: 0,
                })
            }
        };

        Disk {
            partition_style: layout.PartitionStyle,
            partitions,
            extra,
        }
    }
}

impl From<PARTITION_INFORMATION_EX> for Partition {
    fn from(win_partition: PARTITION_INFORMATION_EX) -> Self {
        let partition_style = match win_partition.PartitionStyle {
            PARTITION_STYLE_MBR => PartitionStyle::MBR,
            PARTITION_STYLE_GPT => PartitionStyle::GPT,
            _ => PartitionStyle::RAW,
        };

        let extra = match win_partition.PartitionStyle {
            PARTITION_STYLE_MBR => {
                unsafe {
                    let mbr = win_partition.Anonymous.Mbr;
                    // Generate a partition ID for MBR partitions (they don't have one natively)
                    let partition_id = uuid::Uuid::nil();

                    PartitionExtra::Mbr(PartitionExtraMbr {
                        partition_type: mbr.PartitionType,
                        boot_indicator: mbr.BootIndicator,
                        recognized_partition: mbr.RecognizedPartition,
                        hidden_sectors: mbr.HiddenSectors,
                        partition_id,
                    })
                }
            }
            PARTITION_STYLE_GPT => {
                unsafe {
                    let gpt = win_partition.Anonymous.Gpt;

                    // Convert GUIDs to UUIDs
                    let mut type_bytes = [0u8; 16];
                    type_bytes[0..4].copy_from_slice(&gpt.PartitionType.data1.to_le_bytes());
                    type_bytes[4..6].copy_from_slice(&gpt.PartitionType.data2.to_le_bytes());
                    type_bytes[6..8].copy_from_slice(&gpt.PartitionType.data3.to_le_bytes());
                    type_bytes[8..16].copy_from_slice(&gpt.PartitionType.data4);

                    let mut id_bytes = [0u8; 16];
                    id_bytes[0..4].copy_from_slice(&gpt.PartitionId.data1.to_le_bytes());
                    id_bytes[4..6].copy_from_slice(&gpt.PartitionId.data2.to_le_bytes());
                    id_bytes[6..8].copy_from_slice(&gpt.PartitionId.data3.to_le_bytes());
                    id_bytes[8..16].copy_from_slice(&gpt.PartitionId.data4);

                    PartitionExtra::Gpt(PartitionExtraGpt {
                        partition_type: uuid::Uuid::from_bytes_le(type_bytes),
                        partition_id: uuid::Uuid::from_bytes_le(id_bytes),
                        attributes: gpt.Attributes.0,
                        name: gpt.Name,
                    })
                }
            }
            _ => {
                // RAW or unknown, use default MBR
                PartitionExtra::Mbr(PartitionExtraMbr {
                    partition_type: 0,
                    boot_indicator: false,
                    recognized_partition: false,
                    hidden_sectors: 0,
                    partition_id: uuid::Uuid::nil(),
                })
            }
        };

        Partition {
            partition_style,
            starting_offset: win_partition.StartingOffset,
            partition_length: win_partition.PartitionLength,
            partition_number: win_partition.PartitionNumber,
            rewrite_partition: win_partition.RewritePartition,
            is_service_partition: false, // This field is not available in PARTITION_INFORMATION_EX
            extra,
        }
    }
}
