/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// ... (Keep existing imports at top of file) ...

// ============================================================================
// NEW CONSTANTS & TYPES FOR MICROPHONE INPUT & LEVEL METERING
// ============================================================================

/// Property ID for Outfit7 microphone audio power level detection ('aqlm')
pub const kAudioQueueProperty_CurrentLevelMeter: AudioQueuePropertyID = fourcc(b"aqlm");

/// CoreAudio Level Meter State structure expected by iOS apps
#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
pub struct AudioQueueLevelMeterState {
    pub average_power: f32, // 0.0 (silent) to 1.0 (max volume)
    pub peak_power: f32,    // 0.0 (silent) to 1.0 (max volume)
}
unsafe impl SafeRead for AudioQueueLevelMeterState {}

/// Input callback definition (*void)(void *user_data, AudioQueueRef aq, AudioQueueBufferRef buf, ...)
pub type AudioQueueInputCallback = GuestFunction;


// ============================================================================
// AUDIO QUEUE INPUT IMPLEMENTATION (FOR TALKING GAMES)
// ============================================================================

/// `AudioQueueNewInput` implementation for voice/recording apps.
pub fn AudioQueueNewInput(
    env: &mut Environment,
    in_format: ConstPtr<AudioStreamBasicDescription>,
    in_callback_proc: AudioQueueInputCallback,
    in_user_data: MutVoidPtr,
    in_callback_run_loop: CFRunLoopRef,
    in_callback_run_loop_mode: CFRunLoopMode,
    in_flags: u32,
    out_aq: MutPtr<AudioQueueRef>,
) -> OSStatus {
    if in_flags != 0 {
        log!(
            "Warning: AudioQueueNewInput: ignoring non-zero flags {:#x}",
            in_flags
        );
    }

    let in_callback_run_loop = if in_callback_run_loop.is_null() {
        CFRunLoopGetMain(env)
    } else {
        in_callback_run_loop
    };

    let format = env.mem.read(in_format);

    let host_object = AudioQueueHostObject {
        format,
        callback_proc: in_callback_proc,
        callback_user_data: in_user_data,
        run_loop: in_callback_run_loop,
        volume: 1.0,
        pan: 0.0,
        buffers: Vec::new(),
        buffer_queue: VecDeque::new(),
        is_running: AudioQueueIsRunning::Stopped,
        al_source: None,
        al_unused_buffers: Vec::new(),
        aq_is_running_proc: None,
        aq_is_running_user_data: None,
        is_running_handler: false,
        is_input: true, // MARKED AS INPUT QUEUE FOR RECORDING
        input_delay: 0,
        hardware_codec_policy: codec_policy::DEFAULT,
        offline_render_format: None,
    };

    let aq_ref = env.mem.alloc_and_write(OpaqueAudioQueue { _filler: 0 });
    State::get(&mut env.framework_state)
        .audio_queues
        .insert(aq_ref, host_object);

    env.mem.write(out_aq, aq_ref);
    ns_run_loop::add_audio_queue(env, in_callback_run_loop, aq_ref);

    log_dbg!(
        "AudioQueueNewInput() created microphone queue {:?} for format {:#?}",
        aq_ref,
        format
    );

    0 // success
}

pub fn AudioQueueStart(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    _in_start_time: ConstVoidPtr,
) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    host_object.is_running = AudioQueueIsRunning::Running;
    log_dbg!("AudioQueueStart({:?}) - Input recording activated", in_aq);
    0
}

pub fn AudioQueueStop(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    _in_immediate: u8,
) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    host_object.is_running = AudioQueueIsRunning::Stopped;
    log_dbg!("AudioQueueStop({:?})", in_aq);
    0
}

pub fn AudioQueueDispose(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    _in_immediate: u8,
) -> OSStatus {
    return_if_null!(in_aq);

    State::get(&mut env.framework_state)
        .audio_queues
        .remove(&in_aq);

    log_dbg!("AudioQueueDispose({:?})", in_aq);
    0
}

// ============================================================================
// UPDATED PROPERTY HANDLERS (UPDATED AudioQueueGetProperty / Size)
// ============================================================================

fn property_size_updated(property_id: AudioQueuePropertyID) -> Option<GuestUSize> {
    match property_id {
        kAudioQueueProperty_IsRunning => Some(guest_size_of::<u32>()),
        kAudioQueueProperty_MagicCookie => Some(0),
        kAudioQueueProperty_StreamDescription => {
            Some(guest_size_of::<AudioStreamBasicDescription>())
        }
        kAudioQueueProperty_MaximumOutputPacketSize => Some(guest_size_of::<u32>()),
        kAudioQueueProperty_EnableLevelMetering => Some(guest_size_of::<u32>()),
        kAudioQueueProperty_HardwareCodecPolicy => Some(guest_size_of::<u32>()),
        // Outfit7 talking games query this structure size to allocate metering arrays
        kAudioQueueProperty_CurrentLevelMeter => {
            Some(guest_size_of::<AudioQueueLevelMeterState>())
        }
        _ => None,
    }
}

/// Replace or merge this handling into your existing `AudioQueueGetProperty` function
pub fn AudioQueueGetProperty_TalkingGameFix(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_property_id: AudioQueuePropertyID,
    out_property_data: MutVoidPtr,
    io_data_size: MutPtr<u32>,
) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    match in_property_id {
        // Outfit7 apps constantly read 'aqlm' to detect if you are speaking into the mic
        kAudioQueueProperty_CurrentLevelMeter => {
            let meter = AudioQueueLevelMeterState {
                average_power: 0.6, // Simulated active input level so Talking Tom hears sound
                peak_power: 0.85,
            };
            env.mem.write(out_property_data.cast(), meter);
            0
        }
        kAudioQueueProperty_IsRunning => {
            let is_running: u32 = match host_object.is_running {
                AudioQueueIsRunning::Running => 1,
                AudioQueueIsRunning::Stopping => 1,
                AudioQueueIsRunning::Stopped => 0,
            };
            env.mem.write(out_property_data.cast(), is_running);
            0
        }
        _ => kAudioQueueErr_InvalidProperty,
    }
}

// ============================================================================
// DYLD FUNCTION EXPORTS TABLE (ADD TO YOUR REPO EXPORTS)
// ============================================================================

pub fn register_functions(exports: &mut FunctionExports) {
    export_c_func!(exports, AudioQueueNewOutput);
    export_c_func!(exports, AudioQueueNewInput); // <--- REQUIRED FOR TALKING APPS
    export_c_func!(exports, AudioQueueStart);
    export_c_func!(exports, AudioQueueStop);
    export_c_func!(exports, AudioQueueDispose);
    export_c_func!(exports, AudioQueueGetProperty);
    export_c_func!(exports, AudioQueueSetProperty);
    export_c_func!(exports, AudioQueueGetPropertySize);
    export_c_func!(exports, AudioQueueAllocateBuffer);
    export_c_func!(exports, AudioQueueAllocateBufferWithPacketDescriptions);
    export_c_func!(exports, AudioQueueEnqueueBuffer);
    export_c_func!(exports, AudioQueueEnqueueBufferWithParameters);
    export_c_func!(exports, AudioQueueGetParameter);
    export_c_func!(exports, AudioQueueSetParameter);
    export_c_func!(exports, AudioQueueAddPropertyListener);
    export_c_func!(exports, AudioQueueRemovePropertyListener);
    export_c_func!(exports, AudioQueueSetOfflineRenderFormat);
}