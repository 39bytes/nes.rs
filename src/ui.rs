use crate::renderer::{Layer, Renderer, Sprite};
use emu::{
    cpu::{
        instructions::{AddressMode, Instruction},
        Cpu,
    },
    nes::Nes,
    palette::Color,
    ppu::{
        sprite::{Sprite as PpuSprite, SpriteAttribute},
        PatternTable, Ppu,
    },
};

pub fn draw_oam(renderer: &mut Renderer, oam: &[u8], x: usize, y: usize) {
    for (i, sprite) in oam.chunks_exact(4).enumerate().take(8) {
        let sp_y = sprite[0];
        let tile_id = sprite[1];
        let attr = sprite[2];
        let sp_x = sprite[3];

        renderer.draw_text(
            &format!(
                "X: {:02X}, Y: {:02X}, Tile: {:02X}, Attr: {:02X}",
                sp_x, sp_y, tile_id, attr
            ),
            x,
            y + (20 + i * 20),
        );
    }
}

pub fn draw_mem_page(renderer: &mut Renderer, nes: &Nes, page: u8, x: usize, y: usize) {
    let page_start = (page as u16) * 0x100;
    let page_end = page_start + 0xFF;

    renderer.draw_text(&format!("{:#06X}-{:#06X}", page_start, page_end), x, y);
    for (i, line) in nes.cpu_mem_page_str(page).split('\n').enumerate() {
        renderer.draw_text(line, x, y + 40 + i * 20);
    }
}

pub fn draw_flags(renderer: &mut Renderer, flags: u8, text: &str, x: usize, y: usize) {
    renderer.draw_text_with_computed_color(text, x, y, |i| {
        if flags & (1 << (7 - i)) != 0 {
            Color::WHITE
        } else {
            Color::GRAY
        }
    });
}

pub fn draw_cpu_info(renderer: &mut Renderer, nes: &Nes, x: usize, y: usize) {
    let cpu = nes.cpu();

    renderer.draw_text("CPU Registers:", x, y);
    renderer.draw_text(&format!("A: {:#04X}", cpu.a()), x, y + 20);
    renderer.draw_text(&format!("X: {:#04X}", cpu.x()), x, y + 40);
    renderer.draw_text(&format!("Y: {:#04X}", cpu.y()), x, y + 60);
    renderer.draw_text(&format!("Stack: {:#04X}", cpu.stkp()), x, y + 80);
    renderer.draw_text("Status:", x, y + 100);

    draw_flags(renderer, cpu.status().bits(), "NVUBDIZC", x + 96, y + 100);
    renderer.draw_text(&format!("PC: {:#06X}", cpu.pc()), x, y + 140);

    renderer.draw_text(&format!("Cycles: {}", cpu.cycles()), x, y + 160);

    let mut addr = cpu.pc();

    for i in 0..10 {
        let instruction = Instruction::lookup(cpu.read_debug(addr));
        let instruction_repr = get_instruction_repr(&cpu, addr);

        renderer.draw_text(
            &format!("${:4X}: {}", addr, instruction_repr),
            x,
            y + 200 + i * 20,
        );

        let stride = instruction.address_mode.arg_size() + 1;
        // Prevent overflow when near end of address space
        if 0xFFFF - addr < stride {
            break;
        }
        addr += stride;
    }
}

