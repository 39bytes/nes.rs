use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct ControllerButtons: u8 {
        const A = 1 << 0;
        const B = 1 << 1;
        const Select = 1 << 2;
        const Start = 1 << 3;
        const Up = 1 << 4;
        const Down = 1 << 5;
        const Left = 1 << 6;
        const Right = 1 << 7;
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ControllerInput {
    One(ControllerButtons),
    Two(ControllerButtons),
}

#[derive(Default, Copy, Clone, Debug)]
pub struct StandardController {
    buttons: ControllerButtons,
    shift_reg: u8,
    reads_since_last_load: u8,
}

impl StandardController {
    pub fn notify_input(&mut self, buttons: ControllerButtons) {
        self.buttons = buttons;
    }

    pub fn reload(&mut self) {
        self.shift_reg = self.buttons.bits();
        self.reads_since_last_load = 0;
    }

    pub fn peek_button(&self) -> u8 {
        self.shift_reg & 0x01
    }

    pub fn read_button(&mut self) -> u8 {
        // Reading more than 8 times just returns 1
        // See: https://www.nesdev.org/wiki/Standard_controller
        if self.reads_since_last_load >= 8 {
            return 0x01;
        }

        let data = self.shift_reg & 0x01;
        self.shift_reg >>= 1;
        self.reads_since_last_load += 1;
        data
    }
}
