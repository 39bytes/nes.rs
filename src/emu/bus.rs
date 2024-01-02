use std::fmt::Display;

const RAM_SIZE: usize = 64 * 1024;

pub struct Bus {
    ram: [u8; RAM_SIZE],
}

impl Bus {
    pub fn new() -> Self {
        Bus { ram: [0; RAM_SIZE] }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        self.ram[addr as usize] = data;
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn page_str(&self, page: u8) -> String {
        let page_start = (page as u16) * 0x100;
        // println!(
        //     "Address range: {:#06X} - {:#06X}",
        //     page_start,
        //     page_start + 0xFF
        // );
        let mut s = String::new();
        s.push_str("   ");
        for i in 0..16 {
            s.push_str(&format!("{:X}  ", i));
        }
        s.push('\n');
        for i in 0..16 {
            s.push_str(&format!("{:X}  ", i));
            for j in 0..16 {
                s.push_str(&format!(
                    "{:02X} ",
                    self.ram[(page_start + i * 0x10 + j) as usize]
                ));
            }
            s.push('\n');
        }

        s
    }
}

impl Display for Bus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.page_str(0))
    }
}
