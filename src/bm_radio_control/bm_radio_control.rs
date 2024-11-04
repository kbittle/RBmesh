use defmt::{unwrap, write};
use defmt_rtt as _; // global logger
use core::{default, time::Duration};
use stm32wlxx_hal::{
    gpio::{pins, Output},
    spi::{SgMiso, SgMosi},
    subghz::{
        self, rfbusys, wakeup, AddrComp, CalibrateImage, CfgIrq, CmdStatus, CodingRate, CrcType,
        FallbackMode, 
        GenericPacketParams, HeaderType, Irq, IrqLine, LoRaBandwidth, LoRaModParams, LoRaPacketParams,
        LoRaSyncWord, Ocp, PaConfig, PaSel, PacketType, PktCtrl, PreambleDetection, RampTime, RegMode,
        RfFreq, SleepCfg, SpreadingFactor, StandbyClk, Startup, Status, StatusMode, SubGhz,
        TcxoMode, TcxoTrim, Timeout, TxParams,
    }, Ratio,
};
use heapless::{String, Vec};
use super::bm_radio_rx_buffer::{RadioRxBuffer, RADIO_MAX_BUFF_SIZE};

const PREAMBLE_LEN: u16 = 16;
const DATA_LEN: u8 = 255;
const RADIO_RX_BUFFER_SIZE: usize = 3;

enum RfSwitchType {
    RfSwitchOff,
    RfSwitchRx,
    RfSwitchTxLp,
    RfSwitchTxHp
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum RadioState {
    #[default]
    Idle,
    Receiving,
    Transmitting,
    Failure,
}

impl defmt::Format for RadioState {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            RadioState::Idle => write!(fmt, "Idle"),
            RadioState::Receiving => write!(fmt, "Receiving"),
            RadioState::Transmitting => write!(fmt, "Transmitting"),
            RadioState::Failure => write!(fmt, "Failure"),
            _ => { write!(fmt, "Unknown") }
        }
    }
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
    // Radio HAL library
    radio: SubGhz<SgMiso, SgMosi>,
    // GPIO's specific to the LoRa-E5 module
    gpio_txco_pwr: Output<pins::B0>,
    gpio_rf_ctrl_1: Output<pins::A4>,
    gpio_rf_ctrl_2: Output<pins::A5>,
    // LoRa radio configurations
    config: RadioConfiguration,
    // LoRa radio receieve queue
    pub rx_buffer: Vec<RadioRxBuffer, RADIO_RX_BUFFER_SIZE>,
    // Tx/Rx/Idle state
    pub current_state: RadioState,
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
            rx_buffer: Vec::new(),
            current_state: RadioState::default(),
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

        let (_stat, rssi) = self.radio.rssi_inst().unwrap();

        String::try_from(rssi.to_integer()).unwrap()
    }

    // Todo - figure out return type. Maybe Result<tbd>
    pub fn send_test_message(&mut self) -> String<100> {
        // Load packet in buffer and bytes to send
        if self.do_transmit(b"Hello World", 11).is_ok() {
            String::try_from("Sent").unwrap()
        }
        else {
            String::try_from("Error").unwrap()
        }        
    }

    pub fn send_packet(&mut self, length: u8, payload: &[u8]) -> Result<(), &str> {
        self.do_transmit(payload, length)
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

            self.current_state = RadioState::Idle;

            // Go back to RX mode
            self.do_receive();
        } else if irq_status & Irq::PreambleDetected.mask() != 0 {
            defmt::info!("Preamble detect: {:#X} {}", irq_status, status);
            unwrap!(self.radio.clear_irq_status(Irq::PreambleDetected.mask())); 

            // Set state to receiving to prevent transmitting while radio is mid rx.
            // Need to add timeout before enabling this feature
            //self.current_state = RadioState::Receiving;       
        } else if irq_status & Irq::RxDone.mask() != 0 {
            let (_status, len, ptr) = unwrap!(self.radio.rx_buffer_status());
            defmt::info!("RxDone len={} ptr={} {} irq={:#X}", len, ptr, status, irq_status);

            self.current_state = RadioState::Idle;

            // move this to preamble???
            let rx_rssi = self.radio.rssi_inst();

            if self.rx_buffer.len() >= RADIO_RX_BUFFER_SIZE {
                defmt::error!("Receive buffer is full!");
                return
            }

            // Store in some rx buffer, dont do processing in irq handler
            let mut receieved_buffer = RadioRxBuffer::new()
                .with_len(len)
                .with_rssi(rx_rssi.unwrap().1.to_integer());
            // Read data from radio into RadioRxBuffer
            unwrap!(self.radio.read_buffer( 0, &mut receieved_buffer.buffer ));
            // If the read succeeds push buffer in shared memory space
            self.rx_buffer.push(receieved_buffer).unwrap();
            // Clear IRQ
            unwrap!(self.radio.clear_irq_status(Irq::RxDone.mask()));            
        } else if irq_status & Irq::Timeout.mask() != 0 {
            defmt::warn!("Timeout {}", self.radio.op_error());
            unwrap!(self.radio.clear_irq_status(Irq::Timeout.mask()));

            // Flag failure
            self.current_state = RadioState::Failure;
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

    fn do_transmit(&mut self, data: &[u8], len: u8) -> Result<(), &str> {
        // Take radio out of RX
        unwrap!(self.radio.set_standby(StandbyClk::Hse));

        // Set RF switch
        self.configure_rf_switch(RfSwitchType::RfSwitchTxHp);

        // Load packet in buffer and bytes to send
        unwrap!(self.radio.write_buffer(0, data));
        self.config.pkt_params = self.config.pkt_params.set_payload_len(len);
        unwrap!(self.radio.set_lora_packet_params(&self.config.pkt_params));

        // Start TX
        unwrap!(self.radio.set_tx(Timeout::from_duration_sat(Duration::from_secs(15))));

        // Set internal state
        self.current_state = RadioState::Transmitting;

        Ok(())
    }

    fn do_receive(&mut self) {
        // Set rf switch
        self.configure_rf_switch(RfSwitchType::RfSwitchRx);
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
