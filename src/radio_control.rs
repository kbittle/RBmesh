use defmt::unwrap;
use defmt_rtt as _; // global logger
use core::time::Duration;
use stm32wlxx_hal::{
    dma::{Dma1Ch1, Dma2Ch1},
    gpio::{pins, Output},
    spi::{SgMiso, SgMosi},
    subghz::{
        self, rfbusys, wakeup, AddrComp, CalibrateImage, CfgIrq, CmdStatus, CodingRate, CrcType,
        FallbackMode, 
        GenericPacketParams, HeaderType, Irq, IrqLine, LoRaBandwidth, LoRaModParams, LoRaPacketParams,
        LoRaSyncWord, Ocp, PaConfig, PaSel, PacketType, PktCtrl, PreambleDetection, RampTime, RegMode,
        RfFreq, SleepCfg, SpreadingFactor, StandbyClk, Startup, Status, StatusMode, SubGhz,
        TcxoMode, TcxoTrim, Timeout, TxParams,
    },
};
use heapless::{String}; 

const PREAMBLE_LEN: u16 = 16;
const DATA_LEN: u8 = 255;

enum RfSwitchType {
    RfSwitchOff,
    RfSwitchRx,
    RfSwitchTxLp,
    RfSwitchTxHp
}

// Structure to store all LoRa radio configurations
#[derive(Debug, Clone, PartialEq)]
struct RadioConfiguration {
    frequency: RfFreq,
    pkt_params: LoRaPacketParams,
    mod_params: LoRaModParams,
    pa_cfg: PaConfig,
    tx_params: TxParams,
    txco: TcxoMode,
    irq_cfg: CfgIrq,
}

#[derive(Debug)]
pub struct RadioControl {
    radio: SubGhz<SgMiso, SgMosi>,
    gpio_txco_pwr: Output<pins::B0>,
    gpio_rf_ctrl_1: Output<pins::A4>,
    gpio_rf_ctrl_2: Output<pins::A5>,
    config: RadioConfiguration,
}

impl RadioControl {
    // Function to initialize radio module variables
    pub fn new(
        radio: SubGhz<SgMiso, SgMosi>,
        gpio_txco_pwr: Output<pins::B0>,
        gpio_rf_ctrl_1: Output<pins::A4>,
        gpio_rf_ctrl_2: Output<pins::A5>,
    ) -> RadioControl {
        // Returns new instance of RadioControlStruct with default Lora settings below
        RadioControl {
            radio,
            gpio_txco_pwr,
            gpio_rf_ctrl_1,
            gpio_rf_ctrl_2,
            config: RadioConfiguration {
                frequency: RfFreq::F915,
                pkt_params: LoRaPacketParams::new()
                    .set_crc_en(true)
                    .set_preamble_len(PREAMBLE_LEN)
                    .set_payload_len(DATA_LEN)
                    .set_invert_iq(false)
                    .set_header_type(HeaderType::Variable),
                mod_params: LoRaModParams::new()
                    .set_bw(LoRaBandwidth::Bw125)
                    .set_cr(CodingRate::Cr45)
                    .set_ldro_en(true)
                    .set_sf(SpreadingFactor::Sf7),
                pa_cfg: PaConfig::new()
                    .set_pa(PaSel::Hp)
                    .set_hp_max(0x2)
                    .set_pa_duty_cycle(0x4),
                tx_params: TxParams::LP_10.set_ramp_time(RampTime::Micros40),
                txco: TcxoMode::new()
                    .set_txco_trim(TcxoTrim::Volts1pt7)
                    .set_timeout(Timeout::from_duration_sat(Duration::from_millis(10))),// this should be longer but panics
                irq_cfg: CfgIrq::new()
                    .irq_enable_all(Irq::TxDone)
                    .irq_enable_all(Irq::RxDone)
                    .irq_enable_all(Irq::PreambleDetected)
                    .irq_enable_all(Irq::Err)
                    .irq_enable_all(Irq::Timeout),
            },
        }
    }

