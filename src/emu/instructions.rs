use std::fmt::Debug;
use strum_macros::AsRefStr;

#[derive(Clone)]
pub struct Instruction {
    pub instruction_type: InstructionType,
    pub address_mode: AddressMode,
    pub cycles: u8,
}

impl Instruction {
    pub fn new(instruction_type: InstructionType, address_mode: AddressMode, cycles: u8) -> Self {
        Instruction {
            instruction_type,
            address_mode,
            cycles,
        }
    }

    #[inline(always)]
    pub fn lookup(opcode: u8) -> Self {
        use AddressMode::*;
        use Instruction as I;
        use InstructionType::*;

        // References:
        // https://www.oxyron.de/html/opcodes02.html
        // https://www.nesdev.org/wiki/CPU_unofficial_opcodes
        match opcode {
            0x00 => I::new(Brk, Imm, 7),
            0x01 => I::new(Ora, Izx, 6),
            0x02 => I::new(Stp, Imp, 3),
            0x03 => I::new(Slo, Izx, 8),
            0x04 => I::new(Nop, Zp0, 3),
            0x05 => I::new(Ora, Zp0, 3),
            0x06 => I::new(Asl, Zp0, 5),
            0x07 => I::new(Slo, Zp0, 5),
            0x08 => I::new(Php, Imp, 3),
            0x09 => I::new(Ora, Imm, 2),
            0x0A => I::new(Asl, Acc, 2),
            0x0B => I::new(Anc, Imm, 2),
            0x0C => I::new(Nop, Abs, 4),
            0x0D => I::new(Ora, Abs, 4),
            0x0E => I::new(Asl, Abs, 6),
            0x0F => I::new(Slo, Abs, 6),
            0x10 => I::new(Bpl, Rel, 2),
            0x11 => I::new(Ora, Izy, 5),
            0x12 => I::new(Stp, Imp, 3),
            0x13 => I::new(Slo, Izy, 8),
            0x14 => I::new(Nop, Zpx, 4),
            0x15 => I::new(Ora, Zpx, 4),
            0x16 => I::new(Asl, Zpx, 6),
            0x17 => I::new(Slo, Zpx, 6),
            0x18 => I::new(Clc, Imp, 2),
            0x19 => I::new(Ora, Aby, 4),
            0x1A => I::new(Nop, Imp, 2),
            0x1B => I::new(Slo, Aby, 7),
            0x1C => I::new(Nop, Abx, 4),
            0x1D => I::new(Ora, Abx, 4),
            0x1E => I::new(Asl, Abx, 7),
            0x1F => I::new(Slo, Abx, 7),
            0x20 => I::new(Jsr, Abs, 6),
            0x21 => I::new(And, Izx, 6),
            0x22 => I::new(Stp, Imp, 3),
            0x23 => I::new(Rla, Izx, 8),
            0x24 => I::new(Bit, Zp0, 3),
            0x25 => I::new(And, Zp0, 3),
            0x26 => I::new(Rol, Zp0, 5),
            0x27 => I::new(Rla, Zp0, 5),
            0x28 => I::new(Plp, Imp, 4),
            0x29 => I::new(And, Imm, 2),
            0x2A => I::new(Rol, Acc, 2),
            0x2B => I::new(Anc, Imm, 2),
            0x2C => I::new(Bit, Abs, 4),
            0x2D => I::new(And, Abs, 4),
            0x2E => I::new(Rol, Abs, 6),
            0x2F => I::new(Rla, Abs, 6),
            0x30 => I::new(Bmi, Rel, 2),
            0x31 => I::new(And, Izy, 5),
            0x32 => I::new(Stp, Imp, 3),
            0x33 => I::new(Rla, Izy, 8),
            0x34 => I::new(Nop, Zpx, 4),
            0x35 => I::new(And, Zpx, 4),
            0x36 => I::new(Rol, Zpx, 6),
            0x37 => I::new(Rla, Zpx, 6),
            0x38 => I::new(Sec, Imp, 2),
            0x39 => I::new(And, Aby, 4),
            0x3A => I::new(Nop, Imp, 2),
            0x3B => I::new(Rla, Aby, 7),
            0x3C => I::new(Nop, Abx, 4),
            0x3D => I::new(And, Abx, 4),
            0x3E => I::new(Rol, Abx, 7),
            0x3F => I::new(Rla, Abx, 7),
            0x40 => I::new(Rti, Imp, 6),
            0x41 => I::new(Eor, Izx, 6),
            0x42 => I::new(Stp, Imp, 3),
            0x43 => I::new(Sre, Izx, 8),
            0x44 => I::new(Nop, Zp0, 3),
            0x45 => I::new(Eor, Zp0, 3),
            0x46 => I::new(Lsr, Zp0, 5),
            0x47 => I::new(Sre, Zp0, 5),
            0x48 => I::new(Pha, Imp, 3),
            0x49 => I::new(Eor, Imm, 2),
            0x4A => I::new(Lsr, Acc, 2),
            0x4B => I::new(Alr, Imm, 2),
            0x4C => I::new(Jmp, Abs, 3),
            0x4D => I::new(Eor, Abs, 4),
            0x4E => I::new(Lsr, Abs, 6),
            0x4F => I::new(Sre, Abs, 6),
            0x50 => I::new(Bvc, Rel, 2),
            0x51 => I::new(Eor, Izy, 5),
            0x52 => I::new(Stp, Imp, 2),
            0x53 => I::new(Sre, Izy, 8),
            0x54 => I::new(Nop, Zpx, 4),
            0x55 => I::new(Eor, Zpx, 4),
            0x56 => I::new(Lsr, Zpx, 6),
            0x57 => I::new(Sre, Zpx, 6),
            0x58 => I::new(Cli, Imp, 2),
            0x59 => I::new(Eor, Aby, 4),
            0x5A => I::new(Nop, Imp, 2),
            0x5B => I::new(Sre, Aby, 7),
            0x5C => I::new(Nop, Abx, 4),
            0x5D => I::new(Eor, Abx, 4),
            0x5E => I::new(Lsr, Abx, 7),
            0x5F => I::new(Sre, Abx, 7),
            0x60 => I::new(Rts, Imp, 6),
            0x61 => I::new(Adc, Izx, 6),
            0x62 => I::new(Stp, Imp, 3),
            0x63 => I::new(Rra, Izx, 8),
            0x64 => I::new(Nop, Zp0, 3),
            0x65 => I::new(Adc, Zp0, 3),
            0x66 => I::new(Ror, Zp0, 5),
            0x67 => I::new(Rra, Zp0, 5),
            0x68 => I::new(Pla, Imp, 4),
            0x69 => I::new(Adc, Imm, 2),
            0x6A => I::new(Ror, Acc, 2),
            0x6B => I::new(Arr, Imm, 2),
            0x6C => I::new(Jmp, Ind, 5),
            0x6D => I::new(Adc, Abs, 4),
            0x6E => I::new(Ror, Abs, 6),
            0x6F => I::new(Rra, Abs, 6),
            0x70 => I::new(Bvs, Rel, 2),
            0x71 => I::new(Adc, Izy, 5),
            0x72 => I::new(Stp, Imp, 3),
            0x73 => I::new(Rra, Izy, 8),
            0x74 => I::new(Nop, Zpx, 4),
            0x75 => I::new(Adc, Zpx, 4),
            0x76 => I::new(Ror, Zpx, 6),
            0x77 => I::new(Rra, Zpx, 6),
            0x78 => I::new(Sei, Imp, 2),
            0x79 => I::new(Adc, Aby, 4),
            0x7A => I::new(Nop, Imp, 2),
            0x7B => I::new(Rra, Aby, 7),
            0x7C => I::new(Nop, Abx, 4),
            0x7D => I::new(Adc, Abx, 4),
            0x7E => I::new(Ror, Abx, 7),
            0x7F => I::new(Rra, Abx, 7),
            0x80 => I::new(Nop, Imm, 2),
            0x81 => I::new(Sta, Izx, 6),
            0x82 => I::new(Nop, Imm, 2),
            0x83 => I::new(Sax, Izx, 6),
            0x84 => I::new(Sty, Zp0, 3),
            0x85 => I::new(Sta, Zp0, 3),
            0x86 => I::new(Stx, Zp0, 3),
            0x87 => I::new(Sax, Zp0, 3),
            0x88 => I::new(Dey, Imp, 2),
            0x89 => I::new(Nop, Imm, 2),
            0x8A => I::new(Txa, Imp, 2),
            0x8B => I::new(Xaa, Imm, 2),
            0x8C => I::new(Sty, Abs, 4),
            0x8D => I::new(Sta, Abs, 4),
            0x8E => I::new(Stx, Abs, 4),
            0x8F => I::new(Sax, Abs, 4),
            0x90 => I::new(Bcc, Rel, 2),
            0x91 => I::new(Sta, Izy, 6),
            0x92 => I::new(Stp, Imp, 3),
            0x93 => I::new(Ahx, Izy, 6),
            0x94 => I::new(Sty, Zpx, 4),
            0x95 => I::new(Sta, Zpx, 4),
            0x96 => I::new(Stx, Zpy, 4),
            0x97 => I::new(Sax, Zpy, 4),
            0x98 => I::new(Tya, Imp, 2),
            0x99 => I::new(Sta, Aby, 5),
            0x9A => I::new(Txs, Imp, 2),
            0x9B => I::new(Tas, Aby, 5),
            0x9C => I::new(Shy, Abx, 5),
            0x9D => I::new(Sta, Abx, 5),
            0x9E => I::new(Shx, Aby, 5),
            0x9F => I::new(Ahx, Aby, 5),
            0xA0 => I::new(Ldy, Imm, 2),
            0xA1 => I::new(Lda, Izx, 6),
            0xA2 => I::new(Ldx, Imm, 2),
            0xA3 => I::new(Lax, Izx, 6),
            0xA4 => I::new(Ldy, Zp0, 3),
            0xA5 => I::new(Lda, Zp0, 3),
            0xA6 => I::new(Ldx, Zp0, 3),
            0xA7 => I::new(Lax, Zp0, 3),
            0xA8 => I::new(Tay, Imp, 2),
            0xA9 => I::new(Lda, Imm, 2),
            0xAA => I::new(Tax, Imp, 2),
            0xAB => I::new(Lax, Imm, 4),
            0xAC => I::new(Ldy, Abs, 4),
            0xAD => I::new(Lda, Abs, 4),
            0xAE => I::new(Ldx, Abs, 4),
            0xAF => I::new(Lax, Abs, 4),
            0xB0 => I::new(Bcs, Rel, 2),
            0xB1 => I::new(Lda, Izy, 5),
            0xB2 => I::new(Stp, Imp, 3),
            0xB3 => I::new(Lax, Izy, 5),
            0xB4 => I::new(Ldy, Zpx, 4),
            0xB5 => I::new(Lda, Zpx, 4),
            0xB6 => I::new(Ldx, Zpy, 4),
            0xB7 => I::new(Lax, Zpy, 4),
            0xB8 => I::new(Clv, Imp, 2),
            0xB9 => I::new(Lda, Aby, 4),
            0xBA => I::new(Tsx, Imp, 2),
            0xBB => I::new(Las, Aby, 4),
            0xBC => I::new(Ldy, Abx, 4),
            0xBD => I::new(Lda, Abx, 4),
            0xBE => I::new(Ldx, Aby, 4),
            0xBF => I::new(Lax, Aby, 4),
            0xC0 => I::new(Cpy, Imm, 2),
            0xC1 => I::new(Cmp, Izx, 6),
            0xC2 => I::new(Nop, Imm, 2),
            0xC3 => I::new(Dcp, Izx, 8),
            0xC4 => I::new(Cpy, Zp0, 3),
            0xC5 => I::new(Cmp, Zp0, 3),
            0xC6 => I::new(Dec, Zp0, 5),
            0xC7 => I::new(Dcp, Zp0, 5),
            0xC8 => I::new(Iny, Imp, 2),
            0xC9 => I::new(Cmp, Imm, 2),
            0xCA => I::new(Dex, Imp, 2),
            0xCB => I::new(Axs, Imm, 2),
            0xCC => I::new(Cpy, Abs, 4),
            0xCD => I::new(Cmp, Abs, 4),
            0xCE => I::new(Dec, Abs, 6),
            0xCF => I::new(Dcp, Abs, 6),
            0xD0 => I::new(Bne, Rel, 2),
            0xD1 => I::new(Cmp, Izy, 5),
            0xD2 => I::new(Stp, Imp, 3),
            0xD3 => I::new(Dcp, Izy, 8),
            0xD4 => I::new(Nop, Zpx, 4),
            0xD5 => I::new(Cmp, Zpx, 4),
            0xD6 => I::new(Dec, Zpx, 6),
            0xD7 => I::new(Dcp, Zpx, 6),
            0xD8 => I::new(Cld, Imp, 2),
            0xD9 => I::new(Cmp, Aby, 4),
            0xDA => I::new(Nop, Imp, 2),
            0xDB => I::new(Dcp, Aby, 7),
            0xDC => I::new(Nop, Abx, 4),
            0xDD => I::new(Cmp, Abx, 4),
            0xDE => I::new(Dec, Abx, 7),
            0xDF => I::new(Dcp, Abx, 7),
            0xE0 => I::new(Cpx, Imm, 2),
            0xE1 => I::new(Sbc, Izx, 6),
            0xE2 => I::new(Nop, Imm, 2),
            0xE3 => I::new(Isc, Izx, 8),
            0xE4 => I::new(Cpx, Zp0, 3),
            0xE5 => I::new(Sbc, Zp0, 3),
            0xE6 => I::new(Inc, Zp0, 5),
            0xE7 => I::new(Isc, Zp0, 5),
            0xE8 => I::new(Inx, Imp, 2),
            0xE9 => I::new(Sbc, Imm, 2),
            0xEA => I::new(Nop, Imp, 2),
            0xEB => I::new(Sbc, Imm, 2),
            0xEC => I::new(Cpx, Abs, 4),
            0xED => I::new(Sbc, Abs, 4),
            0xEE => I::new(Inc, Abs, 6),
            0xEF => I::new(Isc, Abs, 6),
            0xF0 => I::new(Beq, Rel, 2),
            0xF1 => I::new(Sbc, Izy, 5),
            0xF2 => I::new(Stp, Imp, 3),
            0xF3 => I::new(Isc, Izy, 8),
            0xF4 => I::new(Nop, Zpx, 4),
            0xF5 => I::new(Sbc, Zpx, 4),
            0xF6 => I::new(Inc, Zpx, 6),
            0xF7 => I::new(Isc, Zpx, 6),
            0xF8 => I::new(Sed, Imp, 2),
            0xF9 => I::new(Sbc, Aby, 4),
            0xFA => I::new(Nop, Imp, 2),
            0xFB => I::new(Isc, Aby, 7),
            0xFC => I::new(Nop, Abx, 4),
            0xFD => I::new(Sbc, Abx, 4),
            0xFE => I::new(Inc, Abx, 7),
            0xFF => I::new(Isc, Abx, 7),
        }
    }
}

