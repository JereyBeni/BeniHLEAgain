//! BeniAudio experimental microphone support.
//!
//! This module is intentionally kept separate from the original
//! AudioToolbox implementation while microphone support is being tested.
//
//! IMPORTANT:
//! AudioQueueNewInput currently returns an error instead of pretending
//! that a valid AudioQueue was created. This prevents Unity from receiving
//! a NULL/invalid AudioQueue handle and subsequently crashing.

use crate::environment::Environment;
use crate::mem::Ptr;
use crate::objc::id;

/// Generic AudioToolbox error used for the temporary microphone test.
///
/// We intentionally use a non-zero error code here. The important part of
/// this test is that AudioQueueNewInput does NOT report success when no
/// actual AudioQueue object exists.
const BENI_AUDIO_MICROPHONE_UNAVAILABLE: i32 = -1;

/// Experimental AudioQueueNewInput implementation.
///
/// At the moment this function does not create a real audio queue.
/// Returning an error allows Unity to gracefully disable microphone input
/// instead of receiving a successful result with an invalid queue handle.
pub fn audio_queue_new_input(
    _env: &mut Environment,
    _format: Ptr<()>,
    _callback: Ptr<()>,
    _user_data: Ptr<()>,
    _run_loop: id,
    _run_loop_mode: id,
    _flags: u32,
    _out_aq: Ptr<Ptr<()>>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueNewInput called - \
         microphone temporarily disabled for crash testing."
    );

    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

/// Experimental AudioQueueStart implementation.
///
/// This should not normally be reached while AudioQueueNewInput is
/// returning an error.
pub fn audio_queue_start(
    _env: &mut Environment,
    _aq: Ptr<()>,
    _start_time: Ptr<()>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueStart called while microphone backend \
         is disabled."
    );

    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

/// Experimental AudioQueueStop implementation.
pub fn audio_queue_stop(
    _env: &mut Environment,
    _aq: Ptr<()>,
    _immediate: bool,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueStop called while microphone backend \
         is disabled."
    );

    0
}

/// Experimental AudioQueueDispose implementation.
pub fn audio_queue_dispose(
    _env: &mut Environment,
    _aq: Ptr<()>,
    _immediate: bool,
) -> i32 {
    log!("[BeniAudio] AudioQueueDispose called.");

    0
}

/// Experimental AudioQueueAllocateBuffer implementation.
///
/// This does not allocate a real AudioQueue buffer yet.
pub fn audio_queue_allocate_buffer(
    _env: &mut Environment,
    _aq: Ptr<()>,
    _buffer_size: u32,
    _out_buffer: Ptr<Ptr<()>>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueAllocateBuffer called \
         (size: {}) while microphone backend is disabled.",
        _buffer_size
    );

    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

/// Experimental AudioQueueFreeBuffer implementation.
pub fn audio_queue_free_buffer(
    _env: &mut Environment,
    _aq: Ptr<()>,
    _buffer: Ptr<()>,
) -> i32 {
    log!("[BeniAudio] AudioQueueFreeBuffer called.");

    0
}

/// Experimental AudioQueueEnqueueBuffer implementation.
pub fn audio_queue_enqueue_buffer(
    _env: &mut Environment,
    _aq: Ptr<()>,
    _buffer: Ptr<()>,
    _num_packets: u32,
    _packet_descs: Ptr<()>,
) -> i32 {
    log!(
        "[BeniAudio] AudioQueueEnqueueBuffer called \
         while microphone backend is disabled."
    );

    BENI_AUDIO_MICROPHONE_UNAVAILABLE
}

/// AudioServicesPlaySystemSound stub.
///
/// This is kept here for future BeniAudio system-sound support.
pub fn audio_services_play_system_sound(
    _env: &mut Environment,
    sound_id: u32,
) {
    log!(
        "[BeniAudio] AudioServicesPlaySystemSound({})",
        sound_id
    );
}

/// AudioServicesCreateSystemSoundID stub.
pub fn audio_services_create_system_sound_id(
    _env: &mut Environment,
    _file_url: id,
    _out_sound_id: Ptr<u32>,
) -> i32 {
    log!(
        "[BeniAudio] AudioServicesCreateSystemSoundID called."
    );

    0
}

/// AudioServicesDisposeSystemSoundID stub.
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

/// AudioSessionInitialize stub.
pub fn audio_session_initialize(
    _env: &mut Environment,
    _run_loop: Ptr<()>,
    _run_loop_mode: Ptr<()>,
    _interruption_listener: Ptr<()>,
    _user_data: Ptr<()>,
) -> i32 {
    log!("[BeniAudio] AudioSessionInitialize called.");

    0
}

/// AudioSessionSetActive stub.
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

/// AudioSessionSetProperty stub.
pub fn audio_session_set_property(
    _env: &mut Environment,
    property_id: u32,
    data_size: u32,
    _property_data: Ptr<()>,
) -> i32 {
    log!(
        "[BeniAudio] AudioSessionSetProperty(\
         property={}, size={})",
        property_id,
        data_size
    );

    0
}

/// AudioSessionGetProperty stub.
pub fn audio_session_get_property(
    _env: &mut Environment,
    property_id: u32,
    _io_data_size: Ptr<u32>,
    _out_property_data: Ptr<()>,
) -> i32 {
    log!(
        "[BeniAudio] AudioSessionGetProperty({})",
        property_id
    );

    0
}