    // LoRa-E5 radio clk requires power from GPIO
    pub fn power_on(&mut self) {        
        unsafe { subghz::wakeup() };

        // Turn on TCXO which is the radio HSE
        self.gpio_txco_pwr.set_level_high();

        //Turns On in Rx Mode the RF Switch
        self.configure_rf_switch(RfSwitchType::RfSwitchRx);

        // This crashes
        // unwrap!(unsafe 
        //     {self.radio.set_sleep(SleepCfg::new()
        //         .set_startup(Startup::Cold)
        //         .set_rtc_wakeup_en(false))
        //     }
        // );

        defmt::info!("power_on");
    }

    pub fn configure(&mut self) {
        unwrap!(self.radio.set_standby(StandbyClk::Rc));
        let status: Status = unwrap!(self.radio.status());
        defmt::info!("configure_radio: {} {}", status.cmd(), status.mode());
        //defmt::assert_ne!(status.cmd(), Ok(CmdStatus::ExecutionFailure));
        //defmt::assert_eq!(status.mode(), Ok(StatusMode::StandbyRc));
            
        unwrap!(self.radio.set_tcxo_mode(&self.config.txco));
        defmt::info!("set tcxo");

        unwrap!(self.radio.calibrate(0x7F));
        defmt::info!("set calibrate");

        unwrap!(self.radio.set_standby(StandbyClk::Hse));
        let status: Status = unwrap!(self.radio.status());
        defmt::info!("configure_radio: {} {}", status.cmd(), status.mode());
        
        let (status, errors) = unwrap!(self.radio.op_error());
        defmt::info!("configure_radio: {} error {:#X}", status, errors);

        unwrap!(self.radio.clear_error());

        unwrap!(self.radio.set_tx_rx_fallback_mode(FallbackMode::StandbyHse));    
        unwrap!(self.radio.set_regulator_mode(RegMode::Ldo));
        unwrap!(self.radio.set_buffer_base_address(0, 0));
        unwrap!(self.radio.set_packet_type(PacketType::LoRa));    
        unwrap!(self.radio.set_pa_config(&self.config.pa_cfg));
        unwrap!(self.radio.set_pa_ocp(Ocp::Max60m));    
        unwrap!(self.radio.set_tx_params(&self.config.tx_params));        
        unwrap!(self.radio.set_lora_sync_word(LoRaSyncWord::Public));
        unwrap!(self.radio.set_lora_mod_params(&self.config.mod_params));
        unwrap!(self.radio.set_rf_frequency(&self.config.frequency));
        
        // I did this on the c project??
        //unwrap!(self.radio.set_lora_symb_timeout(0));
        
        // set dio irq params - tx done, rx done, preamble, timeout, cmd error, error
        unwrap!(self.radio.set_irq_cfg(&self.config.irq_cfg));  
    
        let (status, irq_status) = self.radio.irq_status().unwrap();
        defmt::info!("configure_radio: {} {} irq: {}", status.cmd(), status.mode(), irq_status);    
        self.radio.clear_irq_status(0xFFFF).unwrap();

        // Start in RX mode
        self.do_receive();
    
    }

    // todo  figure out return type
    pub fn check_signal_strength(&mut self) -> String<100> {
        // check if in rx mode??

        let (stat, rssi) = self.radio.rssi_inst().unwrap();

        String::try_from(rssi.to_integer()).unwrap()
    }

    // Todo - figure out return type. Maybe Result<tbd>
    pub fn send_hello_world(&mut self) -> String<100> {
        // Load packet in buffer and bytes to send
        if self.do_transmit(b"Hello World", 11).is_ok() {
            String::try_from("Sent").unwrap()
        }
        else {
            String::try_from("Error").unwrap()
        }        
    }

    pub fn get_status(&mut self) -> String<100> {
        let (status, irq_status) = unwrap!(self.radio.irq_status());
        
        defmt::info!("get_status: status={} irq={:#X} busy={}", status, irq_status, subghz::rfbusys());

        if status.mode().is_ok() {
            match status.mode().unwrap() {
                StatusMode::Rx => { String::try_from("RX Mode").unwrap() }
                StatusMode::Tx => { String::try_from("TX Mode").unwrap() }
                StatusMode::StandbyHse => { String::try_from("Standby Mode").unwrap() }
                _ => { String::try_from("Unknown Mode").unwrap() }
            }
        }
        else {
            String::try_from("Error").unwrap()
        }        
    }