pub fn get_instruction_repr(cpu: &Cpu, addr: u16) -> String {
    let instruction = Instruction::lookup(cpu.read_debug(addr));
    let arg_addr = addr + 1;

    let name = instruction.instruction_type.as_ref().to_uppercase();

    match instruction.address_mode {
        AddressMode::Imp => name,
        AddressMode::Acc => format!("{} A", name),
        AddressMode::Imm => format!("{} #${:02X}", name, cpu.read_debug(arg_addr)),
        AddressMode::Zp0 => format!("{} ${:02X}", name, cpu.read_debug(arg_addr),),
        AddressMode::Zpx => format!("{} ${:02X},X", name, cpu.read_debug(arg_addr)),
        AddressMode::Zpy => format!("{} ${:02X},Y", name, cpu.read_debug(arg_addr)),
        AddressMode::Rel => {
            let offset = cpu.read_debug(arg_addr) as i8;
            let computed_addr = if offset < 0 {
                addr.wrapping_sub(offset.unsigned_abs() as u16)
            } else {
                addr.wrapping_add(offset as u16)
            };
            format!("{} ${:02X}", name, computed_addr + 2)
        }
        AddressMode::Abs => format!("{} ${:04X}", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Abx => format!("{} ${:04X},X", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Aby => format!("{} ${:04X},Y", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Ind => format!("{} (${:04X})", name, cpu.read_debug_u16(arg_addr)),
        AddressMode::Izx => format!("{} (${:02X},X)", name, cpu.read_debug(arg_addr)),
        AddressMode::Izy => format!("{} (${:02X}),Y", name, cpu.read_debug(arg_addr)),
    }
}

pub fn draw_ppu_info(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    renderer.draw_text("PPU Registers:", x, y);
    renderer.draw_text("CTRL: ", x, y + 20);
    draw_flags(renderer, ppu.ctrl().bits(), "VPHBSINN", x + 80, y + 20);

    renderer.draw_text("MASK: ", x, y + 40);
    draw_flags(renderer, ppu.mask().bits(), "BGRsbMmG", x + 80, y + 40);

    renderer.draw_text("STATUS: ", x, y + 60);
    draw_flags(renderer, ppu.status().bits(), "VSO-----", x + 96, y + 60);

    renderer.draw_text(&format!("OAMADDR: {:#06X}", ppu.oam_addr()), x, y + 80);
    renderer.draw_text(&format!("ADDR: {:#06X}", ppu.addr()), x, y + 100);
    renderer.draw_text(&format!("DATA: {:#06X}", ppu.data()), x, y + 120);
    renderer.draw_text(&format!("S: {}", ppu.scanline()), x, y + 140);
    renderer.draw_text(&format!("C: {}", ppu.cycle()), x + 80, y + 140);
}

pub fn draw_pattern_tables(renderer: &mut Renderer, ppu: &Ppu, palette: u8, x: usize, y: usize) {
    let left_pattern_table = ppu.get_pattern_table(PatternTable::Left, palette, false);
    let right_pattern_table = ppu.get_pattern_table(PatternTable::Right, palette, true);

    renderer.draw_text("Pattern Tables", x, y);
    renderer.draw(
        Layer::UI,
        &Sprite::from_slice(&left_pattern_table, 128, 128).unwrap(),
        x,
        y + 24,
    );
    renderer.draw(
        Layer::UI,
        &Sprite::from_slice(&right_pattern_table, 128, 128).unwrap(),
        x + 144,
        y + 24,
    );
}

fn palette_sprite(ppu: &Ppu, palette_index: u8) -> Sprite {
    let bg_color = ppu.get_palette_color(palette_index, 0);
    let color1 = ppu.get_palette_color(palette_index, 1);
    let color2 = ppu.get_palette_color(palette_index, 2);
    let color3 = ppu.get_palette_color(palette_index, 3);

    Sprite::new(
        [
            bg_color.as_slice(),
            color1.as_slice(),
            color2.as_slice(),
            color3.as_slice(),
        ]
        .concat(),
        4,
        1,
    )
    .unwrap()
}

pub fn draw_palettes(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    renderer.draw_text("Background", x, y);
    for i in 0..4 {
        renderer.draw(
            Layer::UI,
            &palette_sprite(ppu, i).scale(16),
            x + 80 * i as usize,
            y + 24,
        );
    }

    renderer.draw_text("Sprites", x, y + 48);
    for i in 4..8 {
        renderer.draw(
            Layer::UI,
            &palette_sprite(ppu, i).scale(16),
            x + 80 * (i - 4) as usize,
            y + 72,
        );
    }
}

pub fn draw_nametable(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    const ATTRIBUTE_MEMORY_OFFSET: u16 = 0x2000 + 0x03C0;

    for i in 0..30 {
        for j in 0..32 {
            let group_offset = (i / 4) * 8 + j / 4;
            let mut palette_byte = ppu.read_debug(ATTRIBUTE_MEMORY_OFFSET + group_offset);
            if i % 4 >= 2 {
                palette_byte >>= 4;
            }
            if j % 4 >= 2 {
                palette_byte >>= 2;
            }
            let palette = palette_byte & 0x03;

            renderer.draw_text(
                &format!("{:02X}", palette),
                (j as usize) * 24 + x,
                (i as usize) * 16 + y,
            );
        }
    }
}

pub fn draw_oam_sprites(renderer: &mut Renderer, ppu: &Ppu, x: usize, y: usize) {
    let oam = ppu.oam();

    for (i, sprite) in oam.chunks_exact(4).enumerate() {
        let sprite = PpuSprite::from_bytes(sprite, i);

        let tx = i % 4;
        let ty = i / 4;

        let palette = (sprite.attribute.contains(SpriteAttribute::PaletteMSB) as u8) << 1
            | (sprite.attribute.contains(SpriteAttribute::PaletteLSB) as u8);

        renderer.draw_text(
            &format!("{:02X}-{}", sprite.tile_id, palette),
            x + tx * 56,
            y + ty * 20,
        );
    }
}
