use crate::bus::Bus;
use crate::instructions::lookup::INSTRUCTIONS;
use crate::instructions::{AddressMode, InstructionType};
use bitflags::bitflags;
use std::cell::RefCell;
use std::rc::Rc;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct Flags6502: u8 {
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

pub struct Cpu6502 {
    bus: Option<Rc<RefCell<Bus>>>,

    /* Registers */
    a: u8,             // Accumulator
    x: u8,             // X register
    y: u8,             // Y register
    stkp: u8,          // Stack Pointer
    pc: u16,           // Program Counter
    status: Flags6502, // Status register

    fetched: u8,

    // Addresses for jump instructions
    addr_abs: u16,
    addr_rel: u16,

    opcode: usize,
    cycles: u8,
}

impl Cpu6502 {
    pub fn new() -> Self {
        Cpu6502 {
            bus: None,
            a: 0x00,
            x: 0x00,
            y: 0x00,
            stkp: 0x00,
            pc: 0x0000,
            status: Flags6502::empty(),

            fetched: 0x00,
            addr_abs: 0x0000,
            addr_rel: 0x0000,

            opcode: 0x00,
            cycles: 0,
        }
    }

    pub fn connect_bus(&mut self, bus: Rc<RefCell<Bus>>) {
        self.bus = Some(bus);
    }

    pub fn read(&self, addr: u16) -> u8 {
        match &self.bus {
            Some(bus) => bus.borrow().read(addr, false),
            None => panic!("No bus connected"),
        }
    }

    fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;

        (hi << 8) | lo
    }

    pub fn write(&self, addr: u16, data: u8) {
        match &self.bus {
            Some(bus) => bus.borrow_mut().write(addr, data),
            None => panic!("No bus connected"),
        }
    }

    /// Pushes a byte onto the stack.
    fn push(&mut self, data: u8) {
        self.write(STACK_BASE_ADDR + self.stkp as u16, data);
        self.stkp -= 1;
    }

    /// Pushes 2 bytes onto the stack.
    fn push_u16(&mut self, data: u16) {
        let hi = (data >> 8) & 0x00FF;
        let lo = data & 0x00FF;

        self.write(STACK_BASE_ADDR + self.stkp as u16, hi as u8);
        self.stkp -= 1;
        self.write(STACK_BASE_ADDR + self.stkp as u16, lo as u8);
        self.stkp -= 1;
    }

    /// Pops a byte from the stack.
    fn pop(&mut self) -> u8 {
        self.stkp += 1;
        self.read(STACK_BASE_ADDR + self.stkp as u16)
    }

    fn pop_u16(&mut self) -> u16 {
        self.stkp += 1;
        let val = self.read_u16(STACK_BASE_ADDR + self.stkp as u16);
        self.stkp += 1;

        val
    }

    fn get_flag(&self, flag: Flags6502) -> bool {
        !(self.status & flag).is_empty()
    }

    fn set_flag(&mut self, flag: Flags6502, value: bool) {
        let mask = if value { flag } else { !flag };
        self.status &= mask;
    }

    fn clock(&mut self) {
        if self.cycles == 0 {
            self.opcode = self.read(self.pc) as usize;

            self.set_flag(Flags6502::U, true);

            self.pc += 1;

            let instruction = &INSTRUCTIONS[self.opcode];
            self.cycles = instruction.cycles;

            type A = AddressMode;
            let additional_cycle_1 = match instruction.address_mode {
                A::Imp => self.imp(),
                A::Imm => self.imm(),
                A::Zp0 => self.zp0(),
                A::Zpx => self.zpx(),
                A::Zpy => self.zpy(),
                A::Rel => self.rel(),
                A::Abs => self.abs(),
                A::Abx => self.abx(),
                A::Aby => self.aby(),
                A::Ind => self.ind(),
                A::Izx => self.izx(),
                A::Izy => self.izy(),
            };

            type I = InstructionType;
            let additional_cycle_2 = match instruction.instruction_type {
                I::Adc => self.adc(),
                I::And => self.and(),
                I::Asl => self.asl(),
                I::Bcc => self.bcc(),
                I::Bcs => self.bcs(),
                I::Beq => self.beq(),
                I::Bit => self.bit(),
                I::Bmi => self.bmi(),
                I::Bne => self.bne(),
                I::Bpl => self.bpl(),
                I::Brk => self.brk(),
                I::Bvc => self.bvc(),
                I::Bvs => self.bvs(),
                I::Clc => self.clc(),
                I::Cld => self.cld(),
                I::Cli => self.cli(),
                I::Clv => self.clv(),
                I::Cmp => self.cmp(),
                I::Cpx => self.cpx(),
                I::Cpy => self.cpy(),
                I::Dec => self.dec(),
                I::Dex => self.dex(),
                I::Dey => self.dey(),
                I::Eor => self.eor(),
                I::Inc => self.inc(),
                I::Inx => self.inx(),
                I::Iny => self.iny(),
                I::Jmp => self.jmp(),
                I::Jsr => self.jsr(),
                I::Lda => self.lda(),
                I::Ldx => self.ldx(),
                I::Ldy => self.ldy(),
                I::Lsr => self.lsr(),
                I::Nop => self.nop(),
                I::Ora => self.ora(),
                I::Pha => self.pha(),
                I::Php => self.php(),
                I::Pla => self.pla(),
                I::Plp => self.plp(),
                I::Rol => self.rol(),
                I::Ror => self.ror(),
                I::Rti => self.rti(),
                I::Rts => self.rts(),
                I::Sbc => self.sbc(),
                I::Sec => self.sec(),
                I::Sed => self.sed(),
                I::Sei => self.sei(),
                I::Sta => self.sta(),
                I::Stx => self.stx(),
                I::Sty => self.sty(),
                I::Tax => self.tax(),
                I::Tay => self.tay(),
                I::Tsx => self.tsx(),
                I::Txa => self.txa(),
                I::Txs => self.txs(),
                I::Tya => self.tya(),
                I::Xxx => self.xxx(),
            };

            self.cycles += additional_cycle_1 & additional_cycle_2;

            self.set_flag(Flags6502::U, true);
        }

        self.cycles -= 1;
    }

    // Addressing modes

    /// Implied addressing mode.
    ///
    /// For operating on the accumulator.
    fn imp(&mut self) -> u8 {
        self.fetched = self.a;

        0
    }

    /// Immediate addressing mode.
    ///
    /// Reads data from the next byte of the instruction.
    fn imm(&mut self) -> u8 {
        self.addr_abs = self.read(self.pc) as u16;
        self.pc += 1;

        0
    }

    /// Zero page addressing mode.
    ///
    /// Reads data from page 0 of memory (0x0000 - 0x00FF).
    fn zp0(&mut self) -> u8 {
        self.addr_abs = self.read(self.pc) as u16;
        self.pc += 1;
        self.addr_abs &= 0x00FF;

        0
    }

    /// Zero page addressing mode with X offset.
    ///
    /// Reads data from page 0 of memory (0x0000 - 0x00FF)
    /// but offset by the value of the X register.
    fn zpx(&mut self) -> u8 {
        self.addr_abs = (self.read(self.pc) + self.x) as u16;
        self.pc += 1;
        self.addr_abs &= 0x00FF;

        0
    }

    /// Zero page addressing mode with Y offset.
    ///
    /// Reads data from page 0 of memory (0x0000 - 0x00FF)
    /// but offset by the value of the Y register.
    fn zpy(&mut self) -> u8 {
        self.addr_abs = (self.read(self.pc) + self.y) as u16;
        self.pc += 1;
        self.addr_abs &= 0x00FF;

        0
    }

    /// Absolute addressing mode.
    ///
    /// Reads data from a 16 bit absolute address.
    fn abs(&mut self) -> u8 {
        let lo = self.read(self.pc) as u16;
        self.pc += 1;
        let hi = self.read(self.pc) as u16;
        self.pc += 1;

        self.addr_abs = (hi << 8) | lo;

        0
    }

    /// Absolute addressing mode with X offset.
    ///
    /// Reads data from a 16 bit absolute addressing
    /// but offset by the value of the X register.
    fn abx(&mut self) -> u8 {
        let lo = self.read(self.pc) as u16;
        self.pc += 1;
        let hi = self.read(self.pc) as u16;
        self.pc += 1;

        self.addr_abs = (hi << 8) | lo;
        self.addr_abs += self.x as u16;

        // May need additional clock cycle if page boundary is crossed
        if (self.addr_abs & 0xFF00) != (hi << 8) {
            1
        } else {
            0
        }
    }

    /// Absolute addressing mode with Y offset.
    ///
    /// Reads data from a 16 bit absolute addressing
    /// but offset by the value of the Y register.
    fn aby(&mut self) -> u8 {
        let lo = self.read(self.pc) as u16;
        self.pc += 1;
        let hi = self.read(self.pc) as u16;
        self.pc += 1;

        self.addr_abs = (hi << 8) | lo;
        self.addr_abs += self.y as u16;

        // May need additional clock cycle if page boundary is crossed
        if (self.addr_abs & 0xFF00) != (hi << 8) {
            1
        } else {
            0
        }
    }

    fn rel(&mut self) -> u8 {
        self.addr_rel = self.read(self.pc) as u16;
        self.pc += 1;

        // If the value represents an 8 bit negative integer,
        // then convert it to a 16 bit negative integer.
        if (self.addr_rel & 0x80) != 0 {
            self.addr_rel |= 0xFF00;
        }

        0
    }

    /// Indirect addressing mode.
    /// Follows a pointer to get the data.
    fn ind(&mut self) -> u8 {
        let ptr_lo = self.read(self.pc) as u16;
        self.pc += 1;
        let ptr_hi = self.read(self.pc) as u16;
        self.pc += 1;

        let ptr = (ptr_hi << 8) | ptr_lo;

        // Simulates the page boundary bug where if the low byte of the supplied
        // address is 0xFF, then the high byte is read from the same page
        if ptr_lo == 0x00FF {
            let data_lo = self.read(ptr) as u16;
            let data_hi = self.read(ptr & 0xFF00) as u16;
            self.addr_abs = (data_hi << 8) | data_lo;
        } else {
            let data_lo = self.read(ptr) as u16;
            let data_hi = self.read(ptr + 1) as u16;
            self.addr_abs = (data_hi << 8) | data_lo;
        }

        0
    }

    /// Indirect addressing mode with X offset.
    /// Follows an 8 bit pointer offset by the value of the X register.
    /// This 8 bit pointer is assumed to be in page 0.
    fn izx(&mut self) -> u8 {
        let ptr = self.read(self.pc) as u16;
        self.pc += 1;

        let lo = self.read((ptr + self.x as u16) & 0x00FF) as u16;
        let hi = self.read((ptr + self.x as u16 + 1) & 0x00FF) as u16;

        self.addr_abs = (hi << 8) | lo;

        0
    }

    /// Indirect addressing mode with Y offset.
    /// Follows an 8 bit pointer, then offsets the underlying data by the value of the Y register.
    fn izy(&mut self) -> u8 {
        let ptr = self.read(self.pc) as u16;
        self.pc += 1;

        let lo = self.read(ptr & 0x00FF) as u16;
        let hi = self.read((ptr + 1) & 0x00FF) as u16;

        self.addr_abs = (hi << 8) | lo;
        self.addr_abs += self.y as u16;

        // May need additional clock cycle if page boundary is crossed
        if (self.addr_abs & 0xFF00) != (hi << 8) {
            1
        } else {
            0
        }
    }

    // Opcodes (instructions)

    /// Addition with carry.
    /// Adds the argument and the accumulator, and the carry bit.
    /// Sets the carry bit if the result is greater than 255.
    /// Sets the zero bit if the result is 0.
    /// Sets the overflow bit if there is an overflow, i.e the accumulator and the argument are
    /// both negative or both positive, but the result is the opposite.
    /// This is determined by computing (A XNOR M) AND (A XOR R), where A, M, and R represent the
    /// most significant bit of the accumulator, the argument, and the result respectively.
    fn adc(&mut self) -> u8 {
        self.fetch();
        let a = self.a as u16;
        let fetched = self.fetched as u16;

        let temp = a + fetched + self.get_flag(Flags6502::C) as u16;
        self.set_flag(Flags6502::C, temp > 255);
        self.set_flag(Flags6502::Z, (temp & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (temp & 0x80) != 0);

        let m = fetched;
        let r = temp;
        let overflow = !(a ^ m) & (a ^ r);
        self.set_flag(Flags6502::V, (overflow & 0x80) != 0);

        self.a = (temp & 0x00FF) as u8;

        1
    }

    /// Performs bitwise AND with the argument and the accumulator.
    fn and(&mut self) -> u8 {
        self.fetch();
        self.a &= self.fetched;
        self.set_flag(Flags6502::Z, self.a == 0x00);
        self.set_flag(Flags6502::N, (self.a & 0x80) != 0);

        1
    }

    fn asl(&mut self) -> u8 {
        self.fetch();
        let val = (self.fetched as u16) << 1;

        self.set_flag(Flags6502::C, val > 255);
        self.set_flag(Flags6502::Z, (val & 0x80) == 0x00);
        self.set_flag(Flags6502::N, (val & 0x80) != 0);

        let res = (val & 0x00FF) as u8;

        match INSTRUCTIONS[self.opcode].address_mode {
            AddressMode::Imp => self.a = res,
            _ => self.write(self.addr_abs, res),
        }

        0
    }

    /// Branch if carry bit is not set.
    fn bcc(&mut self) -> u8 {
        self.branch_if(Flags6502::C, false)
    }

    /// Branch if carry bit set.
    fn bcs(&mut self) -> u8 {
        self.branch_if(Flags6502::C, true)
    }

    /// Branch if equal (zero bit set).
    fn beq(&mut self) -> u8 {
        self.branch_if(Flags6502::Z, true)
    }

    /// Bit test.
    ///
    /// ANDs the argument with the mask in A, and sets flags according to the result.
    fn bit(&mut self) -> u8 {
        self.fetch();
        let val = self.a & self.fetched;

        self.set_flag(Flags6502::Z, val == 0x00);
        self.set_flag(Flags6502::V, (val & (1 << 6)) != 0x00);
        self.set_flag(Flags6502::N, (val & (1 << 7)) != 0x00);

        0
    }

    /// Branch if negative bit set.
    fn bmi(&mut self) -> u8 {
        self.branch_if(Flags6502::N, true)
    }

    /// Branch if not equal (zero bit not set).
    fn bne(&mut self) -> u8 {
        self.branch_if(Flags6502::Z, false)
    }

    /// Branch if positive (negative bit not set).
    fn bpl(&mut self) -> u8 {
        self.branch_if(Flags6502::N, false)
    }

    /// Break.
    /// Forces an interrupt.
    fn brk(&mut self) -> u8 {
        self.pc += 1;

        self.push_u16(self.pc);
        self.push(self.status.bits());

        self.pc = self.read_u16(0xFFFE);

        0
    }

    /// Branch if overflow bit not set.
    fn bvc(&mut self) -> u8 {
        self.branch_if(Flags6502::V, false)
    }

    /// Branch if overflow bit not set.
    fn bvs(&mut self) -> u8 {
        self.branch_if(Flags6502::V, true)
    }

    /// Clear carry bit.
    fn clc(&mut self) -> u8 {
        self.set_flag(Flags6502::C, false);
        0
    }

    /// Clear decimal bit.
    fn cld(&mut self) -> u8 {
        self.set_flag(Flags6502::D, false);
        0
    }

    /// Clear interrupts bit.
    fn cli(&mut self) -> u8 {
        self.set_flag(Flags6502::I, false);
        0
    }

    /// Clear overflow bit.
    fn clv(&mut self) -> u8 {
        self.set_flag(Flags6502::V, false);
        0
    }

    /// Compare accumulator with argument.
    fn cmp(&mut self) -> u8 {
        self.fetch();

        let res = self.a as u16 - self.fetched as u16;

        self.set_flag(Flags6502::C, self.a >= self.fetched);
        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        1
    }

    /// Compare X register with argument.
    fn cpx(&mut self) -> u8 {
        self.fetch();

        let res = self.x as u16 - self.fetched as u16;

        self.set_flag(Flags6502::C, self.x >= self.fetched);
        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        0
    }

    /// Compare X register with argument.
    fn cpy(&mut self) -> u8 {
        self.fetch();

        let res = self.y as u16 - self.fetched as u16;

        self.set_flag(Flags6502::C, self.y >= self.fetched);
        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        0
    }

    /// Decrement argument.
    fn dec(&mut self) -> u8 {
        self.fetch();

        let res = self.fetched as u16 - 1;
        self.write(self.addr_abs, (res & 0x00FF) as u8);

        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        0
    }

    /// Decrement X register.
    fn dex(&mut self) -> u8 {
        self.x -= 1;

        self.set_flag(Flags6502::Z, self.x == 0);
        self.set_flag(Flags6502::N, (self.x & 0x80) != 0);

        0
    }

    /// Decrement Y register.
    fn dey(&mut self) -> u8 {
        self.y -= 1;

        self.set_flag(Flags6502::Z, self.y == 0);
        self.set_flag(Flags6502::N, (self.y & 0x80) != 0);

        0
    }

    /// Bitwise exclusive or between accumulator and argument.
    fn eor(&mut self) -> u8 {
        self.fetch();

        self.a ^= self.fetched;

        self.set_flag(Flags6502::Z, self.a == 0);
        self.set_flag(Flags6502::N, (self.a & 0x80) != 0);

        1
    }

    /// Increment the argument by 1.
    fn inc(&mut self) -> u8 {
        self.fetch();

        let res = self.fetched as u16 + 1;
        self.write(self.addr_abs, (res & 0x00FF) as u8);

        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        0
    }

    /// Increment the X register by 1.
    fn inx(&mut self) -> u8 {
        self.x += 1;

        self.set_flag(Flags6502::Z, self.x == 0);
        self.set_flag(Flags6502::N, (self.x & 0x80) != 0);

        0
    }

    /// Increment the Y register by 1.
    fn iny(&mut self) -> u8 {
        self.y += 1;

        self.set_flag(Flags6502::Z, self.y == 0);
        self.set_flag(Flags6502::N, (self.y & 0x80) != 0);

        0
    }

    /// Jump to an absolute address.
    fn jmp(&mut self) -> u8 {
        self.pc = self.addr_abs;

        0
    }

    /// Jump to a subroutine.
    fn jsr(&mut self) -> u8 {
        self.fetch();

        self.pc -= 1;
        self.push_u16(self.pc);

        self.pc = self.addr_abs;

        0
    }

    /// Load byte to accumulator.
    fn lda(&mut self) -> u8 {
        self.fetch();

        self.a = self.fetched;

        self.set_flag(Flags6502::Z, self.a == 0);
        self.set_flag(Flags6502::N, (self.a & 0x80) != 0);

        1
    }

    /// Load byte to X register.
    fn ldx(&mut self) -> u8 {
        self.fetch();

        self.x = self.fetched;

        self.set_flag(Flags6502::Z, self.x == 0);
        self.set_flag(Flags6502::N, (self.x & 0x80) != 0);

        1
    }

    /// Load byte to Y register.
    fn ldy(&mut self) -> u8 {
        self.fetch();

        self.y = self.fetched;

        self.set_flag(Flags6502::Z, self.y == 0);
        self.set_flag(Flags6502::N, (self.y & 0x80) != 0);

        1
    }

    /// Logical shift right.
    fn lsr(&mut self) -> u8 {
        self.fetch();
        self.set_flag(Flags6502::C, (self.fetched & 0x0001) != 0);
        let res = (self.fetched as u16) >> 1;
        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        match INSTRUCTIONS[self.opcode].address_mode {
            AddressMode::Imp => self.a = (res & 0x00FF) as u8,
            _ => self.write(self.addr_abs, (res & 0x00FF) as u8),
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
    fn ora(&mut self) -> u8 {
        self.fetch();
        self.a |= self.fetched;

        self.set_flag(Flags6502::Z, self.a == 0);
        self.set_flag(Flags6502::N, (self.a & 0x80) == 0);

        1
    }

    /// Push the accumulator onto the stack.
    fn pha(&mut self) -> u8 {
        self.push(self.a);

        0
    }

    /// Push status register onto the stack.
    fn php(&mut self) -> u8 {
        self.push(self.status.bits());

        0
    }

    /// Pop the accumulator from the stack.
    fn pla(&mut self) -> u8 {
        self.stkp += 1;
        self.a = self.read(STACK_BASE_ADDR + self.stkp as u16);
        self.set_flag(Flags6502::Z, self.a == 0x00);
        self.set_flag(Flags6502::N, (self.a & 0x80) != 0);

        0
    }

    /// Pop status from the stack.
    fn plp(&mut self) -> u8 {
        self.status = Flags6502::from_bits(self.pop())
            .expect("Invalid status register state popped from stack");

        0
    }

    /// Rotate left.
    fn rol(&mut self) -> u8 {
        self.fetch();
        let res = ((self.fetched as u16) << 1) | self.get_flag(Flags6502::C) as u16;

        self.set_flag(Flags6502::C, res > 255);
        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        let res = (res & 0x00FF) as u8;

        match INSTRUCTIONS[self.opcode].address_mode {
            AddressMode::Imp => self.a = res,
            _ => self.write(self.addr_abs, res),
        }

        0
    }

    /// Rotate right.
    fn ror(&mut self) -> u8 {
        self.fetch();

        let res = ((self.fetched as u16) >> 1) | ((self.get_flag(Flags6502::C) as u16) << 7);

        self.set_flag(Flags6502::C, (self.fetched & 0x01) != 0);
        self.set_flag(Flags6502::Z, (res & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (res & 0x0080) != 0);

        let res = (res & 0x00FF) as u8;

        match INSTRUCTIONS[self.opcode].address_mode {
            AddressMode::Imp => self.a = res,
            _ => self.write(self.addr_abs, res),
        }

        0
    }

    /// Return from interrupt.
    fn rti(&mut self) -> u8 {
        self.stkp += 1;
        self.status = Flags6502::from_bits(self.read(STACK_BASE_ADDR + self.stkp as u16))
            .expect("Invalid flags read from memory when returning from interrupt.");
        self.status.remove(Flags6502::B);
        self.status.remove(Flags6502::U);

        self.stkp += 1;
        self.pc = self.read_u16(STACK_BASE_ADDR + self.stkp as u16);
        self.stkp += 1;

        0
    }

    /// Return from subroutine.
    fn rts(&mut self) -> u8 {
        self.pc = self.pop_u16();
        self.pc += 1;

        0
    }

    /// Subtraction with carry.
    fn sbc(&mut self) -> u8 {
        self.fetch();
        let a = self.a as u16;

        // Invert the bits first for subtraction
        let val: u16 = (self.fetched as u16) ^ 0x00FF;

        // Rest is the same as addition
        let temp = a + val + self.get_flag(Flags6502::C) as u16;
        self.set_flag(Flags6502::C, temp > 255);
        self.set_flag(Flags6502::Z, (temp & 0x00FF) == 0);
        self.set_flag(Flags6502::N, (temp & 0x80) != 0);

        let m = val;
        let r = temp;
        let overflow = !(a ^ m) & (a ^ r);
        self.set_flag(Flags6502::V, (overflow & 0x80) != 0);

        self.a = (temp & 0x00FF) as u8;

        1
    }

    /// Set carry.
    fn sec(&mut self) -> u8 {
        self.set_flag(Flags6502::C, true);

        0
    }

    /// Set decimal.
    fn sed(&mut self) -> u8 {
        self.set_flag(Flags6502::D, true);

        0
    }

    /// Set disable interrupts.
    fn sei(&mut self) -> u8 {
        self.set_flag(Flags6502::I, true);

        0
    }

    /// Store accumulator in memory.
    fn sta(&self) -> u8 {
        self.write(self.addr_abs, self.a);

        0
    }

    /// Store X register in memory.
    fn stx(&self) -> u8 {
        self.write(self.addr_abs, self.x);

        0
    }

    /// Store Y register in memory.
    fn sty(&self) -> u8 {
        self.write(self.addr_abs, self.y);

        0
    }

    /// Transfer X register to accumulator.
    fn tax(&mut self) -> u8 {
        self.x = self.a;

        self.set_flag(Flags6502::Z, self.x == 0);
        self.set_flag(Flags6502::N, (self.x & 0x80) != 0);

        0
    }

    /// Transfer Y register to accumulator.
    fn tay(&mut self) -> u8 {
        self.y = self.a;

        self.set_flag(Flags6502::Z, self.y == 0);
        self.set_flag(Flags6502::N, (self.y & 0x80) != 0);

        0
    }

    /// Transfer stack pointer to X register.
    fn tsx(&mut self) -> u8 {
        self.x = self.stkp;

        self.set_flag(Flags6502::Z, self.x == 0);
        self.set_flag(Flags6502::N, (self.x & 0x80) != 0);

        0
    }

    /// Transfer stack pointer to accumulator.
    fn txa(&mut self) -> u8 {
        self.a = self.stkp;

        self.set_flag(Flags6502::Z, self.a == 0);
        self.set_flag(Flags6502::N, (self.a & 0x80) != 0);

        0
    }

    /// Transfer X register to stack pointer.
    fn txs(&mut self) -> u8 {
        self.stkp = self.x;

        self.set_flag(Flags6502::Z, self.stkp == 0);
        self.set_flag(Flags6502::N, (self.stkp & 0x80) != 0);

        0
    }

    fn tya(&mut self) -> u8 {
        self.a = self.y;

        self.set_flag(Flags6502::Z, self.a == 0);
        self.set_flag(Flags6502::N, (self.a & 0x80) != 0);

        0
    }

    // invalid opcode
    fn xxx(&self) -> u8 {
        0
    }

    // interrupts
    fn reset(&mut self) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.stkp = 0xFD;
        self.status = Flags6502::empty() | Flags6502::U;

        self.addr_abs = 0xFFFC;

        self.pc = self.read_u16(self.addr_abs);

        self.addr_rel = 0x0000;
        self.addr_abs = 0x0000;
        self.fetched = 0x00;

        self.cycles = 8;
    }

    /// Interrupt request.
    fn irq(&mut self) {
        if self.get_flag(Flags6502::I) {
            return;
        }

        self.interrupt(0xFFFE, 7);
    }

    /// Non-maskable interrupt, can't be disabled
    fn nmi(&mut self) {
        self.interrupt(0xFFFA, 8);
    }

    fn interrupt(&mut self, interrupt_addr: u16, cycles: u8) {
        self.push_u16(self.pc);

        self.set_flag(Flags6502::B, false);
        self.set_flag(Flags6502::U, true);
        self.set_flag(Flags6502::I, true);
        self.write(STACK_BASE_ADDR + self.stkp as u16, self.status.bits());
        self.stkp -= 1;

        // Get the interrupt address to jump to
        self.addr_abs = interrupt_addr;
        self.pc = self.read_u16(self.addr_abs);

        self.cycles = cycles;
    }

    /// Gets the data from the current address to fetch from.
    fn fetch(&mut self) -> u8 {
        match INSTRUCTIONS[self.opcode].address_mode {
            AddressMode::Imp => self.fetched,
            _ => {
                self.fetched = self.read(self.addr_abs);
                self.fetched
            }
        }
    }

    fn branch_if(&mut self, flag: Flags6502, set: bool) -> u8 {
        if self.get_flag(flag) {
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            // Additional cycle if page boundary crossed
            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00) {
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }

        0
    }
}
