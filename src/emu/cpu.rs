use super::bits::IntoBit;
use super::cartridge::Cartridge;
use super::input::ControllerButtons;
use super::instructions::{AddressMode, Instruction, InstructionType};
use super::ppu::Ppu;
use bitflags::bitflags;
use std::cell::RefCell;
use std::rc::Rc;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct StatusFlags: u8 {
        /// Carry Bit
        const C = 1 << 0;
        /// Zero
        const Z = 1 << 1;
        /// Disable interrupts
        const I = 1 << 2;
        /// Decimal mode
        const D = 1 << 3;
        /// Break
        const B = 1 << 4;
        /// Unused
        const U = 1 << 5;
        /// Overflow
        const V = 1 << 6;
        /// Negative
        const N = 1 << 7;
    }
}

const STACK_BASE_ADDR: u16 = 0x0100;
const CPU_RAM_SIZE: usize = 2 * 1024;
const PAGE_SIZE: u16 = 0x0100;

pub struct Cpu6502 {
    /* Registers */
    a: u8,               // Accumulator
    x: u8,               // X register
    y: u8,               // Y register
    sp: u8,              // Stack Pointer
    pc: u16,             // Program Counter
    status: StatusFlags, // Status register

    opcode: u8,
    cycles: u8,

    total_cycles: u64,

    // Indicates whether or not the CPU is in the middle of performing
    // a DMA transfer to the PPU
    dma_transfer: bool,
    // Whether or not we are waiting to start the DMA
    // Always waits on the first cycle that DMA is triggered,
    // then optionally for another alignment cycle
    // See: https://www.nesdev.org/wiki/DMA
    dma_halting: bool,
    // For reading/writing during DMA
    dma_page: u8,
    dma_index: u8,
    dma_data: u8,

    // Memory
    ram: [u8; CPU_RAM_SIZE],

    // Other components
    ppu: Option<Rc<RefCell<Ppu>>>,
    cartridge: Option<Rc<RefCell<Cartridge>>>,

    // Input
    controller: ControllerButtons,
    controller_shift_reg: u8,
}

struct AddressModeResult {
    /// The pointer that was used to get the computed address, used for debugging
    #[allow(dead_code)]
    ptr: Option<u16>,
    /// The computed address to read from
    addr: u16,
    /// Whether or not the addressing mode can lead to additional clock cycles
    additional_cycles: bool,
}

impl Cpu6502 {
    pub fn new() -> Self {
        Cpu6502 {
            a: 0x00,
            x: 0x00,
            y: 0x00,
            sp: 0x00,
            pc: 0x0000,
            status: StatusFlags::empty(),
            opcode: 0x00,
            cycles: 0,
            total_cycles: 0,

            dma_transfer: false,
            dma_halting: false,
            dma_page: 0x00,
            dma_index: 0x00,
            dma_data: 0x00,

            ram: [0; CPU_RAM_SIZE],

            cartridge: None,
            ppu: None,

            controller: ControllerButtons::empty(),
            controller_shift_reg: 0x00,
        }
    }

    pub fn with_ppu(&mut self, ppu: Rc<RefCell<Ppu>>) {
        self.ppu = Some(ppu);
    }

    pub fn load_cartridge(&mut self, cartridge: Rc<RefCell<Cartridge>>) {
        self.cartridge = Some(cartridge);
    }

    #[cfg(test)]
    fn next_instruction(&mut self) {
        self.total_cycles += self.cycles as u64;
        self.cycles = 0;
        self.clock();
    }

    pub fn a(&self) -> u8 {
        self.a
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    pub fn stkp(&self) -> u8 {
        self.sp
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn status(&self) -> StatusFlags {
        self.status
    }

    pub fn opcode(&self) -> u8 {
        self.opcode
    }

    pub fn cycles(&self) -> u8 {
        self.cycles
    }

    pub fn total_cycles(&self) -> u64 {
        self.total_cycles
    }

    pub fn trigger_inputs(&mut self, buttons: ControllerButtons) {
        self.controller = buttons;
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                // Actual RAM is from 0x0000 to 0x07FF, but it is mirrored
                // for the rest of the address range
                let mapped_addr = addr as usize & 0x07FF;
                self.ram[mapped_addr]
            }
            0x2000..=0x3FFF => match &self.ppu {
                Some(ppu) => ppu.borrow_mut().cpu_read(addr),
                None => panic!("PPU not attached"),
            },
            0x4016..=0x4017 => {
                let out = self.controller_shift_reg & 0x01;
                self.controller_shift_reg >>= 1;
                out
            }
            0x4020..=0xFFFF => match &self.cartridge {
                Some(cartridge) => cartridge.borrow_mut().cpu_read(addr).unwrap_or(0),
                None => panic!("Cartridge not attached"),
            },
            _ => 0,
            // _ => todo!("Reading from CPU address {:04X} not implemented yet", addr),
        }
    }

