/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::VecDeque;

use crate::cpu::GuestFunction;
use crate::frameworks::audio_toolbox::audio_converter::*;
use crate::frameworks::audio_toolbox::core_audio_types::*;
use crate::frameworks::core_foundation::cf_run_loop::*;
use crate::frameworks::foundation::ns_run_loop;
use crate::mem::*;
use crate::os::environment::Environment;
use crate::os::framework_state::FrameworkState;
use crate::os::host_object::*;
use crate::util::fourcc;

// ============================================================================
// TYPES & CONSTANTS
// ============================================================================

pub type OSStatus = i32;
pub type AudioQueuePropertyID = u32;
pub type AudioQueueParamID = u32;
pub type AudioQueueParamValue = f32;

pub const kAudioQueueErr_InvalidProperty: OSStatus = -66673;
pub const kAudioQueueErr_BufferEmpty: OSStatus = -66674;
pub const kAudioQueueErr_DisposalPending: OSStatus = -66675;
pub const kAudioQueueErr_InvalidPath: OSStatus = -66676;
pub const kAudioQueueErr_InvalidPropertyValue: OSStatus = -66677;

pub const kAudioQueueParam_Volume: AudioQueueParamID = 1;
pub const kAudioQueueParam_Pan: AudioQueueParamID = 2;

pub const kAudioQueueProperty_IsRunning: AudioQueuePropertyID = fourcc(b"aqrn");
pub const kAudioQueueProperty_MagicCookie: AudioQueuePropertyID = fourcc(b"aqmc");
pub const kAudioQueueProperty_StreamDescription: AudioQueuePropertyID = fourcc(b"aqft");
pub const kAudioQueueProperty_MaximumOutputPacketSize: AudioQueuePropertyID = fourcc(b"xops");
pub const kAudioQueueProperty_EnableLevelMetering: AudioQueuePropertyID = fourcc(b"aqme");
pub const kAudioQueueProperty_CurrentLevelMeter: AudioQueuePropertyID = fourcc(b"aqlm");
pub const kAudioQueueProperty_HardwareCodecPolicy: AudioQueuePropertyID = fourcc(b"aqcp");

pub mod codec_policy {
    pub const DEFAULT: u32 = 0;
    pub const HARDWARE_ONLY: u32 = 1;
    pub const SOFTWARE_ONLY: u32 = 2;
    pub const HARDWARE_PREFERRED: u32 = 3;
}

#[repr(C)]
pub struct OpaqueAudioQueue {
    pub _filler: u32,
}
unsafe impl SafeRead for OpaqueAudioQueue {}
unsafe impl SafeWrite for OpaqueAudioQueue {}

pub type AudioQueueRef = MutPtr<OpaqueAudioQueue>;

#[repr(C)]
pub struct AudioQueueBuffer {
    pub audio_data_bytes_capacity: u32,
    pub audio_data: MutVoidPtr,
    pub audio_data_byte_size: u32,
    pub user_data: MutVoidPtr,
    pub packet_description_capacity: u32,
    pub packet_descriptions: MutPtr<AudioStreamPacketDescription>,
    pub packet_description_count: u32,
}
unsafe impl SafeRead for AudioQueueBuffer {}
unsafe impl SafeWrite for AudioQueueBuffer {}

pub type AudioQueueBufferRef = MutPtr<AudioQueueBuffer>;

pub type AudioQueueOutputCallback = GuestFunction;
pub type AudioQueueInputCallback = GuestFunction;

#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
pub struct AudioQueueLevelMeterState {
    pub average_power: f32,
    pub peak_power: f32,
}
unsafe impl SafeRead for AudioQueueLevelMeterState {}
unsafe impl SafeWrite for AudioQueueLevelMeterState {}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AudioQueueIsRunning {
    Stopped = 0,
    Running = 1,
    Stopping = 2,
}

pub struct AudioQueueHostObject {
    pub format: AudioStreamBasicDescription,
    pub callback_proc: GuestFunction,
    pub callback_user_data: MutVoidPtr,
    pub run_loop: CFRunLoopRef,
    pub volume: f32,
    pub pan: f32,
    pub buffers: Vec<AudioQueueBufferRef>,
    pub buffer_queue: VecDeque<AudioQueueBufferRef>,
    pub is_running: AudioQueueIsRunning,
    pub al_source: Option<u32>,
    pub al_unused_buffers: Vec<u32>,
    pub aq_is_running_proc: Option<GuestFunction>,
    pub aq_is_running_user_data: Option<MutVoidPtr>,
    pub is_running_handler: bool,
    pub is_input: bool,
    pub input_delay: u64,
    pub hardware_codec_policy: u32,
    pub offline_render_format: Option<AudioStreamBasicDescription>,
}

pub struct State {
    pub audio_queues: HostObjectMap<AudioQueueRef, AudioQueueHostObject>,
}

impl State {
    pub fn get(framework_state: &mut FrameworkState) -> &mut Self {
        framework_state
            .audio_toolbox
            .get_or_insert_with(|| Self {
                audio_queues: HostObjectMap::new(),
            })
    }
}

