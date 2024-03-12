use super::bus::Bus;
use super::instructions::{AddressMode, Instruction, InstructionType};
use bitflags::bitflags;
use std::cell::RefCell;
use std::rc::Weak;

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

pub struct Cpu6502 {
    bus: Weak<RefCell<Bus>>,

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
}

struct AddressModeResult {
    /// The pointer that was used to get the computed address, used for debugging
    ptr: Option<u16>,
    /// The computed address to read from
    addr: u16,
    /// Whether or not the addressing mode can lead to additional clock cycles
    additional_cycles: bool,
}

impl Cpu6502 {
    pub fn new(bus: Weak<RefCell<Bus>>) -> Self {
        Cpu6502 {
            bus,
            a: 0x00,
            x: 0x00,
            y: 0x00,
            sp: 0x00,
            pc: 0x0000,
            status: StatusFlags::empty(),
            opcode: 0x00,
            cycles: 0,
            total_cycles: 0,
        }
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

    pub fn read(&self, addr: u16) -> u8 {
        match self.bus.upgrade() {
            Some(bus) => bus.borrow().cpu_read(addr),
            None => panic!("Bus not found"),
        }
    }

    pub fn read_u16(&self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;

        (hi << 8) | lo
    }

    fn write(&self, addr: u16, data: u8) {
        match self.bus.upgrade() {
            Some(bus) => bus.borrow_mut().cpu_write(addr, data),
            None => panic!("Bus not found"),
        }
    }

    /// Pushes a byte onto the stack.
    fn push(&mut self, data: u8) {
        self.write(STACK_BASE_ADDR + self.sp as u16, data);
        self.sp -= 1;
    }

    /// Pushes 2 bytes onto the stack.
    fn push_u16(&mut self, data: u16) {
        let hi = (data >> 8) & 0x00FF;
        let lo = data & 0x00FF;

        let stack_addr = STACK_BASE_ADDR + self.sp as u16;
        self.write(stack_addr, hi as u8);
        self.write(stack_addr - 1, lo as u8);
        self.sp -= 2;
    }

    /// Pops a byte from the stack.
    fn pop(&mut self) -> u8 {
        self.sp += 1;
        self.read(STACK_BASE_ADDR + self.sp as u16)
    }

    fn pop_u16(&mut self) -> u16 {
        self.sp += 1;
        let val = self.read_u16(STACK_BASE_ADDR + self.sp as u16);
        self.sp += 1;

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
        self.reset_to(self.read_u16(0xFFFC))
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
        if self.cycles == 0 {
            self.opcode = self.read(self.pc);

            let instruction = Instruction::lookup(self.opcode);
            self.cycles = instruction.cycles;

            type A = AddressMode;
            let AddressModeResult {
                ptr: _,
                addr,
                additional_cycles,
            } = match instruction.address_mode {
                A::Imp => self.imp(),
                A::Acc => self.acc(),
                A::Imm => self.imm(self.pc + 1),
                A::Zp0 => self.zp0(self.pc + 1),
                A::Zpx => self.zpx(self.pc + 1),
                A::Zpy => self.zpy(self.pc + 1),
                A::Rel => self.rel(self.pc + 1),
                A::Abs => self.abs(self.pc + 1),
                A::Abx => self.abx(self.pc + 1),
                A::Aby => self.aby(self.pc + 1),
                A::Ind => self.ind(self.pc + 1),
                A::Izx => self.izx(self.pc + 1),
                A::Izy => self.izy(self.pc + 1),
            };
            // Add the size in bytes of the instruction (1, 2, or 3) to the program counter
            self.pc += 1 + instruction.address_mode.arg_size();

            type I = InstructionType;
            let extra_cycle_count = match instruction.instruction_type {
                I::Adc => self.adc(addr),
                I::And => self.and(addr),
                I::Asl => self.asl(addr),
                I::Bcc => self.bcc(addr),
                I::Bcs => self.bcs(addr),
                I::Beq => self.beq(addr),
                I::Bit => self.bit(addr),
                I::Bmi => self.bmi(addr),
                I::Bne => self.bne(addr),
                I::Bpl => self.bpl(addr),
                I::Brk => self.brk(),
                I::Bvc => self.bvc(addr),
                I::Bvs => self.bvs(addr),
                I::Clc => self.clc(),
                I::Cld => self.cld(),
                I::Cli => self.cli(),
                I::Clv => self.clv(),
                I::Cmp => self.cmp(addr),
                I::Cpx => self.cpx(addr),
                I::Cpy => self.cpy(addr),
                I::Dec => self.dec(addr),
                I::Dex => self.dex(),
                I::Dey => self.dey(),
                I::Eor => self.eor(addr),
                I::Inc => self.inc(addr),
                I::Inx => self.inx(),
                I::Iny => self.iny(),
                I::Jmp => self.jmp(addr),
                I::Jsr => self.jsr(addr),
                I::Lda => self.lda(addr),
                I::Ldx => self.ldx(addr),
                I::Ldy => self.ldy(addr),
                I::Lsr => self.lsr(addr),
                I::Nop => self.nop(),
                I::Ora => self.ora(addr),
                I::Pha => self.pha(),
                I::Php => self.php(),
                I::Pla => self.pla(),
                I::Plp => self.plp(),
                I::Rol => self.rol(addr),
                I::Ror => self.ror(addr),
                I::Rti => self.rti(),
                I::Rts => self.rts(),
                I::Sbc => self.sbc(addr),
                I::Sec => self.sec(),
                I::Sed => self.sed(),
                I::Sei => self.sei(),
                I::Sta => self.sta(addr),
                I::Stx => self.stx(addr),
                I::Sty => self.sty(addr),
                I::Tax => self.tax(),
                I::Tay => self.tay(),
                I::Tsx => self.tsx(),
                I::Txa => self.txa(),
                I::Txs => self.txs(),
                I::Tya => self.tya(),
                I::Xxx => self.xxx(),
            };

            if additional_cycles {
                self.cycles += extra_cycle_count;
            }
        }
        self.cycles -= 1;

        self.total_cycles += 1;
    }

    /// Debug function
    pub fn next_instruction(&mut self) {
        self.total_cycles += self.cycles as u64;
        self.cycles = 0;
        self.clock();
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
    fn zp0(&self, arg_addr: u16) -> AddressModeResult {
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
    fn zpx(&self, arg_addr: u16) -> AddressModeResult {
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
    fn zpy(&self, arg_addr: u16) -> AddressModeResult {
        AddressModeResult {
            ptr: None,
            addr: self.read(arg_addr).wrapping_add(self.y) as u16,
            additional_cycles: false,
        }
    }

    /// Absolute addressing mode.
    ///
    /// Reads data from a 16 bit absolute address.
    fn abs(&self, arg_addr: u16) -> AddressModeResult {
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
    fn abx(&self, arg_addr: u16) -> AddressModeResult {
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
    fn aby(&self, arg_addr: u16) -> AddressModeResult {
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
    fn rel(&self, arg_addr: u16) -> AddressModeResult {
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
    fn ind(&self, arg_addr: u16) -> AddressModeResult {
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
    fn izx(&self, arg_addr: u16) -> AddressModeResult {
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
    fn izy(&self, arg_addr: u16) -> AddressModeResult {
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
    fn asl(&mut self, addr: u16) -> u8 {
        let addr_mode = Instruction::lookup(self.opcode).address_mode;

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
        self.pc += 1;

        self.push_u16(self.pc);
        self.push(self.status.bits());

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
    fn lsr(&mut self, addr: u16) -> u8 {
        let addr_mode = Instruction::lookup(self.opcode).address_mode;
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
    fn rol(&mut self, addr: u16) -> u8 {
        let addr_mode = Instruction::lookup(self.opcode).address_mode;
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
    fn ror(&mut self, addr: u16) -> u8 {
        let addr_mode = Instruction::lookup(self.opcode).address_mode;
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
    fn sta(&self, addr: u16) -> u8 {
        self.write(addr, self.a);

        0
    }

    /// Store X register in memory.
    fn stx(&self, addr: u16) -> u8 {
        self.write(addr, self.x);

        0
    }

    /// Store Y register in memory.
    fn sty(&self, addr: u16) -> u8 {
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

    // invalid opcode
    fn xxx(&self) -> u8 {
        0
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
    fn nmi(&mut self) {
        self.interrupt(0xFFFA, 8);
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

    fn get_instruction_repr(cpu: &Cpu6502, instruction_addr: u16) -> String {
        let instruction = Instruction::lookup(cpu.read(instruction_addr));
        let arg_addr = instruction_addr + 1;

        let name = instruction.instruction_type.as_ref().to_uppercase();

        match instruction.address_mode {
            AddressMode::Imp => name,
            AddressMode::Acc => format!("{} A", name),
            AddressMode::Imm => format!("{} #${:02X}", name, cpu.read(arg_addr)),
            AddressMode::Zp0 => {
                let addr = cpu.zp0(arg_addr).addr;
                format!("{} ${:02X} = {:02X}", name, addr, cpu.read(addr))
            }
            AddressMode::Zpx => {
                let addr = cpu.zpx(arg_addr).addr;
                format!(
                    "{} ${:02X},X @ {:02X} = {:02X}",
                    name,
                    cpu.read(arg_addr),
                    addr,
                    cpu.read(addr)
                )
            }
            AddressMode::Zpy => {
                let addr = cpu.zpy(arg_addr).addr;
                format!(
                    "{} ${:02X},Y @ {:02X} = {:02X}",
                    name,
                    cpu.read(arg_addr),
                    addr,
                    cpu.read(addr)
                )
            }
            AddressMode::Rel => format!("{} ${:04X}", name, cpu.rel(arg_addr).addr),
            AddressMode::Abs => {
                let addr = cpu.abs(arg_addr).addr;
                match instruction.instruction_type {
                    InstructionType::Jmp | InstructionType::Jsr => {
                        format!("{} ${:04X}", name, addr)
                    }
                    _ => format!("{} ${:04X} = {:02X}", name, addr, cpu.read(addr)),
                }
            }
            AddressMode::Abx => {
                let addr = cpu.abx(arg_addr).addr;
                format!(
                    "{} ${:04X},X @ {:04X} = {:02X}",
                    name,
                    cpu.read_u16(arg_addr),
                    addr,
                    cpu.read(addr)
                )
            }
            AddressMode::Aby => {
                let addr = cpu.aby(arg_addr).addr;
                format!(
                    "{} ${:04X},Y @ {:04X} = {:02X}",
                    name,
                    cpu.read_u16(arg_addr),
                    addr,
                    cpu.read(addr)
                )
            }
            AddressMode::Ind => {
                let res = cpu.ind(arg_addr);
                format!("{} (${:04X}) = {:04X}", name, res.ptr.unwrap(), res.addr)
            }
            AddressMode::Izx => {
                let res = cpu.izx(arg_addr);
                let ptr = res.ptr.unwrap();
                let addr = res.addr;
                format!(
                    "{} (${:02X},X) @ {:02X} = {:04X} = {:02X}",
                    name,
                    cpu.read(arg_addr),
                    ptr,
                    addr,
                    cpu.read(addr)
                )
            }
            AddressMode::Izy => {
                let res = cpu.izy(arg_addr);

                let ptr = cpu.read(arg_addr);
                let lo = cpu.read(ptr as u16) as u16;
                let hi = cpu.read(ptr.wrapping_add(1) as u16) as u16;
                let base_addr = (hi << 8) | lo;

                let addr = res.addr;
                format!(
                    "{} (${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                    name,
                    cpu.read(arg_addr),
                    base_addr,
                    addr,
                    cpu.read(addr)
                )
            }
        }
    }

    fn cpu_log_line(cpu: &Cpu6502) -> String {
        format!(
            "{:04X} {:02X} {:31} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} CYC:{}",
            cpu.pc(),
            cpu.read(cpu.pc()),
            get_instruction_repr(cpu, cpu.pc()),
            cpu.a(),
            cpu.x(),
            cpu.y(),
            cpu.status().bits(),
            cpu.stkp(),
            cpu.total_cycles() + cpu.cycles() as u64
        )
    }

    fn setup() -> (Rc<RefCell<Bus>>, Rc<RefCell<Cpu6502>>) {
        let bus = Rc::new(RefCell::new(Bus::new()));
        let cpu = Rc::new(RefCell::new(Cpu6502::new(Rc::downgrade(&bus))));
        bus.borrow_mut().attach_cpu(cpu.clone());
        (bus, cpu)
    }

    #[test]
    fn test_adc() {
        let (_bus, cpu) = setup();
        let mut cpu = cpu.borrow_mut();

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
        let (_bus, cpu) = setup();
        let mut cpu = cpu.borrow_mut();

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
        let (nes, cpu) = setup();

        // This rom tests everything

        // The address of the last test before the one that
        // tests the illegal opcodes, which I don't care about
        const LAST_TEST_ADDR: u16 = 0xC6A3;

        let cartridge = Rc::new(RefCell::new(
            Cartridge::new("assets/roms/nestest.nes").unwrap(),
        ));
        nes.borrow_mut().attach_cartridge(cartridge);
        let correct_log_file = File::open("assets/roms/nestest.log").unwrap();
        let mut log_reader = BufReader::new(correct_log_file);

        let mut cpu = cpu.borrow_mut();
        cpu.reset_to(0xC000);

        while cpu.pc() != LAST_TEST_ADDR {
            let mut correct_log_line = String::new();
            log_reader.read_line(&mut correct_log_line).unwrap();
            let log_line = cpu_log_line(&cpu);
            assert_eq!(correct_log_line.trim_end(), log_line);

            cpu.next_instruction();
        }
    }
}