    pub fn read_debug(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                // Actual RAM is from 0x0000 to 0x07FF, but it is mirrored
                // for the rest of the address range
                let mapped_addr = addr as usize & 0x07FF;
                self.ram[mapped_addr]
            }
            0x2000..=0x3FFF => match &self.ppu {
                Some(ppu) => ppu.borrow().cpu_read_debug(addr),
                None => panic!("PPU not attached"),
            },
            0x4016..=0x4017 => self.controller_shift_reg & 0x01,
            0x4020..=0xFFFF => match &self.cartridge {
                Some(cartridge) => cartridge.borrow_mut().cpu_read(addr).unwrap_or(0),
                None => panic!("Cartridge not attached"),
            },
            _ => 0,
            // _ => todo!("Reading from CPU address {:04X} not implemented yet", addr),
        }
    }

    pub fn read_u16(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;

        (hi << 8) | lo
    }

    pub fn read_debug_u16(&self, addr: u16) -> u16 {
        let lo = self.read_debug(addr) as u16;
        let hi = self.read_debug(addr + 1) as u16;

        (hi << 8) | lo
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let mapped_addr = addr as usize % CPU_RAM_SIZE;
                self.ram[mapped_addr] = data;
            }
            0x2000..=0x3FFF => match &self.ppu {
                Some(ppu) => ppu.borrow_mut().cpu_write(addr, data),
                None => panic!("PPU not attached"),
            },
            0x4014 => {
                self.dma_transfer = true;
                self.dma_halting = true;
                self.dma_page = data;
                self.dma_index = 0x00;
            }
            0x4016..=0x4017 => self.controller_shift_reg = self.controller.bits(),
            0x4020..=0xFFFF => match &self.cartridge {
                Some(cartridge) => cartridge.borrow_mut().cpu_write(addr, data).unwrap_or(()),
                None => panic!("Cartridge not attached"),
            },
            _ => {}
        }
    }

    /// Pushes a byte onto the stack.
    fn push(&mut self, data: u8) {
        self.write(STACK_BASE_ADDR + self.sp as u16, data);
        self.sp = self.sp.wrapping_sub(1);
    }

    /// Pushes 2 bytes onto the stack.
    fn push_u16(&mut self, data: u16) {
        let hi = (data >> 8) & 0x00FF;
        let lo = data & 0x00FF;

        let stack_addr = STACK_BASE_ADDR + self.sp as u16;
        self.write(stack_addr, hi as u8);
        self.sp = self.sp.wrapping_sub(1);

        let stack_addr = STACK_BASE_ADDR + self.sp as u16;
        self.write(stack_addr, lo as u8);
        self.sp = self.sp.wrapping_sub(1);
    }

    /// Pops a byte from the stack.
    fn pop(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        self.read(STACK_BASE_ADDR + self.sp as u16)
    }

    fn pop_u16(&mut self) -> u16 {
        self.sp = self.sp.wrapping_add(1);
        let val = self.read_u16(STACK_BASE_ADDR + self.sp as u16);
        self.sp = self.sp.wrapping_add(1);

        val
    }

    fn get_flag(&self, flag: StatusFlags) -> bool {
        !(self.status & flag).is_empty()
    }

    fn set_flag(&mut self, flag: StatusFlags, value: bool) {
        self.status.set(flag, value);
    }

    fn branch_if(&mut self, addr: u16, flag: StatusFlags, set: bool) -> u8 {
        let mut extra_cycles = 0;
        if self.get_flag(flag) == set {
            extra_cycles += 1;

            // Additional cycle if page boundary crossed
            if (addr & 0xFF00) != (self.pc & 0xFF00) {
                extra_cycles += 1;
            }

            self.pc = addr;
        }

        extra_cycles
    }

    fn interrupt(&mut self, interrupt_addr: u16, cycles: u8) {
        self.push_u16(self.pc);

        self.set_flag(StatusFlags::B, false);
        self.set_flag(StatusFlags::I, true);
        self.push(self.status.bits());

        // Get the interrupt address to jump to
        self.pc = self.read_u16(interrupt_addr);

        self.cycles = cycles;
    }

    pub fn reset(&mut self) {
        let reset_addr = self.read_u16(0xFFFC);
        self.reset_to(reset_addr);
    }

    pub fn reset_to(&mut self, pc: u16) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = StatusFlags::from_bits(0x24).unwrap();

        self.pc = pc;

        self.cycles = 7;
    }

    /// Run one clock cycle.
    pub fn clock(&mut self) {
        if self.dma_transfer {
            self.dma_clock();
            self.total_cycles += 1;
            return;
        }

        if self.cycles == 0 {
            self.opcode = self.read(self.pc);

            let instruction = Instruction::lookup(self.opcode);
            self.cycles = instruction.cycles;

            let arg_addr = self.pc + 1;

            type A = AddressMode;
            let AddressModeResult {
                ptr: _,
                addr,
                additional_cycles,
            } = match instruction.address_mode {
                A::Imp => self.imp(),
                A::Acc => self.acc(),
                A::Imm => self.imm(arg_addr),
                A::Zp0 => self.zp0(arg_addr),
                A::Zpx => self.zpx(arg_addr),
                A::Zpy => self.zpy(arg_addr),
                A::Rel => self.rel(arg_addr),
                A::Abs => self.abs(arg_addr),
                A::Abx => self.abx(arg_addr),
                A::Aby => self.aby(arg_addr),
                A::Ind => self.ind(arg_addr),
                A::Izx => self.izx(arg_addr),
                A::Izy => self.izy(arg_addr),
            };

            // Add the size in bytes of the instruction (1, 2, or 3) to the program counter
            self.pc += 1 + instruction.address_mode.arg_size();

            use InstructionType::*;
            let extra_cycle_count = match instruction.instruction_type {
                Adc => self.adc(addr),
                And => self.and(addr),
                Asl => self.asl(addr, instruction.address_mode),
                Bcc => self.bcc(addr),
                Bcs => self.bcs(addr),
                Beq => self.beq(addr),
                Bit => self.bit(addr),
                Bmi => self.bmi(addr),
                Bne => self.bne(addr),
                Bpl => self.bpl(addr),
                Brk => self.brk(),
                Bvc => self.bvc(addr),
                Bvs => self.bvs(addr),
                Clc => self.clc(),
                Cld => self.cld(),
                Cli => self.cli(),
                Clv => self.clv(),
                Cmp => self.cmp(addr),
                Cpx => self.cpx(addr),
                Cpy => self.cpy(addr),
                Dec => self.dec(addr),
                Dex => self.dex(),
                Dey => self.dey(),
                Eor => self.eor(addr),
                Inc => self.inc(addr),
                Inx => self.inx(),
                Iny => self.iny(),
                Jmp => self.jmp(addr),
                Jsr => self.jsr(addr),
                Lda => self.lda(addr),
                Ldx => self.ldx(addr),
                Ldy => self.ldy(addr),
                Lsr => self.lsr(addr, instruction.address_mode),
                Nop => self.nop(),
                Ora => self.ora(addr),
                Pha => self.pha(),
                Php => self.php(),
                Pla => self.pla(),
                Plp => self.plp(),
                Rol => self.rol(addr, instruction.address_mode),
                Ror => self.ror(addr, instruction.address_mode),
                Rti => self.rti(),
                Rts => self.rts(),
                Sbc => self.sbc(addr),
                Sec => self.sec(),
                Sed => self.sed(),
                Sei => self.sei(),
                Sta => self.sta(addr),
                Stx => self.stx(addr),
                Sty => self.sty(addr),
                Tax => self.tax(),
                Tay => self.tay(),
                Tsx => self.tsx(),
                Txa => self.txa(),
                Txs => self.txs(),
                Tya => self.tya(),
                // Illegal opcodes
                Slo => self.slo(addr, instruction.address_mode),
                Rla => self.rla(addr, instruction.address_mode),
                Sre => self.sre(addr, instruction.address_mode),
                Rra => self.rra(addr, instruction.address_mode),
                Sax => self.sax(addr),
                Lax => self.lax(addr),
                Dcp => self.dcp(addr),
                Isc => self.isc(addr),
                Anc => self.anc(addr),
                Alr => self.alr(addr),
                Arr => self.arr(addr),
                Xaa => self.xaa(addr),
                Axs => self.axs(addr),
                Ahx => self.ahx(),
                Shy => self.shy(addr, arg_addr, additional_cycles),
                Shx => self.shx(addr, arg_addr, additional_cycles),
                Tas => self.tas(addr, arg_addr),
                Las => self.las(),
                Stp => self.stp(),
            };

            if additional_cycles {
                self.cycles += extra_cycle_count;
            }
        }
        self.cycles -= 1;

        self.total_cycles += 1;
    }

    fn dma_clock(&mut self) {
        if self.dma_halting {
            // If we're on an even cycle, wait again the next cycle so that
            // we start the DMA transfer on an even cycle
            self.dma_halting = self.total_cycles % 2 == 0;
            return;
        }

        // Even cycles: Get from CPU Page (don't have to do anything in code
        // Odd cycles: Put (write) to PPU OAM
        if self.total_cycles % 2 == 0 {
            let page_base_addr = (self.dma_page as u16) << 8;
            let addr = page_base_addr + (self.dma_index as u16);
            self.dma_data = self.read(addr);
        } else {
            match &self.ppu {
                Some(ppu) => ppu
                    .borrow_mut()
                    .dma_oam_write(self.dma_index, self.dma_data),
                None => panic!("Attempted to perform DMA without PPU attached to CPU"),
            }

            if self.dma_index == 0xFF {
                self.dma_transfer = false;
            } else {
                self.dma_index += 1;
            }
        }
    }

    // Addressing modes
    // See https://www.nesdev.org/obelisk-6502-guide/addressing.html for details

    /// Implied addressing mode.
    ///
    /// For instructions with no arguments.
    fn imp(&self) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: 0,
            additional_cycles: false,
        }
    }

    fn acc(&self) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: 0,
            additional_cycles: false,
        }
    }

    /// Immediate addressing mode.
    ///
    /// Read data from the next byte of the instruction.
    fn imm(&self, arg_addr: u16) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: arg_addr,
            additional_cycles: false,
        }
    }

    /// Zero page addressing mode.
    ///
    /// Reads data from page 0 of memory (0x0000 - 0x00FF).
    fn zp0(&mut self, arg_addr: u16) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: self.read(arg_addr) as u16,
            additional_cycles: false,
        }
    }

    /// Zero page addressing mode with X offset.
    ///
    /// Reads data from page 0 of memory (0x0000 - 0x00FF)
    /// but offset by the value of the X register.
    fn zpx(&mut self, arg_addr: u16) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: self.read(arg_addr).wrapping_add(self.x) as u16,
            additional_cycles: false,
        }
    }

    /// Zero page addressing mode with Y offset.
    ///
    /// Reads data from page 0 of memory (0x0000 - 0x00FF)
    /// but offset by the value of the Y register.
    fn zpy(&mut self, arg_addr: u16) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: self.read(arg_addr).wrapping_add(self.y) as u16,
            additional_cycles: false,
        }
    }

    /// Absolute addressing mode.
    ///
    /// Reads data from a 16 bit absolute address.
    fn abs(&mut self, arg_addr: u16) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: self.read_u16(arg_addr),
            additional_cycles: false,
        }
    }

    /// Absolute addressing mode with X offset.
    ///
    /// Reads data from a 16 bit absolute addressing
    /// but offset by the value of the X register.
    fn abx(&mut self, arg_addr: u16) -> AddressModeResult {
        let lo = self.read(arg_addr) as u16;
        let hi = self.read(arg_addr + 1) as u16;

        let addr = ((hi << 8) | lo).wrapping_add(self.x as u16);

        // Additional clock cycle if page boundary is crossed
        let additional_cycles = (addr & 0xFF00) != (hi << 8);

        AddressModeResult {
            ptr: None,
            addr,
            additional_cycles,
        }
    }

    /// Absolute addressing mode with Y offset.
    ///
    /// Reads data from a 16 bit absolute addressing
    /// but offset by the value of the Y register.
    fn aby(&mut self, arg_addr: u16) -> AddressModeResult {
        let lo = self.read(arg_addr) as u16;
        let hi = self.read(arg_addr + 1) as u16;

        let addr = ((hi << 8) | lo).wrapping_add(self.y as u16);

        // Additional clock cycle if page boundary is crossed
        let additional_cycles = (addr & 0xFF00) != (hi << 8);

        AddressModeResult {
            ptr: None,
            addr,
            additional_cycles,
        }
    }

    /// Relative addressing mode.
    ///
    /// Uses a signed byte offset from the current program counter.
    /// This is only used by branch instructions.
    fn rel(&mut self, arg_addr: u16) -> AddressModeResult {
        let offset = self.read(arg_addr) as i8;

        let addr = if offset < 0 {
            arg_addr + 1 - (offset.unsigned_abs() as u16)
        } else {
            arg_addr + 1 + offset as u16
        };

        AddressModeResult {
            ptr: None,
            addr,
            additional_cycles: true,
        }
    }

    /// Indirect addressing mode.
    /// Follows a pointer to get the data.
    fn ind(&mut self, arg_addr: u16) -> AddressModeResult {
        let ptr_lo = self.read(arg_addr) as u16;
        let ptr_hi = self.read(arg_addr + 1) as u16;

        let ptr = (ptr_hi << 8) | ptr_lo;

        // Simulates the page boundary bug where if the low byte of the supplied
        // address is 0xFF, then the high byte is read from the same page
        let (a1, a2) = if ptr_lo == 0x00FF {
            (ptr, ptr & 0xFF00)
        } else {
            (ptr, ptr + 1)
        };

        let lo = self.read(a1) as u16;
        let hi = self.read(a2) as u16;

        let addr = (hi << 8) | lo;

        AddressModeResult {
            ptr: Some(ptr),
            addr,
            additional_cycles: false,
        }
    }

    /// Indirect addressing mode with X offset.
    /// Dereferences a zero page pointer offset by the value of the X register.
    fn izx(&mut self, arg_addr: u16) -> AddressModeResult {
        let ptr = self.read(arg_addr).wrapping_add(self.x);
        let lo = self.read(ptr as u16) as u16;
        let hi = self.read(ptr.wrapping_add(1) as u16) as u16;

        let addr = (hi << 8) | lo;

        AddressModeResult {
            ptr: Some(ptr as u16),
            addr,
            additional_cycles: false,
        }
    }

    /// Indirect addressing mode with Y offset.
    /// Follows an 8 bit pointer, then offsets the underlying data by the value of the Y register.
    fn izy(&mut self, arg_addr: u16) -> AddressModeResult {
        let ptr = self.read(arg_addr);

        let lo = self.read(ptr as u16) as u16;
        let hi = self.read(ptr.wrapping_add(1) as u16) as u16;

        let addr = ((hi << 8) | lo).wrapping_add(self.y as u16);

        // May need additional clock cycle if page boundary is crossed
        let additional_cycles = (addr & 0xFF00) != (hi << 8);

        AddressModeResult {
            ptr: Some(ptr as u16),
            addr,
            additional_cycles,
        }
    }

    // Opcodes (instructions)
    // Reference: https://www.nesdev.org/obelisk-6502-guide/reference.html
    // TODO: Add unofficial opcodes

    /// Addition with carry.
    /// Adds the argument and the accumulator, and the carry bit.
    /// Sets the carry bit if the result is greater than 255.
    /// Sets the zero bit if the result is 0.
    /// Sets the overflow bit if there is an overflow, i.e the accumulator and the argument are
    /// both negative or both positive, but the result is the opposite.
    fn adc(&mut self, addr: u16) -> u8 {
        let a = self.a as u16;
        let arg = self.read(addr) as u16;
        let carry = self.get_flag(StatusFlags::C) as u16;

        let temp = a + arg + carry;
        let res = (temp & 0x00FF) as u8;

        self.set_flag(StatusFlags::C, temp > 255);
        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        // Overflow occurs if the arguments have different signs from the result
        let overflow = ((a ^ temp) & (arg ^ temp) & 0x80) != 0;
        self.set_flag(StatusFlags::V, overflow);

        self.a = res;

        1
    }

    /// Performs bitwise AND with the argument and the accumulator.
    fn and(&mut self, addr: u16) -> u8 {
        self.a &= self.read(addr);
        self.set_flag(StatusFlags::Z, self.a == 0);
        self.set_flag(StatusFlags::N, is_negative(self.a));

        1
    }

    /// Arithmetic shift left.
    fn asl(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        let arg = match addr_mode {
            AddressMode::Acc => self.a,
            _ => self.read(addr),
        };
        let val = (arg as u16) << 1;
        let res = (val & 0x00FF) as u8;

        self.set_flag(StatusFlags::C, val > 255);
        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        match addr_mode {
            AddressMode::Acc => self.a = res,
            _ => self.write(addr, res),
        }

        0
    }

    /// Branch if carry bit is not set.
    fn bcc(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::C, false)
    }

    /// Branch if carry bit set.
    fn bcs(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::C, true)
    }

    /// Branch if equal (zero bit set).
    fn beq(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::Z, true)
    }

    /// Bit test.
    ///
    /// ANDs the argument with the mask in A, and sets flags according to the result.
    fn bit(&mut self, addr: u16) -> u8 {
        let arg = self.read(addr);
        let result = self.a & arg;

        self.set_flag(StatusFlags::Z, result == 0);
        self.set_flag(StatusFlags::V, (arg & (1 << 6)) != 0);
        self.set_flag(StatusFlags::N, (arg & (1 << 7)) != 0);

        0
    }

    /// Branch if negative bit set.
    fn bmi(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::N, true)
    }

    /// Branch if not equal (zero bit not set).
    fn bne(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::Z, false)
    }

    /// Branch if positive (negative bit not set).
    fn bpl(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::N, false)
    }

    /// Break.
    /// Forces an interrupt.
    /// TODO: Implement this properly
    /// apparently this should set I to 1?
    fn brk(&mut self) -> u8 {
        self.push_u16(self.pc);
        self.push((self.status | StatusFlags::B | StatusFlags::U).bits());
        self.set_flag(StatusFlags::I, true);

        self.pc = self.read_u16(0xFFFE);

        0
    }

    /// Branch if overflow bit not set.
    fn bvc(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::V, false)
    }

    /// Branch if overflow bit set.
    fn bvs(&mut self, addr: u16) -> u8 {
        self.branch_if(addr, StatusFlags::V, true)
    }

    /// Clear carry bit.
    fn clc(&mut self) -> u8 {
        self.set_flag(StatusFlags::C, false);
        0
    }

    /// Clear decimal bit.
    fn cld(&mut self) -> u8 {
        self.set_flag(StatusFlags::D, false);
        0
    }

    /// Clear interrupts bit.
    fn cli(&mut self) -> u8 {
        self.set_flag(StatusFlags::I, false);
        0
    }

    /// Clear overflow bit.
    fn clv(&mut self) -> u8 {
        self.set_flag(StatusFlags::V, false);
        0
    }

    /// Compare accumulator with argument.
    fn cmp(&mut self, addr: u16) -> u8 {
        let arg = self.read(addr);

        let res = self.a.wrapping_sub(arg);

        self.set_flag(StatusFlags::C, self.a >= arg);
        self.set_flag(StatusFlags::Z, self.a == arg);
        self.set_flag(StatusFlags::N, is_negative(res));

        1
    }

    /// Compare X register with argument.
    fn cpx(&mut self, addr: u16) -> u8 {
        let arg = self.read(addr);
        let res = self.x.wrapping_sub(arg);

        self.set_flag(StatusFlags::C, self.x >= arg);
        self.set_flag(StatusFlags::Z, self.x == arg);
        self.set_flag(StatusFlags::N, is_negative(res));

        0
    }

    /// Compare Y register with argument.
    fn cpy(&mut self, addr: u16) -> u8 {
        let arg = self.read(addr);
        let res = self.y.wrapping_sub(arg);

        self.set_flag(StatusFlags::C, self.y >= arg);
        self.set_flag(StatusFlags::Z, self.y == arg);
        self.set_flag(StatusFlags::N, is_negative(res));

        0
    }

    /// Decrement argument.
    fn dec(&mut self, addr: u16) -> u8 {
        let arg = self.read(addr);

        let res = arg.wrapping_sub(1);
        self.write(addr, res);

        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        0
    }

    /// Decrement X register.
    fn dex(&mut self) -> u8 {
        self.x = self.x.wrapping_sub(1);

        self.set_flag(StatusFlags::Z, self.x == 0);
        self.set_flag(StatusFlags::N, is_negative(self.x));

        0
    }

    /// Decrement Y register.
    fn dey(&mut self) -> u8 {
        self.y = self.y.wrapping_sub(1);

        self.set_flag(StatusFlags::Z, self.y == 0);
        self.set_flag(StatusFlags::N, is_negative(self.y));

        0
    }

    /// Bitwise exclusive or between accumulator and argument.
    fn eor(&mut self, addr: u16) -> u8 {
        self.a ^= self.read(addr);

        self.set_flag(StatusFlags::Z, self.a == 0);
        self.set_flag(StatusFlags::N, is_negative(self.a));

        1
    }

    /// Increment the argument by 1.
    fn inc(&mut self, addr: u16) -> u8 {
        let arg = self.read(addr);

        let res = arg.wrapping_add(1);
        self.write(addr, res);

        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        0
    }

    /// Increment the X register by 1.
    fn inx(&mut self) -> u8 {
        self.x = self.x.wrapping_add(1);

        self.set_flag(StatusFlags::Z, self.x == 0);
        self.set_flag(StatusFlags::N, is_negative(self.x));

        0
    }

    /// Increment the Y register by 1.
    fn iny(&mut self) -> u8 {
        self.y = self.y.wrapping_add(1);

        self.set_flag(StatusFlags::Z, self.y == 0);
        self.set_flag(StatusFlags::N, is_negative(self.y));

        0
    }

    /// Jump to an absolute address.
    fn jmp(&mut self, addr: u16) -> u8 {
        self.pc = addr;

        0
    }

    /// Jump to a subroutine.
    fn jsr(&mut self, addr: u16) -> u8 {
        self.pc -= 1;
        self.push_u16(self.pc);

        self.pc = addr;

        0
    }

    /// Load byte to accumulator.
    fn lda(&mut self, addr: u16) -> u8 {
        self.a = self.read(addr);

        self.set_flag(StatusFlags::Z, self.a == 0);
        self.set_flag(StatusFlags::N, is_negative(self.a));

        1
    }

    /// Load byte to X register.
    fn ldx(&mut self, addr: u16) -> u8 {
        self.x = self.read(addr);

        self.set_flag(StatusFlags::Z, self.x == 0);
        self.set_flag(StatusFlags::N, is_negative(self.x));

        1
    }

    /// Load byte to Y register.
    fn ldy(&mut self, addr: u16) -> u8 {
        self.y = self.read(addr);

        self.set_flag(StatusFlags::Z, self.y == 0);
        self.set_flag(StatusFlags::N, is_negative(self.y));

        1
    }

    /// Logical shift right.
    fn lsr(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        let arg = match addr_mode {
            AddressMode::Acc => self.a,
            _ => self.read(addr),
        };

        self.set_flag(StatusFlags::C, (arg & 0x01) != 0);
        let res = arg >> 1;

        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        match addr_mode {
            AddressMode::Acc => self.a = res,
            _ => self.write(addr, res),
        };

        0
    }

    /// No op.
    fn nop(&self) -> u8 {
        match self.opcode {
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => 1,
            _ => 0,
        }
    }

    /// Bitwise OR between accumulator and argument.
    fn ora(&mut self, addr: u16) -> u8 {
        self.a |= self.read(addr);

        self.set_flag(StatusFlags::Z, self.a == 0);
        self.set_flag(StatusFlags::N, is_negative(self.a));

        1
    }

    /// Push the accumulator onto the stack.
    fn pha(&mut self) -> u8 {
        self.push(self.a);

        0
    }

    /// Push status register onto the stack.
    fn php(&mut self) -> u8 {
        self.push((self.status | StatusFlags::U | StatusFlags::B).bits());

        0
    }

    /// Pop the accumulator from the stack.
    fn pla(&mut self) -> u8 {
        self.a = self.pop();
        self.set_flag(StatusFlags::Z, self.a == 0);
        self.set_flag(StatusFlags::N, is_negative(self.a));

        0
    }

    /// Pop status from the stack.
    fn plp(&mut self) -> u8 {
        self.status = StatusFlags::from_bits(self.pop())
            .expect("Invalid status register state popped from stack");
        self.status.remove(StatusFlags::B);
        self.status.insert(StatusFlags::U);

        0
    }

    /// Rotate left.
    fn rol(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        let arg = match addr_mode {
            AddressMode::Acc => self.a,
            _ => self.read(addr),
        };

        let old_bit7 = (arg & (1 << 7)) != 0;
        let res = (arg << 1) | self.get_flag(StatusFlags::C) as u8;

        self.set_flag(StatusFlags::C, old_bit7);
        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        match addr_mode {
            AddressMode::Acc => self.a = res,
            _ => self.write(addr, res),
        }

        0
    }

    /// Rotate right.
    fn ror(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        let arg = match addr_mode {
            AddressMode::Acc => self.a,
            _ => self.read(addr),
        };

        let old_bit0 = (arg & 0x01) != 0;
        let res = (arg >> 1) | ((self.get_flag(StatusFlags::C) as u8) << 7);

        self.set_flag(StatusFlags::C, old_bit0);
        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        match addr_mode {
            AddressMode::Acc => self.a = res,
            _ => self.write(addr, res),
        }

        0
    }

    /// Return from interrupt.
    fn rti(&mut self) -> u8 {
        self.status = StatusFlags::from_bits(self.pop())
            .expect("Invalid flags read from memory when returning from interrupt.");
        self.status.remove(StatusFlags::B);
        self.status.insert(StatusFlags::U);

        self.pc = self.pop_u16();

        0
    }

    /// Return from subroutine.
    fn rts(&mut self) -> u8 {
        self.pc = self.pop_u16();
        self.pc += 1;

        0
    }

    /// Subtraction with carry.
    fn sbc(&mut self, addr: u16) -> u8 {
        let a = self.a as u16;

        // Just invert the bits of the argument, then do the same thing as addition
        // This is because we have these two equations:
        // A = A - M - (1 - C)      (Subtraction)
        // ~M = -M - 1              (Two's complement)
        //
        // Thus:
        // A + (~M) + C             (Addition)
        // = A + (-M - 1) + C
        // = A - M - 1 + C
        // = A - M - (1 - C)
        let arg = (self.read(addr) ^ 0xFF) as u16;
        let carry = self.get_flag(StatusFlags::C) as u16;

        let temp = a + arg + carry;
        let res = (temp & 0x00FF) as u8;

        self.set_flag(StatusFlags::C, temp > 255);
        self.set_flag(StatusFlags::Z, res == 0);
        self.set_flag(StatusFlags::N, is_negative(res));

        // Overflow occurs if the arguments have different signs from the result
        let overflow = ((a ^ temp) & (arg ^ temp) & 0x80) != 0;
        self.set_flag(StatusFlags::V, overflow);

        self.a = res;

        1
    }

    /// Set carry.
    fn sec(&mut self) -> u8 {
        self.set_flag(StatusFlags::C, true);

        0
    }

    /// Set decimal.
    fn sed(&mut self) -> u8 {
        self.set_flag(StatusFlags::D, true);

        0
    }

    /// Set disable interrupts.
    fn sei(&mut self) -> u8 {
        self.set_flag(StatusFlags::I, true);

        0
    }

    /// Store accumulator in memory.
    fn sta(&mut self, addr: u16) -> u8 {
        self.write(addr, self.a);

        0
    }

    /// Store X register in memory.
    fn stx(&mut self, addr: u16) -> u8 {
        self.write(addr, self.x);

        0
    }

    /// Store Y register in memory.
    fn sty(&mut self, addr: u16) -> u8 {
        self.write(addr, self.y);

        0
    }

    /// Transfer X register to accumulator.
    fn tax(&mut self) -> u8 {
        self.x = self.a;

        self.set_flag(StatusFlags::Z, self.x == 0);
        self.set_flag(StatusFlags::N, is_negative(self.x));

        0
    }

    /// Transfer Y register to accumulator.
    fn tay(&mut self) -> u8 {
        self.y = self.a;

        self.set_flag(StatusFlags::Z, self.y == 0);
        self.set_flag(StatusFlags::N, is_negative(self.y));

        0
    }

    /// Transfer stack pointer to X register.
    fn tsx(&mut self) -> u8 {
        self.x = self.sp;

        self.set_flag(StatusFlags::Z, self.x == 0);
        self.set_flag(StatusFlags::N, is_negative(self.x));

        0
    }

    /// Transfer X to accumulator.
    fn txa(&mut self) -> u8 {
        self.a = self.x;

        self.set_flag(StatusFlags::Z, self.a == 0);
        self.set_flag(StatusFlags::N, is_negative(self.a));

        0
    }

    /// Transfer X register to stack pointer.
    fn txs(&mut self) -> u8 {
        self.sp = self.x;

        0
    }

    fn tya(&mut self) -> u8 {
        self.a = self.y;

        self.set_flag(StatusFlags::Z, self.a == 0);
        self.set_flag(StatusFlags::N, is_negative(self.a));

        0
    }

    // Illegal opcodes
    // Resource: http://www.ffd2.com/fridge/docs/6502-NMOS.extra.opcodes

    fn slo(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        self.asl(addr, addr_mode);
        self.ora(addr)
    }

    fn rla(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        self.rol(addr, addr_mode);
        self.and(addr)
    }

    fn sre(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        self.lsr(addr, addr_mode);
        self.eor(addr)
    }

    fn rra(&mut self, addr: u16, addr_mode: AddressMode) -> u8 {
        self.ror(addr, addr_mode);
        self.adc(addr)
    }

    fn sax(&mut self, addr: u16) -> u8 {
        let res = self.a & self.x;
        self.write(addr, res);

        0
    }

    fn lax(&mut self, addr: u16) -> u8 {
        self.lda(addr);
        self.ldx(addr)
    }

    fn dcp(&mut self, addr: u16) -> u8 {
        self.dec(addr);
        self.cmp(addr)
    }

    fn isc(&mut self, addr: u16) -> u8 {
        self.inc(addr);
        self.sbc(addr)
    }

    fn anc(&mut self, addr: u16) -> u8 {
        let cycles = self.and(addr);
        self.set_flag(StatusFlags::C, self.get_flag(StatusFlags::N));

        cycles
    }

    fn alr(&mut self, addr: u16) -> u8 {
        self.and(addr);
        self.lsr(addr, AddressMode::Acc)
    }

    fn arr(&mut self, addr: u16) -> u8 {
        self.and(addr);
        self.ror(addr, AddressMode::Acc);

        let b5 = (self.a & (1 << 5)).into_bit();
        let b6 = (self.a & (1 << 6)).into_bit();

        self.set_flag(StatusFlags::C, b6 != 0);
        self.set_flag(StatusFlags::V, (b5 ^ b6) != 0);

        0
    }

    fn xaa(&mut self, addr: u16) -> u8 {
        self.txa();
        self.and(addr)
    }

    fn axs(&mut self, addr: u16) -> u8 {
        let arg = self.read(addr);
        let and = self.a & self.x;
        self.x = and.wrapping_sub(arg);

        self.set_flag(StatusFlags::C, and >= arg);
        self.set_flag(StatusFlags::Z, and == arg);
        self.set_flag(StatusFlags::N, is_negative(self.x));

        0
    }

    fn ahx(&mut self) -> u8 {
        0
    }

    // Had to see this thread to get these two instructions to work
    // https://forums.nesdev.org/viewtopic.php?t=8107
    fn shy(&mut self, addr: u16, arg_addr: u16, page_crossed: bool) -> u8 {
        let high = self.read(arg_addr + 1);
        let val = self.y & (high + 1);
        if !page_crossed {
            self.write(addr, val);
        }

        0
    }

    fn shx(&mut self, addr: u16, arg_addr: u16, page_crossed: bool) -> u8 {
        let high = self.read(arg_addr + 1);
        let val = self.x & (high + 1);
        if !page_crossed {
            self.write(addr, val);
        }

        0
    }

    fn tas(&mut self, addr: u16, arg_addr: u16) -> u8 {
        self.sp = self.a & self.x;
        self.and(arg_addr + 1);
        self.sta(addr)
    }

    fn las(&mut self) -> u8 {
        0
    }

    fn stp(&mut self) -> u8 {
        panic!("CPU halted by STP instruction");
    }

    // Interrupts

    /// Interrupt request.
    fn irq(&mut self) {
        if self.get_flag(StatusFlags::I) {
            return;
        }

        self.interrupt(0xFFFE, 7);
    }

    /// Non-maskable interrupt, can't be disabled
    pub fn nmi(&mut self) {
        self.interrupt(0xFFFA, 7);
    }

    // Debug functions
    pub fn get_instruction_repr(&mut self, instruction_addr: u16) -> String {
        let instruction = Instruction::lookup(self.read_debug(instruction_addr));
        let arg_addr = instruction_addr + 1;

        let name = instruction.instruction_type.as_ref().to_uppercase();

        match instruction.address_mode {
            AddressMode::Imp => name,
            AddressMode::Acc => format!("{} A", name),
            AddressMode::Imm => format!("{} #${:02X}", name, self.read_debug(arg_addr)),
            AddressMode::Zp0 => {
                let addr = self.zp0(arg_addr).addr;
                format!("{} ${:02X} = {:02X}", name, addr, self.read_debug(addr))
            }
            AddressMode::Zpx => {
                let addr = self.zpx(arg_addr).addr;
                format!(
                    "{} ${:02X},X @ {:02X} = {:02X}",
                    name,
                    self.read_debug(arg_addr),
                    addr,
                    self.read_debug(addr)
                )
            }
            AddressMode::Zpy => {
                let addr = self.zpy(arg_addr).addr;
                format!(
                    "{} ${:02X},Y @ {:02X} = {:02X}",
                    name,
                    self.read_debug(arg_addr),
                    addr,
                    self.read_debug(addr)
                )
            }
            AddressMode::Rel => format!("{} ${:04X}", name, self.rel(arg_addr).addr),
            AddressMode::Abs => {
                let addr = self.abs(arg_addr).addr;
                match instruction.instruction_type {
                    InstructionType::Jmp | InstructionType::Jsr => {
                        format!("{} ${:04X}", name, addr)
                    }
                    _ => format!("{} ${:04X} = {:02X}", name, addr, self.read_debug(addr)),
                }
            }
            AddressMode::Abx => {
                let addr = self.abx(arg_addr).addr;
                format!(
                    "{} ${:04X},X @ {:04X} = {:02X}",
                    name,
                    self.read_debug_u16(arg_addr),
                    addr,
                    self.read_debug(addr)
                )
            }
            AddressMode::Aby => {
                let addr = self.aby(arg_addr).addr;
                format!(
                    "{} ${:04X},Y @ {:04X} = {:02X}",
                    name,
                    self.read_debug_u16(arg_addr),
                    addr,
                    self.read_debug(addr)
                )
            }
            AddressMode::Ind => {
                let res = self.ind(arg_addr);
                format!("{} (${:04X}) = {:04X}", name, res.ptr.unwrap(), res.addr)
            }
            AddressMode::Izx => {
                let res = self.izx(arg_addr);
                let ptr = res.ptr.unwrap();
                let addr = res.addr;
                format!(
                    "{} (${:02X},X) @ {:02X} = {:04X} = {:02X}",
                    name,
                    self.read_debug(arg_addr),
                    ptr,
                    addr,
                    self.read_debug(addr)
                )
            }
            AddressMode::Izy => {
                let res = self.izy(arg_addr);

                let ptr = self.read_debug(arg_addr);
                let lo = self.read_debug(ptr as u16) as u16;
                let hi = self.read_debug(ptr.wrapping_add(1) as u16) as u16;
                let base_addr = (hi << 8) | lo;

                let addr = res.addr;
                format!(
                    "{} (${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                    name,
                    self.read_debug(arg_addr),
                    base_addr,
                    addr,
                    self.read_debug(addr)
                )
            }
        }
    }

    pub fn get_log_line(&mut self) -> String {
        format!(
            "{:04X} {:02X} {:31} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            self.pc(),
            self.read_debug(self.pc()),
            self.get_instruction_repr(self.pc()),
            self.a(),
            self.x(),
            self.y(),
            self.status().bits(),
            self.stkp(),
            self.total_cycles() + self.cycles() as u64
        )
    }
}