// ============================================================================
// HELPER FUNCTIONS EXPORTED TO OTHER MODULES
// ============================================================================

/// Decodes an audio queue buffer for playback processing.
pub fn decode_buffer(
    _env: &mut Environment,
    _format: &AudioStreamBasicDescription,
    _buffer: AudioQueueBufferRef,
) -> Vec<u8> {
    // Decoding implementation stub or passthrough
    Vec::new()
}

/// Helper function used to check and log invalid audio formats.
pub fn log_if_broken_audio_format(format: &AudioStreamBasicDescription) {
    if format.sample_rate <= 0.0 || format.channels_per_frame == 0 {
        log!("Warning: Detected broken/unsupported AudioStreamBasicDescription: {:#?}", format);
    }
}

// ============================================================================
// AUDIO QUEUE API IMPLEMENTATION
// ============================================================================

pub fn AudioQueueNewOutput(
    env: &mut Environment,
    in_format: ConstPtr<AudioStreamBasicDescription>,
    in_callback_proc: AudioQueueOutputCallback,
    in_user_data: MutVoidPtr,
    in_callback_run_loop: CFRunLoopRef,
    in_callback_run_loop_mode: CFRunLoopMode,
    in_flags: u32,
    out_aq: MutPtr<AudioQueueRef>,
) -> OSStatus {
    if in_flags != 0 {
        log!("Warning: AudioQueueNewOutput: ignoring flags {:#x}", in_flags);
    }

    let in_callback_run_loop = if in_callback_run_loop.is_null() {
        CFRunLoopGetMain(env)
    } else {
        in_callback_run_loop
    };

    let format = env.mem.read(in_format);
    log_if_broken_audio_format(&format);

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
        is_input: false,
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

    log_dbg!("AudioQueueNewOutput() created queue {:?} for format {:#?}", aq_ref, format);
    0
}

pub fn AudioQueueNewInput(
    env: &mut Environment,
    in_format: ConstPtr<AudioStreamBasicDescription>,
    in_callback_proc: AudioQueueInputCallback,
    in_user_data: MutVoidPtr,
    in_callback_run_loop: CFRunLoopRef,
    _in_callback_run_loop_mode: CFRunLoopMode,
    in_flags: u32,
    out_aq: MutPtr<AudioQueueRef>,
) -> OSStatus {
    if in_flags != 0 {
        log!("Warning: AudioQueueNewInput: ignoring non-zero flags {:#x}", in_flags);
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
        is_input: true,
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

    log_dbg!("AudioQueueNewInput() created microphone queue {:?}", aq_ref);
    0
}

pub fn AudioQueueAllocateBuffer(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer_byte_size: u32,
    out_buffer: MutPtr<AudioQueueBufferRef>,
) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    let data_ptr = env.mem.alloc(in_buffer_byte_size as usize);
    let buf_struct = AudioQueueBuffer {
        audio_data_bytes_capacity: in_buffer_byte_size,
        audio_data: data_ptr,
        audio_data_byte_size: 0,
        user_data: MutVoidPtr::null(),
        packet_description_capacity: 0,
        packet_descriptions: MutPtr::null(),
        packet_description_count: 0,
    };

    let buf_ref = env.mem.alloc_and_write(buf_struct);
    host_object.buffers.push(buf_ref);
    env.mem.write(out_buffer, buf_ref);

    0
}

pub fn AudioQueueAllocateBufferWithPacketDescriptions(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer_byte_size: u32,
    in_number_packet_descriptions: u32,
    out_buffer: MutPtr<AudioQueueBufferRef>,
) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    let data_ptr = env.mem.alloc(in_buffer_byte_size as usize);
    let packet_descs_ptr = env.mem.alloc(
        (in_number_packet_descriptions as usize) * std::mem::size_of::<AudioStreamPacketDescription>(),
    );

    let buf_struct = AudioQueueBuffer {
        audio_data_bytes_capacity: in_buffer_byte_size,
        audio_data: data_ptr,
        audio_data_byte_size: 0,
        user_data: MutVoidPtr::null(),
        packet_description_capacity: in_number_packet_descriptions,
        packet_descriptions: packet_descs_ptr.cast(),
        packet_description_count: 0,
    };

    let buf_ref = env.mem.alloc_and_write(buf_struct);
    host_object.buffers.push(buf_ref);
    env.mem.write(out_buffer, buf_ref);

    0
}

pub fn AudioQueueEnqueueBuffer(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer: AudioQueueBufferRef,
    in_num_packet_descs: u32,
    _in_packet_descs: ConstPtr<AudioStreamPacketDescription>,
) -> OSStatus {
    return_if_null!(in_aq);
    return_if_null!(in_buffer);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    let mut buf = env.mem.read(in_buffer);
    buf.packet_description_count = in_num_packet_descs;
    env.mem.write(in_buffer, buf);

    host_object.buffer_queue.push_back(in_buffer);
    0
}

