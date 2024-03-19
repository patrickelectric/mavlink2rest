use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use strum_macros::{EnumIter, EnumString};

#[derive(Debug, Copy, Clone, PartialEq, EnumIter, FromPrimitive)]
pub enum MavlinkFtpOpcode {
    None = 0,
    TerminateSession = 1,
    ResetSessions = 2,
    ListDirectory = 3,
    OpenFileRO = 4,
    ReadFile = 5,
    CreateFile = 6,
    WriteFile = 7,
    RemoveFile = 8,
    CreateDirectory = 9,
    RemoveDirectory = 10,
    OpenFileWO = 11,
    TruncateFile = 12,
    Rename = 13,
    CalcFileCRC32 = 14,
    BurstReadFile = 15,
    Ack = 128,
    Nak = 129,
}

#[derive(Debug, Copy, Clone, PartialEq, EnumIter, EnumString, FromPrimitive)]
pub enum MavlinkFtpNak {
    #[strum(serialize = "No error")]
    None = 0,
    #[strum(serialize = "Unknown failure")]
    Fail = 1,
    #[strum(serialize = "Command failed, Err number sent back")]
    FailErrno = 2,
    #[strum(serialize = "Payload size is invalid")]
    InvalidDataSize = 3,
    #[strum(serialize = "Session is not currently open")]
    InvalidSession = 4,
    #[strum(serialize = "All available sessions are already in use")]
    NoSessionsAvailable = 5,
    #[strum(serialize = "Offset past end of file for ListDirectory and ReadFile commands")]
    EOF = 6,
    #[strum(serialize = "Unknown command / opcode")]
    UnknownCommand = 7,
    #[strum(serialize = "File/directory already exists")]
    FileExists = 8,
    #[strum(serialize = "File/directory is write protected")]
    FileProtected = 9,
    #[strum(serialize = "File/directory not found")]
    FileNotFound = 10,
}

#[derive(Debug)]
pub enum MavlinkFtpResponse {
    None,
    TerminateSession(u8),
    ResetSessions,
    ListDirectory(Vec<EntryInfo>),

    //OpenFileRO(u32, u32),
    //ReadFile(Vec<u8>),
    /*
    CreateFile(u32),
    WriteFile,
    RemoveFile,
    CreateDirectory,
    RemoveDirectory,
    OpenFileWO(u32),
    TruncateFile,
    Rename,
    CalcFileCRC32(u32),
    BurstReadFile(Vec<u8>),
     */
    Ack,
    Nak(MavlinkFtpNak),
}

#[derive(Debug)]
pub struct EntryInfo {
    pub entry_type: EntryType,
    pub name: String,
    pub size: u32,
}

#[derive(Debug)]
pub enum EntryType {
    File,
    Directory,
    Skip,
}

pub fn parse_directory_entry(entry: &str) -> Result<EntryInfo, &'static str> {
    let mut parts = entry.split('\t');
    let temp_filename = parts.next().unwrap();
    let file_type = temp_filename.chars().next();
    let name: String = temp_filename.chars().skip(1).collect();
    let size = parts.next().map(|s| s.parse().unwrap()).unwrap_or(0);

    let entry_type = match file_type {
        Some('F') => EntryType::File,
        Some('D') => EntryType::Directory,
        Some('S') => EntryType::Skip,
        _ => return Err("Invalid entry type"),
    };

    Ok(EntryInfo {
        entry_type,
        name,
        size,
    })
}