fn is_negative(byte: u8) -> bool {
    (byte & 0x80) != 0
}

#[cfg(test)]
mod test {
    use crate::emu::cartridge::Cartridge;

    use super::*;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
        rc::Rc,
    };

    #[test]
    fn test_adc() {
        let mut cpu = Cpu6502::new();

        // Test that 2 + 3 = 5
        cpu.write(0x1000, 3);
        cpu.write(0x1001, 2);
        cpu.lda(0x1000);
        cpu.sta(0x00);
        cpu.lda(0x1001);
        cpu.adc(0x00);
        cpu.sta(0x00);

        assert_eq!(cpu.a, 5);
        assert_eq!(cpu.read(0x0000), 5);
        assert!(!cpu.get_flag(StatusFlags::Z));
        assert!(!cpu.get_flag(StatusFlags::C));
        assert!(!cpu.get_flag(StatusFlags::V));
    }

    #[test]
    fn test_adc_flags() {
        let mut cpu = Cpu6502::new();

        // Test that the overflow bit is correctly set
        //
        // First check 80 + 70 = 150u8 = -106i8
        // LDA #80
        // STA $00
        // LDA #70
        // ADC $00

        cpu.write(0x1000, 80);
        cpu.write(0x1001, 70);
        cpu.lda(0x1000);
        cpu.sta(0x00);
        cpu.lda(0x1001);
        cpu.adc(0x00);

        assert_eq!(cpu.a, 150);
        assert!(!cpu.get_flag(StatusFlags::Z));
        assert!(!cpu.get_flag(StatusFlags::C));
        assert!(cpu.get_flag(StatusFlags::V));

        cpu.write(0x1002, 255);
        cpu.write(0x1003, 1);
        cpu.lda(0x1002);
        cpu.adc(0x1003);

        assert_eq!(cpu.a, 0);
        assert!(cpu.get_flag(StatusFlags::Z));
        assert!(cpu.get_flag(StatusFlags::C));
        assert!(!cpu.get_flag(StatusFlags::V));
    }

    #[test]
    fn nestest_rom() {
        let mut cpu = Cpu6502::new();

        // This rom tests everything

        // The address of the last test before the one that
        // tests the illegal opcodes
        const LAST_TEST_ADDR: u16 = 0xC6A3;

        let cartridge = Cartridge::new("assets/roms/nestest.nes").unwrap();
        cpu.load_cartridge(Rc::new(RefCell::new(cartridge)));
        let correct_log_file = File::open("assets/roms/nestest.log").unwrap();
        let mut log_reader = BufReader::new(correct_log_file);

        cpu.reset_to(0xC000);

        while cpu.pc() != LAST_TEST_ADDR {
            let mut correct_log_line = String::new();
            log_reader.read_line(&mut correct_log_line).unwrap();
            let log_line = cpu.get_log_line();
            assert_eq!(correct_log_line.trim_end(), log_line);

            cpu.next_instruction();
        }
    }
}