pub fn AudioQueueEnqueueBufferWithParameters(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer: AudioQueueBufferRef,
    in_num_packet_descs: u32,
    in_packet_descs: ConstPtr<AudioStreamPacketDescription>,
    _in_trim_frames_at_start: u32,
    _in_trim_frames_at_end: u32,
    _in_num_param_values: u32,
    _in_param_values: ConstVoidPtr,
    _in_start_time: ConstVoidPtr,
    _out_actual_start_time: MutVoidPtr,
) -> OSStatus {
    AudioQueueEnqueueBuffer(
        env,
        in_aq,
        in_buffer,
        in_num_packet_descs,
        in_packet_descs,
    )
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
    0
}

pub fn AudioQueuePause(env: &mut Environment, in_aq: AudioQueueRef) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    host_object.is_running = AudioQueueIsRunning::Stopped;
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

    0
}

pub fn AudioQueueGetParameter(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_param_id: AudioQueueParamID,
    out_value: MutPtr<AudioQueueParamValue>,
) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    match in_param_id {
        kAudioQueueParam_Volume => env.mem.write(out_value, host_object.volume),
        kAudioQueueParam_Pan => env.mem.write(out_value, host_object.pan),
        _ => return kAudioQueueErr_InvalidProperty,
    }

    0
}

pub fn AudioQueueSetParameter(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_param_id: AudioQueueParamID,
    in_value: AudioQueueParamValue,
) -> OSStatus {
    return_if_null!(in_aq);

    let Some(host_object) = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
    else {
        return kAudioQueueErr_InvalidProperty;
    };

    match in_param_id {
        kAudioQueueParam_Volume => host_object.volume = in_value,
        kAudioQueueParam_Pan => host_object.pan = in_value,
        _ => return kAudioQueueErr_InvalidProperty,
    }

    0
}

pub fn AudioQueueGetProperty(
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
        kAudioQueueProperty_CurrentLevelMeter => {
            let meter = AudioQueueLevelMeterState {
                average_power: 0.6,
                peak_power: 0.85,
            };
            env.mem.write(out_property_data.cast(), meter);
            env.mem.write(io_data_size, std::mem::size_of::<AudioQueueLevelMeterState>() as u32);
            0
        }
        kAudioQueueProperty_IsRunning => {
            let is_running: u32 = match host_object.is_running {
                AudioQueueIsRunning::Running => 1,
                AudioQueueIsRunning::Stopping => 1,
                AudioQueueIsRunning::Stopped => 0,
            };
            env.mem.write(out_property_data.cast(), is_running);
            env.mem.write(io_data_size, 4);
            0
        }
        kAudioQueueProperty_StreamDescription => {
            env.mem.write(out_property_data.cast(), host_object.format);
            env.mem.write(
                io_data_size,
                std::mem::size_of::<AudioStreamBasicDescription>() as u32,
            );
            0
        }
        _ => kAudioQueueErr_InvalidProperty,
    }
}

pub fn AudioQueueSetProperty(
    _env: &mut Environment,
    _in_aq: AudioQueueRef,
    _in_property_id: AudioQueuePropertyID,
    _in_property_data: ConstVoidPtr,
    _in_property_data_size: u32,
) -> OSStatus {
    0
}

pub fn AudioQueueGetPropertySize(
    _env: &mut Environment,
    _in_aq: AudioQueueRef,
    in_property_id: AudioQueuePropertyID,
    out_property_data_size: MutPtr<u32>,
) -> OSStatus {
    let size = match in_property_id {
        kAudioQueueProperty_IsRunning => std::mem::size_of::<u32>(),
        kAudioQueueProperty_StreamDescription => std::mem::size_of::<AudioStreamBasicDescription>(),
        kAudioQueueProperty_CurrentLevelMeter => std::mem::size_of::<AudioQueueLevelMeterState>(),
        _ => return kAudioQueueErr_InvalidProperty,
    };

    _env.mem.write(out_property_data_size, size as u32);
    0
}

pub fn AudioQueueAddPropertyListener(
    _env: &mut Environment,
    _in_aq: AudioQueueRef,
    _in_property_id: AudioQueuePropertyID,
    _in_proc: GuestFunction,
    _in_user_data: MutVoidPtr,
) -> OSStatus {
    0
}

pub fn AudioQueueRemovePropertyListener(
    _env: &mut Environment,
    _in_aq: AudioQueueRef,
    _in_property_id: AudioQueuePropertyID,
    _in_proc: GuestFunction,
    _in_user_data: MutVoidPtr,
) -> OSStatus {
    0
}

pub fn AudioQueueSetOfflineRenderFormat(
    _env: &mut Environment,
    _in_aq: AudioQueueRef,
    _in_format: ConstPtr<AudioStreamBasicDescription>,
    _in_layout: ConstVoidPtr,
) -> OSStatus {
    0
}

// ============================================================================
// DYLD REGISTRATION EXPORTS
// ============================================================================

pub fn register_functions(exports: &mut crate::dyld::FunctionExports) {
    export_c_func!(exports, AudioQueueNewOutput);
    export_c_func!(exports, AudioQueueNewInput);
    export_c_func!(exports, AudioQueueStart);
    export_c_func!(exports, AudioQueuePause);
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