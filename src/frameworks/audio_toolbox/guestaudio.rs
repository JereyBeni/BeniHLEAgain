//! Experimental BeniAudio microphone support.
//!
//! This module is currently used for crash testing.
//! AudioQueueNewInput intentionally returns an error instead of
//! pretending that a valid AudioQueue was created.

use crate::environment::Environment;
use crate::mem::Ptr;
use crate::objc::id;

/// Temporary error used while testing microphone initialization.
///
/// We do NOT create a real AudioQueue yet.
const BENI_AUDIO_MICROPHONE_UNAVAILABLE: i32 = -1;

// -----------------------------------------------------------------------------
// Audio Queue
// -----------------------------------------------------------------------------

pub fn audio_queue_new_input(
    _env: &mut Environment,
    _format: Ptr<(), false>,
    _callback: Ptr<(), false>,
    _user_data: Ptr<(), false>,
    _run_loop: id,
    _run_loop_mode: id,
    _flags: u32,
    _out_aq: Ptr<Ptr<(), false>, true>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueNewInput called - \
         microphone disabled for crash testing."
    );

    // IMPORTANT:
    // Do not return success here because no AudioQueue object exists.
    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

pub fn audio_queue_start(
    _env: &mut Environment,
    _aq: Ptr<(), false>,
    _start_time: Ptr<(), false>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueStart called while \
         microphone backend is disabled."
    );

    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

pub fn audio_queue_stop(
    _env: &mut Environment,
    _aq: Ptr<(), false>,
    _immediate: bool,
) -> i32 {
    log!("[BeniAudio] AudioQueueStop called.");

    0
}

pub fn audio_queue_dispose(
    _env: &mut Environment,
    _aq: Ptr<(), false>,
    _immediate: bool,
) -> i32 {
    log!("[BeniAudio] AudioQueueDispose called.");

    0
}

pub fn audio_queue_allocate_buffer(
    _env: &mut Environment,
    _aq: Ptr<(), false>,
    buffer_size: u32,
    _out_buffer: Ptr<Ptr<(), false>, true>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueAllocateBuffer called \
         (size={}) while microphone backend is disabled.",
        buffer_size
    );

    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

pub fn audio_queue_free_buffer(
    _env: &mut Environment,
    _aq: Ptr<(), false>,
    _buffer: Ptr<(), false>,
) -> i32 {
    log!("[BeniAudio] AudioQueueFreeBuffer called.");

    0
}

pub fn audio_queue_enqueue_buffer(
    _env: &mut Environment,
    _aq: Ptr<(), false>,
    _buffer: Ptr<(), false>,
    _num_packets: u32,
    _packet_descs: Ptr<(), false>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueEnqueueBuffer called \
         while microphone backend is disabled."
    );

    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

// -----------------------------------------------------------------------------
// Audio Services
// -----------------------------------------------------------------------------

pub fn audio_services_play_system_sound(
    _env: &mut Environment,
    sound_id: u32,
) {
    log!(
        "[BeniAudio] AudioServicesPlaySystemSound({})",
        sound_id
    );
}

pub fn audio_services_create_system_sound_id(
    _env: &mut Environment,
    _file_url: id,
    _out_sound_id: Ptr<u32, true>,
) -> i32 {
    log!(
        "[BeniAudio] AudioServicesCreateSystemSoundID called."
    );

    0
}

pub fn audio_services_dispose_system_sound_id(
    _env: &mut Environment,
    sound_id: u32,
) -> i32 {
    log!(
        "[BeniAudio] AudioServicesDisposeSystemSoundID({})",
        sound_id
    );

    0
}

// -----------------------------------------------------------------------------
// Audio Session
// -----------------------------------------------------------------------------

pub fn audio_session_initialize(
    _env: &mut Environment,
    _run_loop: Ptr<(), false>,
    _run_loop_mode: Ptr<(), false>,
    _interruption_listener: Ptr<(), false>,
    _user_data: Ptr<(), false>,
) -> i32 {
    log!("[BeniAudio] AudioSessionInitialize called.");

    0
}

pub fn audio_session_set_active(
    _env: &mut Environment,
    active: bool,
) -> i32 {
    log!(
        "[BeniAudio] AudioSessionSetActive({})",
        active
    );

    0
}

pub fn audio_session_set_property(
    _env: &mut Environment,
    property_id: u32,
    data_size: u32,
    _property_data: Ptr<(), false>,
) -> i32 {
    log!(
        "[BeniAudio] AudioSessionSetProperty(\
         property={}, size={})",
        property_id,
        data_size
    );

    0
}

pub fn audio_session_get_property(
    _env: &mut Environment,
    property_id: u32,
    _io_data_size: Ptr<u32, true>,
    _out_property_data: Ptr<(), true>,
) -> i32 {
    log!(
        "[BeniAudio] AudioSessionGetProperty({})",
        property_id
    );

    0
}