#[derive(Debug, Clone)]
pub struct MavlinkFtpPayload {
    // Sequence number for message (0 - 65535)
    pub seq_number: u16,
    // Session id for read/write operations (0 - 255)
    pub session: u8,
    // OpCode (id) for commands and ACK/NAK messages (0 - 255)
    pub opcode: MavlinkFtpOpcode,
    // Depends on OpCode. For Reads/Writes, it's the size of the data transported
    // For NAK, it's the number of bytes used for error information (1 or 2)
    pub size: usize,
    // OpCode (of original message) returned in an ACK or NAK response
    pub req_opcode: MavlinkFtpOpcode,
    // Code to indicate if a burst is complete (1: burst packets complete, 0: more burst packets coming)
    // Only used if req_opcode is BurstReadFile
    pub burst_complete: u8,
    // Padding for 32-bit alignment
    pub padding: u8,
    // Content offset for ListDirectory and ReadFile commands
    pub offset: u32,
    // Command/response data (varies by OpCode)
    pub data: Vec<u8>,
}

impl MavlinkFtpPayload {
    pub fn newResetSesions(seq_number: u16, session: u8) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::ResetSessions,
            size: 0,
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset: 0,
            data: vec![],
        }
    }

    pub fn newTerminateSession(seq_number: u16, session: u8) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::TerminateSession,
            size: 0,
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset: 0,
            data: vec![],
        }
    }

    pub fn newListDirectory(seq_number: u16, session: u8, offset: u32, path: &str) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::ListDirectory,
            size: path.len(),
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset,
            data: path.as_bytes().to_vec(),
        }
    }

    pub fn newOpenFile(seq_number: u16, session: u8, path: &str) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::OpenFileRO,
            size: path.len(),
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset: 0,
            data: path.as_bytes().to_vec(),
        }
    }

    pub fn newReadFile(seq_number: u16, session: u8, offset: u32, size_left: usize) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::BurstReadFile,
            size: size_left.clamp(0, 239), // 239 is the max size on the data field
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset,
            data: vec![],
        }
    }

    pub fn newCalcFileCRC32(seq_number: u16, session: u8, path: &str) -> Self {
        Self {
            seq_number,
            session,
            opcode: MavlinkFtpOpcode::CalcFileCRC32,
            size: path.len(),
            req_opcode: MavlinkFtpOpcode::None,
            burst_complete: 0,
            padding: 0,
            offset: 0,
            data: path.as_bytes().to_vec(),
        }
    }

    /*
    opcode: MavlinkFtpOpcode,
        req_opcode: MavlinkFtpOpcode,
        burst_complete: u8,
        offset: u32,
        data: Vec<u8>,
        */

    // Convert payload structure into a byte array
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.seq_number.to_le_bytes());
        bytes.push(self.session);
        bytes.push(self.opcode as u8);
        bytes.push(self.size as u8);
        bytes.push(self.req_opcode as u8);
        bytes.push(self.burst_complete);
        bytes.push(self.padding);
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes.extend_from_slice(&self.data);

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<MavlinkFtpPayload, Box<dyn std::error::Error>> {
        if bytes.len() < 12 {
            return Err("Insufficient bytes in input array".into());
        }

        Ok(MavlinkFtpPayload {
            seq_number: u16::from_le_bytes([bytes[0], bytes[1]]),
            session: bytes[2],
            opcode: MavlinkFtpOpcode::from_u8(bytes[3]).ok_or("Invalid opcode")?,
            size: bytes[4] as usize,
            req_opcode: MavlinkFtpOpcode::from_u8(bytes[5]).ok_or("Invalid req_opcode")?,
            burst_complete: bytes[6],
            padding: bytes[7],
            offset: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            data: bytes[12..12 + bytes[4] as usize].to_vec(),
        })
    }
}

const CRC32_TABLE: [u32; 256] = [
    0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
    0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
    0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
    0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
    0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
    0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
    0x76dc4190, 0x01db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
    0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
    0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
    0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
    0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
    0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683,
    0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
    0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
    0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
    0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
    0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
    0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21,
    0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
    0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
    0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
    0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
];

pub fn mavlink_crc32(buffer: &[u8]) -> u32 {
    let mut crc = 0;
    for &b in buffer {
        let index = ((crc ^ (b as u32)) & 0xff) as usize;
        let table_value = CRC32_TABLE[index];
        crc = table_value ^ (crc >> 8);
    }

    crc
}
