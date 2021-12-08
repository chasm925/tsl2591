use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

const DEVICE_ADDR: u16 = 0x29;
const COMMAND_BIT: u8 = 0xA0;
const REGISTER_DEVICE_ID: u8 = 0x12;
const REGISTER_ENABLE: u8 = 0x00;
const REGISTER_CONTROL: u8 = 0x01;
const REGISTER_CHAN0_LOW: u8 = 0x14;
const REGISTER_CHAN1_LOW: u8 = 0x16;

const POWER_OFF: u8 = 0x00;
const POWER_ON: u8 = 0x01;
const ENABLE_AEN: u8 = 0x02;
const ENABLE_AIEN: u8 = 0x10;
const ENABLE_NPIEN: u8 = 0x80;

const LUX_DF: f32 = 408.0;

pub struct Tsl2591 {
    enabled: bool,
    integration: IntegrationTime,
    gain: Gain,
    i2cdev: LinuxI2CDevice
}

impl Tsl2591 {
    pub fn new() -> Self {
        let mut dev = LinuxI2CDevice::new("/dev/i2c-1", DEVICE_ADDR).unwrap();

        let device_id = dev.smbus_read_byte_data(COMMAND_BIT | REGISTER_DEVICE_ID);
        
        match device_id {
            Ok(id) => println!("Found tsl2591: {}", id),
            Err(error) => panic!("Failed to find tsl2591: {:?}", error)
        };

        let mut tsl_2591 = Tsl2591 {
            enabled: false,
            integration: IntegrationTime::IT100MS,
            gain: Gain::MEDIUM,
            i2cdev: dev
        };

        // low power mode by default
        tsl_2591.disable();

        tsl_2591
    }

    pub fn read(&mut self) -> f32 {
        let (ch0, ch1) = self.get_full_luminosity();
        self.calculate_lux(ch0, ch1)
    }

    pub fn enable(&mut self) {
        self.i2cdev
            .smbus_write_byte_data(COMMAND_BIT | REGISTER_ENABLE, POWER_ON | ENABLE_AEN | ENABLE_AIEN | ENABLE_NPIEN)
            .expect("Failed to enable device");
            
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.i2cdev
            .smbus_write_byte_data(COMMAND_BIT | REGISTER_ENABLE, POWER_OFF)
            .expect("Failed to disable device");

        self.enabled = false;
    }

    pub fn set_timing_gain(&mut self, timing: IntegrationTime, gain: Gain) {
        self.i2cdev
            .smbus_write_byte_data(COMMAND_BIT | REGISTER_CONTROL, timing as u8 | gain as u8)
            .expect("Failed to set device timing");

        self.integration = timing;
        self.gain = gain;
    }

    fn get_full_luminosity(&mut self) -> (u16 ,u16) {
        let c0 = self.i2cdev
            .smbus_read_i2c_block_data(COMMAND_BIT | REGISTER_CHAN0_LOW, 2)
            .expect("Error reading chan 0");

        // println!("c0: {}, {}", c0[0], c0[1]);

        let c1 = self.i2cdev
            .smbus_read_i2c_block_data(COMMAND_BIT | REGISTER_CHAN1_LOW, 2)
            .expect("Error reading chan 1");

        // println!("c1: {}, {}", c1[0], c1[1]);

        // https://stackoverflow.com/questions/50243866/how-do-i-convert-two-u8-primitives-into-a-u16-primitive
        let c0:u16 = ((c0[1] as u16) << 8) | c0[0] as u16;
        let c1:u16 = ((c1[1] as u16) << 8) | c1[0] as u16;

        (c0, c1)
    }

    fn calculate_lux(&mut self, ch0: u16, ch1: u16) -> f32 {
        let atime: f32;
        let again: f32;
        
        if ch0 == 0xFFFF || ch1 == 0xFFFF {
            panic!("overflow encountered");
        }

        match self.integration {
            IntegrationTime::IT100MS => atime = 100.0,
            IntegrationTime::IT200MS => atime = 200.0,
            IntegrationTime::IT300MS => atime = 300.0,
            IntegrationTime::IT400MS => atime = 400.0,
            IntegrationTime::IT500MS => atime = 500.0,
            IntegrationTime::IT600MS => atime = 600.0,
        }

        match self.gain {
            Gain::LOW => again = 1.0,
            Gain::MEDIUM => again = 25.0,
            Gain::GHIGH => again = 428.0,
            Gain::MAX => again = 9876.0
        }

        let ch0 = ch0 as f32;
        let ch1 = ch1 as f32;

        let cpl = (atime * again) / LUX_DF;
        let lux = (ch0 - ch1) * (1.0 - (ch1 / ch0)) / cpl;

        lux
    }
}

#[derive(Copy, Clone)]
pub enum IntegrationTime {
    IT100MS = 0x00,
    IT200MS = 0x01,
    IT300MS = 0x02,
    IT400MS = 0x03,
    IT500MS = 0x04,
    IT600MS = 0x05
}

#[derive(Copy, Clone)]
pub enum Gain {
    LOW = 0x00,
    MEDIUM = 0x10,
    GHIGH = 0x20,
    MAX = 0x30,
}

#[cfg(test)]
mod tests {
    #[test]
    fn calc_lux() {
        let lux = super::calculate_lux2(430, 151);
        println!("{}", lux);
        assert_eq!(lux, 29.33);
    }
}
