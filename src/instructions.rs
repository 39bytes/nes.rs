use std::fmt::Debug;

#[rustfmt::skip]
pub mod lookup {
    use super::{AddressMode, Instruction, InstructionType};
    use lazy_static::lazy_static;

    use Instruction as I;
    type A = AddressMode;
    type IT = InstructionType;

    lazy_static! {
        pub static ref INSTRUCTIONS: [Instruction; 256] = [
            I::new(IT::Brk, A::Imm, 7),I::new(IT::Ora, A::Izx, 6),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 3),I::new(IT::Ora, A::Zp0, 3),I::new(IT::Asl, A::Zp0, 5),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Php, A::Imp, 3),I::new(IT::Ora, A::Imm, 2),I::new(IT::Asl, A::Imp, 2),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Nop, A::Imp, 4),I::new(IT::Ora, A::Abs, 4),I::new(IT::Asl, A::Abs, 6),I::new(IT::Xxx, A::Imp, 6),
            I::new(IT::Bpl, A::Rel, 2),I::new(IT::Ora, A::Izy, 5),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 4),I::new(IT::Ora, A::Zpx, 4),I::new(IT::Asl, A::Zpx, 6),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Clc, A::Imp, 2),I::new(IT::Ora, A::Aby, 4),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 7),I::new(IT::Nop, A::Imp, 4),I::new(IT::Ora, A::Abx, 4),I::new(IT::Asl, A::Abx, 7),I::new(IT::Xxx, A::Imp, 7),
            I::new(IT::Jsr, A::Abs, 6),I::new(IT::And, A::Izx, 6),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Bit, A::Zp0, 3),I::new(IT::And, A::Zp0, 3),I::new(IT::Rol, A::Zp0, 5),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Plp, A::Imp, 4),I::new(IT::And, A::Imm, 2),I::new(IT::Rol, A::Imp, 2),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Bit, A::Abs, 4),I::new(IT::And, A::Abs, 4),I::new(IT::Rol, A::Abs, 6),I::new(IT::Xxx, A::Imp, 6),
            I::new(IT::Bmi, A::Rel, 2),I::new(IT::And, A::Izy, 5),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 4),I::new(IT::And, A::Zpx, 4),I::new(IT::Rol, A::Zpx, 6),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Sec, A::Imp, 2),I::new(IT::And, A::Aby, 4),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 7),I::new(IT::Nop, A::Imp, 4),I::new(IT::And, A::Abx, 4),I::new(IT::Rol, A::Abx, 7),I::new(IT::Xxx, A::Imp, 7),
            I::new(IT::Rti, A::Imp, 6),I::new(IT::Eor, A::Izx, 6),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 3),I::new(IT::Eor, A::Zp0, 3),I::new(IT::Lsr, A::Zp0, 5),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Pha, A::Imp, 3),I::new(IT::Eor, A::Imm, 2),I::new(IT::Lsr, A::Imp, 2),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Jmp, A::Abs, 3),I::new(IT::Eor, A::Abs, 4),I::new(IT::Lsr, A::Abs, 6),I::new(IT::Xxx, A::Imp, 6),
            I::new(IT::Bvc, A::Rel, 2),I::new(IT::Eor, A::Izy, 5),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 4),I::new(IT::Eor, A::Zpx, 4),I::new(IT::Lsr, A::Zpx, 6),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Cli, A::Imp, 2),I::new(IT::Eor, A::Aby, 4),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 7),I::new(IT::Nop, A::Imp, 4),I::new(IT::Eor, A::Abx, 4),I::new(IT::Lsr, A::Abx, 7),I::new(IT::Xxx, A::Imp, 7),
            I::new(IT::Rts, A::Imp, 6),I::new(IT::Adc, A::Izx, 6),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 3),I::new(IT::Adc, A::Zp0, 3),I::new(IT::Ror, A::Zp0, 5),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Pla, A::Imp, 4),I::new(IT::Adc, A::Imm, 2),I::new(IT::Ror, A::Imp, 2),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Jmp, A::Ind, 5),I::new(IT::Adc, A::Abs, 4),I::new(IT::Ror, A::Abs, 6),I::new(IT::Xxx, A::Imp, 6),
            I::new(IT::Bvs, A::Rel, 2),I::new(IT::Adc, A::Izy, 5),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 4),I::new(IT::Adc, A::Zpx, 4),I::new(IT::Ror, A::Zpx, 6),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Sei, A::Imp, 2),I::new(IT::Adc, A::Aby, 4),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 7),I::new(IT::Nop, A::Imp, 4),I::new(IT::Adc, A::Abx, 4),I::new(IT::Ror, A::Abx, 7),I::new(IT::Xxx, A::Imp, 7),
            I::new(IT::Nop, A::Imp, 2),I::new(IT::Sta, A::Izx, 6),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Sty, A::Zp0, 3),I::new(IT::Sta, A::Zp0, 3),I::new(IT::Stx, A::Zp0, 3),I::new(IT::Xxx, A::Imp, 3),I::new(IT::Dey, A::Imp, 2),I::new(IT::Nop, A::Imp, 2),I::new(IT::Txa, A::Imp, 2),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Sty, A::Abs, 4),I::new(IT::Sta, A::Abs, 4),I::new(IT::Stx, A::Abs, 4),I::new(IT::Xxx, A::Imp, 4),
            I::new(IT::Bcc, A::Rel, 2),I::new(IT::Sta, A::Izy, 6),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Sty, A::Zpx, 4),I::new(IT::Sta, A::Zpx, 4),I::new(IT::Stx, A::Zpy, 4),I::new(IT::Xxx, A::Imp, 4),I::new(IT::Tya, A::Imp, 2),I::new(IT::Sta, A::Aby, 5),I::new(IT::Txs, A::Imp, 2),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Nop, A::Imp, 5),I::new(IT::Sta, A::Abx, 5),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Xxx, A::Imp, 5),
            I::new(IT::Ldy, A::Imm, 2),I::new(IT::Lda, A::Izx, 6),I::new(IT::Ldx, A::Imm, 2),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Ldy, A::Zp0, 3),I::new(IT::Lda, A::Zp0, 3),I::new(IT::Ldx, A::Zp0, 3),I::new(IT::Xxx, A::Imp, 3),I::new(IT::Tay, A::Imp, 2),I::new(IT::Lda, A::Imm, 2),I::new(IT::Tax, A::Imp, 2),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Ldy, A::Abs, 4),I::new(IT::Lda, A::Abs, 4),I::new(IT::Ldx, A::Abs, 4),I::new(IT::Xxx, A::Imp, 4),
            I::new(IT::Bcs, A::Rel, 2),I::new(IT::Lda, A::Izy, 5),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Ldy, A::Zpx, 4),I::new(IT::Lda, A::Zpx, 4),I::new(IT::Ldx, A::Zpy, 4),I::new(IT::Xxx, A::Imp, 4),I::new(IT::Clv, A::Imp, 2),I::new(IT::Lda, A::Aby, 4),I::new(IT::Tsx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 4),I::new(IT::Ldy, A::Abx, 4),I::new(IT::Lda, A::Abx, 4),I::new(IT::Ldx, A::Aby, 4),I::new(IT::Xxx, A::Imp, 4),
            I::new(IT::Cpy, A::Imm, 2),I::new(IT::Cmp, A::Izx, 6),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Cpy, A::Zp0, 3),I::new(IT::Cmp, A::Zp0, 3),I::new(IT::Dec, A::Zp0, 5),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Iny, A::Imp, 2),I::new(IT::Cmp, A::Imm, 2),I::new(IT::Dex, A::Imp, 2),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Cpy, A::Abs, 4),I::new(IT::Cmp, A::Abs, 4),I::new(IT::Dec, A::Abs, 6),I::new(IT::Xxx, A::Imp, 6),
            I::new(IT::Bne, A::Rel, 2),I::new(IT::Cmp, A::Izy, 5),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 4),I::new(IT::Cmp, A::Zpx, 4),I::new(IT::Dec, A::Zpx, 6),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Cld, A::Imp, 2),I::new(IT::Cmp, A::Aby, 4),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 7),I::new(IT::Nop, A::Imp, 4),I::new(IT::Cmp, A::Abx, 4),I::new(IT::Dec, A::Abx, 7),I::new(IT::Xxx, A::Imp, 7),
            I::new(IT::Cpx, A::Imm, 2),I::new(IT::Sbc, A::Izx, 6),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Cpx, A::Zp0, 3),I::new(IT::Sbc, A::Zp0, 3),I::new(IT::Inc, A::Zp0, 5),I::new(IT::Xxx, A::Imp, 5),I::new(IT::Inx, A::Imp, 2),I::new(IT::Sbc, A::Imm, 2),I::new(IT::Nop, A::Imp, 2),I::new(IT::Sbc, A::Imp, 2),I::new(IT::Cpx, A::Abs, 4),I::new(IT::Sbc, A::Abs, 4),I::new(IT::Inc, A::Abs, 6),I::new(IT::Xxx, A::Imp, 6),
            I::new(IT::Beq, A::Rel, 2),I::new(IT::Sbc, A::Izy, 5),I::new(IT::Xxx, A::Imp, 2),I::new(IT::Xxx, A::Imp, 8),I::new(IT::Nop, A::Imp, 4),I::new(IT::Sbc, A::Zpx, 4),I::new(IT::Inc, A::Zpx, 6),I::new(IT::Xxx, A::Imp, 6),I::new(IT::Sed, A::Imp, 2),I::new(IT::Sbc, A::Aby, 4),I::new(IT::Nop, A::Imp, 2),I::new(IT::Xxx, A::Imp, 7),I::new(IT::Nop, A::Imp, 4),I::new(IT::Sbc, A::Abx, 4),I::new(IT::Inc, A::Abx, 7),I::new(IT::Xxx, A::Imp, 7),
        ];
    }
}

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
}

#[derive(Debug)]
pub enum AddressMode {
    Imp,
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

#[derive(Debug)]
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
    Xxx,
}