#[derive(Debug, AsRefStr, Copy, Clone)]
pub enum AddressMode {
    Imp,
    Acc,
    Imm,
    Zp0,
    Zpx,
    Zpy,
    Rel,
    Abs,
    Abx,
    Aby,
    Ind,
    Izx,
    Izy,
}

impl AddressMode {
    pub fn arg_size(&self) -> u16 {
        match self {
            AddressMode::Imp => 0,
            AddressMode::Acc => 0,
            AddressMode::Imm => 1,
            AddressMode::Zp0 => 1,
            AddressMode::Zpx => 1,
            AddressMode::Zpy => 1,
            AddressMode::Rel => 1,
            AddressMode::Abs => 2,
            AddressMode::Abx => 2,
            AddressMode::Aby => 2,
            AddressMode::Ind => 2,
            AddressMode::Izx => 1,
            AddressMode::Izy => 1,
        }
    }
}

#[derive(Debug, AsRefStr, Copy, Clone)]
pub enum InstructionType {
    Adc,
    And,
    Asl,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Lda,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sbc,
    Sec,
    Sed,
    Sei,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
    Slo,
    Rla,
    Sre,
    Rra,
    Sax,
    Lax,
    Dcp,
    Isc,
    Anc,
    Alr,
    Arr,
    Xaa,
    Axs,
    Ahx,
    Shy,
    Shx,
    Tas,
    Las,
    Stp,
}
