/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Implementation of the AudioToolbox framework.

use crate::audio::openal::{OpenAL, OpenALContext, OpenALManager};

/// Macro used by AudioToolbox functions to reject NULL parameters.
///
/// This macro is exported at crate level so the AudioToolbox child
/// modules can use it.
#[macro_export]
macro_rules! return_if_null {
    ($param:ident) => {
        if $param.is_null() {
            log_dbg!(
                "Received NULL parameter {}, returning paramErr in {}:{}",
                stringify!($param),
                file!(),
                line!()
            );

            return crate::frameworks::carbon_core::paramErr;
        }
    };
}

// AudioToolbox submodules.

pub mod au_graph;
pub mod audio_components;
pub mod audio_converter;
pub mod audio_file;
pub mod audio_queue;
pub mod audio_services;
pub mod audio_session;
pub mod audio_unit;
pub mod ext_audio_file;

// BeniAudio support.
pub mod guestaudio;

/// Container for AudioToolbox state.
#[derive(Default)]
pub struct State {
    pub(crate) audio_file: audio_file::State,
    pub(crate) audio_queue: audio_queue::State,
    pub(crate) audio_components: audio_components::State,
    pub(crate) audio_services: audio_services::State,
    pub(crate) audio_session: audio_session::State,
    pub(crate) au_graph: au_graph::State,
    pub(crate) ext_audio_file: ext_audio_file::State,

    pub(crate) al_context: LazyALContext,
}

impl State {
    /// Makes the AudioToolbox OpenAL context current.
    pub fn make_al_context_current<'s, 'manager: 's>(
        &'s mut self,
        manager: &'manager mut OpenALManager,
    ) -> OpenAL<'s> {
        self.al_context.make_al_context_current(manager)
    }
}

/// Lazily-created OpenAL context used by AudioToolbox.
#[derive(Default)]
pub struct LazyALContext(Option<OpenALContext>);

impl LazyALContext {
    pub fn make_al_context_current<'s, 'manager: 's>(
        &'s mut self,
        manager: &'manager mut OpenALManager,
    ) -> OpenAL<'s> {
        self.get_context(manager).make_current(manager)
    }

    pub fn try_get_context(
        &mut self,
        manager: &mut OpenALManager,
    ) -> Option<&mut OpenALContext> {
        if self.0.is_none() {
            match OpenALContext::new(manager) {
                Ok(context) => {
                    log_dbg!(
                        "New internal OpenAL context for AudioToolbox ({:?})",
                        context
                    );

                    self.0 = Some(context);
                }

                Err(err) => {
                    log!(
                        "Warning: could not create OpenAL context for \
                         AudioToolbox: {}. Audio will be unavailable \
                         until a working backend is found.",
                        err
                    );

                    return None;
                }
            }
        }

        self.0.as_mut()
    }

    pub fn get_context(
        &mut self,
        manager: &mut OpenALManager,
    ) -> &mut OpenALContext {
        if self.try_get_context(manager).is_some() {
            // Context was successfully created.
        }

        self.0
            .as_mut()
            .expect(
                "OpenAL context unavailable; see prior log message for details"
            )
    }
}

/// AudioToolbox dynamic library.
pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/AudioToolbox.framework/AudioToolbox",

    aliases: &[
        "/usr/lib/libAudioToolbox.dylib",
        "AudioToolbox",
    ],

    class_exports: &[],

    constant_exports: &[],

    function_exports: &[
        au_graph::FUNCTIONS,
        audio_components::FUNCTIONS,
        audio_converter::FUNCTIONS,
        audio_file::FUNCTIONS,
        audio_queue::FUNCTIONS,
        audio_services::FUNCTIONS,
        audio_session::FUNCTIONS,
        audio_unit::FUNCTIONS,
        ext_audio_file::FUNCTIONS,
    ],
};
