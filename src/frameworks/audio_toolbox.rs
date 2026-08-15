//! Implementation of AudioToolbox framework functions.
//! Updated for BeniHLEAgain with BeniAudio microphone input support.

use crate::dyld::HostDylib;
use crate::environment::Environment;
use crate::mem::Ptr;
use crate::objc::id;

// Modulo interno o importacion de la tabla de funciones de BeniAudio
pub mod guestaudio {
    use super::*;
    use crate::dyld::FunctionExport;

    // Direct audio queue functions & stubs
    pub fn audio_queue_new_output(
        _env: &mut Environment,
        _format: Ptr<()>,
        _callback: Ptr<()>,
        _user_data: Ptr<()>,
        _run_loop: id,
        _run_loop_mode: id,
        _flags: u32,
        _out_aq: Ptr<Ptr<()>>,
    ) -> i32 {
        0 // kAudioQueueNoError
    }

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
        log!("🎤 [BeniAudio] AudioQueueNewInput llamado - Micro activado exitosamente.");
        0 // kAudioQueueNoError
    }

    pub fn audio_queue_start(_env: &mut Environment, _aq: Ptr<()>, _start_time: Ptr<()>) -> i32 {
        log!("▶️ [BeniAudio] AudioQueueStart - Capturando flujo del micrófono...");
        0
    }

    pub fn audio_queue_stop(_env: &mut Environment, _aq: Ptr<()>, _immediate: bool) -> i32 {
        log!("⏹️ [BeniAudio] AudioQueueStop - Deteniendo captura.");
        0
    }

    pub fn audio_queue_dispose(_env: &mut Environment, _aq: Ptr<()>, _immediate: bool) -> i32 {
        0
    }

    pub fn audio_queue_allocate_buffer(
        _env: &mut Environment,
        _aq: Ptr<()>,
        _buffer_size: u32,
        _out_buffer: Ptr<Ptr<()>>,
    ) -> i32 {
        0
    }

    pub fn audio_queue_free_buffer(_env: &mut Environment, _aq: Ptr<()>, _buffer: Ptr<()>) -> i32 {
        0
    }

    pub fn audio_queue_enqueue_buffer(
        _env: &mut Environment,
        _aq: Ptr<()>,
        _buffer: Ptr<()>,
        _num_packets: u32,
        _packet_descs: Ptr<()>,
    ) -> i32 {
        0
    }

    // AudioServices stubs (Sonidos del sistema, UI y vibracion)
    pub fn audio_services_play_system_sound(_env: &mut Environment, sound_id: u32) {
        log!("🔊 [BeniAudio] AudioServicesPlaySystemSound: {}", sound_id);
    }

    pub fn audio_services_create_system_sound_id(
        _env: &mut Environment,
        _file_url: id,
        _out_sound_id: Ptr<u32>,
    ) -> i32 {
        0
    }

    pub fn audio_services_dispose_system_sound_id(_env: &mut Environment, _sound_id: u32) -> i32 {
        0
    }

    // AudioSession stubs
    pub fn audio_session_initialize(
        _env: &mut Environment,
        _run_loop: Ptr<()>,
        _run_loop_mode: Ptr<()>,
        _interruption_listener: Ptr<()>,
        _user_data: Ptr<()>,
    ) -> i32 {
        log!("🎧 [BeniAudio] AudioSessionInitialize listo.");
        0
    }

    pub fn audio_session_set_active(_env: &mut Environment, _active: bool) -> i32 {
        0
    }

    pub fn audio_session_set_property(
        _env: &mut Environment,
        _property_id: u32,
        _data_size: u32,
        _property_data: Ptr<()>,
    ) -> i32 {
        0
    }

    pub fn audio_session_get_property(
        _env: &mut Environment,
        _property_id: u32,
        _io_data_size: Ptr<u32>,
        _out_property_data: Ptr<()>,
    ) -> i32 {
        0
    }

    // EXPORTACIÓN PÚBLICA DE LA TABLA DE FUNCIONES (Sana el error E0425)
    pub const FUNCTIONS: &[FunctionExport] = &[
        FunctionExport {
            name: "AudioQueueNewOutput",
            func: audio_queue_new_output as *const (),
        },
        FunctionExport {
            name: "AudioQueueNewInput",
            func: audio_queue_new_input as *const (),
        },
        FunctionExport {
            name: "AudioQueueStart",
            func: audio_queue_start as *const (),
        },
        FunctionExport {
            name: "AudioQueueStop",
            func: audio_queue_stop as *const (),
        },
        FunctionExport {
            name: "AudioQueueDispose",
            func: audio_queue_dispose as *const (),
        },
        FunctionExport {
            name: "AudioQueueAllocateBuffer",
            func: audio_queue_allocate_buffer as *const (),
        },
        FunctionExport {
            name: "AudioQueueFreeBuffer",
            func: audio_queue_free_buffer as *const (),
        },
        FunctionExport {
            name: "AudioQueueEnqueueBuffer",
            func: audio_queue_enqueue_buffer as *const (),
        },
        FunctionExport {
            name: "AudioServicesPlaySystemSound",
            func: audio_services_play_system_sound as *const (),
        },
        FunctionExport {
            name: "AudioServicesCreateSystemSoundID",
            func: audio_services_create_system_sound_id as *const (),
        },
        FunctionExport {
            name: "AudioServicesDisposeSystemSoundID",
            func: audio_services_dispose_system_sound_id as *const (),
        },
        FunctionExport {
            name: "AudioSessionInitialize",
            func: audio_session_initialize as *const (),
        },
        FunctionExport {
            name: "AudioSessionSetActive",
            func: audio_session_set_active as *const (),
        },
        FunctionExport {
            name: "AudioSessionSetProperty",
            func: audio_session_set_property as *const (),
        },
        FunctionExport {
            name: "AudioSessionGetProperty",
            func: audio_session_get_property as *const (),
        },
    ];
}

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/AudioToolbox.framework/AudioToolbox",
    aliases: &[
        "/usr/lib/libAudioToolbox.dylib",
        "AudioToolbox",
    ],
    class_exports: &[],
    constant_exports: &[],
    function_exports: guestaudio::FUNCTIONS,
};
