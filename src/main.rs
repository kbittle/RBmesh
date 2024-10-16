// Blinks the LED on the LoRa-E5_STM32WLE5JC_Module.

#![no_std]
#![no_main]

use core::fmt::Write;
use defmt::unwrap;
use defmt_rtt as _; // global logger
use panic_probe as _; // panic handler
use stm32wlxx_hal::{
    self as hal,
    cortex_m::{self, delay::Delay},
    dma::{AllDma, Dma1Ch1, Dma2Ch1, Dma1Ch3, Dma2Ch6},
    embedded_hal::prelude::*,
    gpio::{pins, Output, PinState, PortA, PortB, PortC, Exti},
    info::{self, Package, Uid, Uid64},
    pac,
    rcc,
    rng::{self, Rng},
    spi::{SgMiso, SgMosi},
    subghz::{
        rfbusys, wakeup, AddrComp, CalibrateImage, CfgIrq, CmdStatus, CodingRate, CrcType,
        FallbackMode, FskBandwidth, FskBitrate, FskFdev, FskModParams, FskPulseShape,
        GenericPacketParams, HeaderType, Irq, LoRaBandwidth, LoRaModParams, LoRaPacketParams,
        LoRaSyncWord, Ocp, PaConfig, PacketType, PktCtrl, PreambleDetection, RampTime, RegMode,
        RfFreq, SleepCfg, SpreadingFactor, StandbyClk, Startup, Status, StatusMode, SubGhz,
        TcxoMode, TcxoTrim, Timeout, TxParams,
    },
    uart::{self, Uart1},
    util::new_delay,
};
use rtic::app;
use rtic_monotonics::systick::prelude::*;
use core::time::Duration;
use heapless::Vec; // fixed capacity `std::Vec`

mod at_command_handler;

systick_monotonic!(Mono, 1000);

const PREAMBLE_LEN: u16 = 16;
const DATA_LEN: u8 = 255;

const LORA_PACKET_PARAMS: LoRaPacketParams = LoRaPacketParams::new()
    .set_crc_en(true)
    .set_preamble_len(PREAMBLE_LEN)
    .set_payload_len(DATA_LEN)
    .set_invert_iq(false)
    .set_header_type(HeaderType::Fixed);

const LORA_MOD_PARAMS: LoRaModParams = LoRaModParams::new()
    .set_bw(LoRaBandwidth::Bw125)
    .set_cr(CodingRate::Cr45)
    .set_ldro_en(true)
    .set_sf(SpreadingFactor::Sf7);

const PA_CONFIG: PaConfig = PaConfig::LP_10;
const TX_PARAMS: TxParams = TxParams::LP_10.set_ramp_time(RampTime::Micros40);

const TCXO_MODE: TcxoMode = TcxoMode::new()
    .set_txco_trim(TcxoTrim::Volts1pt7)
    .set_timeout(Timeout::from_duration_sat(Duration::from_millis(10)));

fn configure_radio(sg: &mut SubGhz<Dma1Ch1, Dma2Ch1>) {
    unwrap!(sg.set_standby(StandbyClk::Rc));
    let status: Status = unwrap!(sg.status());
    defmt::assert_ne!(status.cmd(), Ok(CmdStatus::ExecutionFailure));
    defmt::assert_eq!(status.mode(), Ok(StatusMode::StandbyRc));

    unwrap!(sg.set_tcxo_mode(&TCXO_MODE));
    unwrap!(sg.set_standby(StandbyClk::Hse));
    let status: Status = unwrap!(sg.status());
    defmt::assert_ne!(status.cmd(), Ok(CmdStatus::ExecutionFailure));
    defmt::assert_eq!(status.mode(), Ok(StatusMode::StandbyHse));
    unwrap!(sg.set_tx_rx_fallback_mode(FallbackMode::StandbyHse));

    unwrap!(sg.set_regulator_mode(RegMode::Ldo));
    unwrap!(sg.set_buffer_base_address(0, 0));
    unwrap!(sg.set_pa_config(&PA_CONFIG));
    unwrap!(sg.set_pa_ocp(Ocp::Max60m));
    unwrap!(sg.set_tx_params(&TX_PARAMS));

    let status: Status = unwrap!(sg.status());
    defmt::assert_eq!(status.mode(), Ok(StatusMode::StandbyHse));

    unwrap!(sg.set_lora_sync_word(LoRaSyncWord::Public));
    unwrap!(sg.set_lora_mod_params(&LORA_MOD_PARAMS));
    unwrap!(sg.set_lora_packet_params(&LORA_PACKET_PARAMS));
}

fn write_uart1(uart1: &mut Uart1<pins::B7, (pins::B6, Dma1Ch3)>)
{
    // test func to write device number
    let devnum: u32 = info::Uid64::read_devnum();

    unwrap!(uart1.bwrite_all(&devnum.to_be_bytes()));
}

fn locked_radio_irq_handler(sg: &mut SubGhz<Dma1Ch1, Dma2Ch1> ) 
{
    let (status, irq_status) = unwrap!(sg.irq_status());

        if irq_status & Irq::TxDone.mask() != 0 {
            defmt::info!("TxDone {}", status);
            defmt::assert_eq!(status.mode(), Ok(StatusMode::StandbyHse));
            unwrap!(sg.clear_irq_status(Irq::TxDone.mask()));

        } else if irq_status & Irq::RxDone.mask() != 0 {
            let (_status, len, ptr) = unwrap!(sg.rx_buffer_status());
            defmt::info!("RxDone len={} ptr={} {}", len, ptr, status);
            //unwrap!(sg.read_buffer(
            //    ptr,
            //    &mut bytemuck::bytes_of_mut::<[u32; 64]>(buf)[..usize::from(len)]
            //));
            unwrap!(sg.clear_irq_status(Irq::RxDone.mask()));            
        } else if irq_status & Irq::Timeout.mask() != 0 {
            unwrap!(sg.clear_irq_status(Irq::Timeout.mask()));
            defmt::error!(
                "server did not respond to time sync request in {}",
                Timeout::from_duration_sat(Duration::from_millis(100)).as_duration()
            );
            // clear nonce
            //*time_sync_nonce = 0;
            // restart timer
            //lptim1.start(u16::MAX);
        } else if irq_status & Irq::Err.mask() != 0 {
            defmt::warn!("Packet error {}", sg.fsk_packet_status());
            unwrap!(sg.clear_irq_status(Irq::Err.mask()));
        } else {
            defmt::error!("Unhandled IRQ: {:#X} {}", irq_status, status);
            unwrap!(sg.clear_irq_status(irq_status));
        }
}