    // Radio interrupt handler
    pub fn locked_radio_irq_handler(&mut self) 
    {
        let (status, irq_status) = unwrap!(self.radio.irq_status());
        if irq_status & Irq::TxDone.mask() != 0 {
            defmt::info!("TxDone {}", status);
            defmt::assert_eq!(status.mode(), Ok(StatusMode::StandbyHse));
            unwrap!(self.radio.clear_irq_status(Irq::TxDone.mask()));

            // Go back to RX mode
            self.do_receive();
        } else if irq_status & Irq::PreambleDetected.mask() != 0 {
            defmt::info!("Preamble detect: {:#X} {}", irq_status, status);
            unwrap!(self.radio.clear_irq_status(Irq::PreambleDetected.mask()));        
        } else if irq_status & Irq::RxDone.mask() != 0 {
            let (_status, len, ptr) = unwrap!(self.radio.rx_buffer_status());
            defmt::info!("RxDone len={} ptr={} {} irq={:#X}", len, ptr, status, irq_status);

            // Todo - what to do with receieved packet??
            let mut data_read = [0; 255];
            unwrap!(self.radio.read_buffer( 0, &mut data_read ));
            defmt::info!("Buffer={}", data_read);

            unwrap!(self.radio.clear_irq_status(Irq::RxDone.mask()));            
        } else if irq_status & Irq::Timeout.mask() != 0 {
            defmt::warn!("Timeout {}", self.radio.op_error());
            unwrap!(self.radio.clear_irq_status(Irq::Timeout.mask()));
        } else if irq_status & Irq::Err.mask() != 0 {
            defmt::warn!("Packet error {}", self.radio.op_error());
            unwrap!(self.radio.clear_irq_status(Irq::Err.mask()));
        } else {
            defmt::error!("Unhandled IRQ: {:#X} {}", irq_status, status);
            unwrap!(self.radio.clear_irq_status(irq_status));
        }
    }

    //-----------------------------------------------------------
    // Private functions
    //----------------------------------------------------------- 

    fn do_transmit(&mut self, data: &[u8], len: u8) -> Result<(), &str>{
        // Take radio out of RX
        unwrap!(self.radio.set_standby(StandbyClk::Hse));

        // Set RF switch
        self.configure_rf_switch(RfSwitchType::RfSwitchTxHp);

        // Load packet in buffer and bytes to send
        unwrap!(self.radio.write_buffer(0, data));
        self.config.pkt_params = self.config.pkt_params.set_payload_len(len);
        unwrap!(self.radio.set_lora_packet_params(&self.config.pkt_params));

        // Start TX
        unwrap!(self.radio.set_tx(Timeout::DISABLED));

        Ok(())
    }

    fn do_receive(&mut self) {
        // Set for full length read
        self.config.pkt_params = self.config.pkt_params.set_payload_len(255);
        unwrap!(self.radio.set_lora_packet_params(&self.config.pkt_params));
        // Start read
        unwrap!(self.radio.set_rx(Timeout::DISABLED));
    }

    // Function to toggle GPIO's for RF switch specific to Lora-E5 hardware.
    // Wio-E5 module ONLY transmits through RFO_HP:
    // Receive: PA4=1, PA5=0
    // Transmit(high output power, SMPS mode): PA4=0, PA5=1
    fn configure_rf_switch(&mut self, mode: RfSwitchType) {
        match mode {
            RfSwitchType::RfSwitchOff => {
                self.gpio_rf_ctrl_1.set_level_low();
                self.gpio_rf_ctrl_2.set_level_low();
            },
            RfSwitchType::RfSwitchRx => {
                self.gpio_rf_ctrl_1.set_level_high();
                self.gpio_rf_ctrl_2.set_level_low();
            },
            RfSwitchType::RfSwitchTxLp => {
                self.gpio_rf_ctrl_1.set_level_high();
                self.gpio_rf_ctrl_2.set_level_high();
            },
            RfSwitchType::RfSwitchTxHp => {
                self.gpio_rf_ctrl_1.set_level_low();
                self.gpio_rf_ctrl_2.set_level_high();
            },
        }
    }
}
