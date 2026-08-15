// src/frameworks/audio_toolbox/guestaudio.rs

pub type GuestAddr = u32;

/// Estructura que mapea el formato PCM esperado por el juego en iOS
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AudioStreamBasicDescription {
    pub sample_rate: f64,
    pub format_id: u32,
    pub format_flags: u32,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub bytes_per_frame: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
    pub reserved: u32,
}

/// Handler HLE para AudioQueueNewInput
pub fn audio_queue_new_input(
    _format: GuestAddr,
    _callback: GuestAddr,
    _user_data: GuestAddr,
    _run_loop: GuestAddr,
    _run_loop_mode: GuestAddr,
    _flags: u32,
    _out_aq: GuestAddr,
) -> i32 {
    // Retorna 0 (kAudioQueueErr_NoError) para simular éxito
    0
}

/// Handler HLE para iniciar la grabación
pub fn audio_queue_start(_aq: GuestAddr, _start_time: GuestAddr) -> i32 {
    0
}

/// Handler HLE para detener la grabación
pub fn audio_queue_stop(_aq: GuestAddr, _immediate: u8) -> i32 {
    0
}