#[app(device = stm32wlxx_hal::pac, peripherals = true, dispatchers = [SPI1])]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        sg: SubGhz<Dma1Ch1, Dma2Ch1>,
        uart1: Uart1<pins::B7, (pins::B6, Dma1Ch3)>,
        rx_buffer: Vec<u8, 512>, // Buffer to store received byte
        at_cmd_resp_inst: at_command_handler::AtCmdResp,
        exti: pac::EXTI,
    }

    #[local]
    struct Local {
        led1: Output<pins::C0>,
        state: bool,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut dp: pac::Peripherals = ctx.device;

        // symptom of a version mismatch when using the RTIC alpha
        // see: https://github.com/rust-embedded/cortex-m/pull/350
        // replace with `ctx.cs` when cortex-m gets updated
        // KKB: This was reported in 2021, no fixed??
        let cs = unsafe { &hal::cortex_m::interrupt::CriticalSection::new() };
        unsafe {
            rcc::set_sysclk_msi(
                &mut dp.FLASH,
                &mut dp.PWR,
                &mut dp.RCC,
                rcc::MsiRange::Range48M,
                cs,
            )
        };
        
        // Initialize the systick interrupt & obtain the token to prove that we did
        Mono::start(ctx.core.SYST, 36_000_000); // default STM32F303 clock-rate is 36MHz

        
        let dma: AllDma = AllDma::split(dp.DMAMUX, dp.DMA1, dp.DMA2, &mut dp.RCC);

        // Setup GPIO
        let gpioa: PortA = PortA::split(dp.GPIOA, &mut dp.RCC);
        let gpiob: PortB = PortB::split(dp.GPIOB, &mut dp.RCC);
        let gpioc: PortC = PortC::split(dp.GPIOC, &mut dp.RCC);

        // Setup LED
        let mut led1: Output<pins::C0> = Output::default(gpioc.c0, cs);
        led1.set_level(PinState::High);

        // Setup uart1
        let mut uart1: Uart1<pins::B7, (pins::B6, Dma1Ch3)> =
            Uart1::new(dp.USART1, 115200, uart::Clk::Hsi16, &mut dp.RCC)
                .enable_rx(gpiob.b7, cs)
                .enable_tx_dma(gpiob.b6, dma.d1.c3, cs);

        // Test write function
        write_uart1(&mut uart1);

        // Setup LoRa-E5 specific gpio's
        let mut txcoPwr: Output<pins::B0> = Output::default(gpiob.b0, cs);
        let mut rfCtrl1: Output<pins::A4> = Output::default(gpioa.a4, cs);
        let mut rfCtrl2: Output<pins::A5> = Output::default(gpioa.a5, cs);

        let mut sg: SubGhz<Dma1Ch1, Dma2Ch1> = SubGhz::new_with_dma(dp.SPI3, dma.d1.c1, dma.d2.c1, &mut dp.RCC);

        let exti = dp.EXTI;

        configure_radio(&mut sg);

        // Schedule the blinking task
        blinkTask::spawn().unwrap();

        // Setup rx buffer
        let rx_buffer: Vec<u8, 512> = Vec::new();
        
        let at_cmd_resp_inst = at_command_handler::AtCmdResp::default();

        (
            Shared 
            { 
                sg, 
                uart1, 
                rx_buffer,
                at_cmd_resp_inst,
                exti
            }, 
            Local { led1, state: false }
        )
    }

    #[task(
        local = [led1, state]
    )]
    async fn blinkTask(cx: blinkTask::Context) {
        loop {
            //rprintln!("blink");
            if *cx.local.state {
                cx.local.led1.set_level_high();
                *cx.local.state = false;
            } else {
                cx.local.led1.set_level_low();
                *cx.local.state = true;
            }
            Mono::delay(1000.millis()).await;
        }
    }

    #[task(
        binds = USART1, 
        shared = [uart1, rx_buffer, at_cmd_resp_inst]
    )]
    fn usart1_interrupt(mut ctx: usart1_interrupt::Context) {
        (
            ctx.shared.uart1,
            ctx.shared.rx_buffer,
            ctx.shared.at_cmd_resp_inst,
        )
        .lock(|uart1, rx_buffer, at_cmd_resp_inst| {
            if let Ok(received_byte) = uart1.read() {
                // Store the received byte in the buffer
                rx_buffer.push(received_byte).unwrap();

                // TODO - move rx buffer inside cmd resp
                //      - if ahndle cmd retrn true, figure out how to queue response 

                // Here you could also handle the byte directly (e.g., process it)
                at_cmd_resp_inst.handle_command(rx_buffer);
            }
        });
    }

    #[task(
        binds = RADIO_IRQ_BUSY,
        shared = [sg],
    )]
    fn radioIrqHandler(mut ctx: radioIrqHandler::Context)
    {
        ctx.shared.sg.lock(|sg| {
            locked_radio_irq_handler(sg)
        });        
    }
}