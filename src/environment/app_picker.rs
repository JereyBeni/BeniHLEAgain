//! App picker GUI con estética Glossy Retro Vibe & Neon Accents.
//! Re-diseñado para BeniHLEAgain respetando el diseño Clean-Room MPL 2.0.

use crate::bundle::Bundle;
use crate::frameworks::core_graphics::cg_bitmap_context::{
    CGBitmapContextCreate, CGBitmapContextCreateImage,
};
use crate::frameworks::core_graphics::cg_color_space::CGColorSpaceCreateDeviceRGB;
use crate::frameworks::core_graphics::cg_context::{
    CGContextFillRect, CGContextRelease, CGContextScaleCTM, CGContextSetRGBFillColor,
    CGContextTranslateCTM,
};
use crate::frameworks::core_graphics::cg_image::{self, kCGImageAlphaPremultipliedLast};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_run_loop::run_run_loop_single_iteration;
use crate::frameworks::foundation::ns_string;
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_font::{
    UITextAlignmentCenter, UITextAlignmentLeft, UITextAlignmentRight,
};
use crate::frameworks::uikit::ui_graphics::{UIGraphicsPopContext, UIGraphicsPushContext};
use crate::frameworks::uikit::ui_view::ui_control::ui_button::{
    UIButtonTypeCustom, UIButtonTypeRoundedRect,
};
use crate::frameworks::uikit::ui_view::ui_control::{
    UIControlEventTouchUpInside, UIControlEventValueChanged, UIControlStateNormal,
};
use crate::fs::BundleData;
use crate::image::Image;
use crate::mem::Ptr;
use crate::objc::{id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject};
use crate::options::Options;
use crate::paths;
use crate::window::DeviceOrientation;
use crate::Environment;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

struct AppInfo {
    path: PathBuf,
    display_name: String,
    icon: Option<Image>,
    display_name_ns_string: Option<id>,
    icon_ui_image: Option<id>,
}

pub fn app_picker(options: Options) -> Result<(PathBuf, Vec<String>), String> {
    let apps_dir = paths::user_data_base_path().join(paths::APPS_DIR);

    let apps: Result<Vec<AppInfo>, String> = if !apps_dir.is_dir() {
        Err(format!("🔥 ¡Ojo bro! No existe la carpeta {}. Creala e inyectale tus .ipa/.app de Talking Friends.", apps_dir.display()))
    } else {
        enumerate_apps(&apps_dir)
            .map_err(|err| format!("Error escaneando {}: {}.", apps_dir.display(), err))
            .and_then(|apps| {
                if apps.is_empty() {
                    Err(format!("💀 No hay apps ni juegos en {}. ¡Meté unos binarios ahí!", apps_dir.display()))
                } else {
                    Ok(apps)
                }
            })
    };

    show_app_picker_gui(options, apps)
}

fn enumerate_apps(apps_dir: &Path) -> Result<Vec<AppInfo>, std::io::Error> {
    let mut apps = Vec::new();
    for app in std::fs::read_dir(apps_dir)? {
        let app_path = app?.path();
        if app_path.extension() != Some(OsStr::new("app"))
            && app_path.extension() != Some(OsStr::new("ipa"))
        {
            continue;
        }

        let (bundle, fs) = match BundleData::open_any(&app_path).and_then(|bundle_data| {
            Bundle::new_bundle_and_fs_from_host_path(bundle_data, true)
        }) {
            Ok(ok) => ok,
            Err(e) => {
                log!("Warning: No se pudo abrir el bundle {}: {} (skipping)", app_path.display(), e);
                continue;
            }
        };

        let display_name = bundle.display_name().to_owned();
        let icon = match bundle.load_icon(&fs) {
            Ok(icon) => Some(icon),
            Err(e) => {
                log!("Warning: Sin icono para {}: {}", app_path.display(), e);
                None
            }
        };

        apps.push(AppInfo {
            path: app_path,
            display_name,
            icon,
            display_name_ns_string: None,
            icon_ui_image: None,
        });
    }

    apps.sort_by_key(|app| app.display_name.to_uppercase());
    Ok(apps)
}

#[derive(Default)]
struct AppPickerDelegateHostObject {
    icon_tapped: id,
    copyright_show: bool,
    copyright_hide: bool,
    copyright_prev: bool,
    copyright_next: bool,
    quick_options_show: bool,
    quick_options_hide: bool,
    scale_hack_default: bool,
    scale_hack1: bool,
    scale_hack2: bool,
    scale_hack3: bool,
    scale_hack4: bool,
    orientation_default: bool,
    orientation_landscape_left: bool,
    orientation_landscape_right: bool,
    orientation_portrait_upside_down: bool,
    analog_stick_tilt_controls: Option<bool>,
    network: Option<bool>,
    fullscreen: Option<bool>,
    device_model_tag: Option<i32>,
    device_model_toggle: bool,
    device_model_scroll_up: bool,
    device_model_scroll_down: bool,
}
impl HostObject for AppPickerDelegateHostObject {}

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/.touchHLE/AppPickerHelpers.dylib",
    aliases: &[],
    class_exports: &[CLASSES],
    constant_exports: &[],
    function_exports: &[],
};

const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);

@implementation _touchHLE_AppPickerDelegate: NSObject

- (())iconTapped:(id)sender {
    let host_obj = env.objc.borrow_mut::<AppPickerDelegateHostObject>(this);
    host_obj.icon_tapped = sender;
}

- (())copyrightInfoShow { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_show = true; }
- (())copyrightInfoHide { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_hide = true; }
- (())copyrightInfoPrevPage { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_prev = true; }
- (())copyrightInfoNextPage { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).copyright_next = true; }

- (())quickOptionsShow { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).quick_options_show = true; }
- (())quickOptionsHide { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).quick_options_hide = true; }
- (())scaleHackDefault { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack_default = true; }
- (())scaleHack1 { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack1 = true; }
- (())scaleHack2 { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack2 = true; }
- (())scaleHack3 { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack3 = true; }
- (())scaleHack4 { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).scale_hack4 = true; }
- (())orientationDefault { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_default = true; }
- (())orientationLandscapeLeft { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_landscape_left = true; }
- (())orientationLandscapeRight { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_landscape_right = true; }
- (())orientationPortraitUpsideDown { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).orientation_portrait_upside_down = true; }

- (())analogStickTiltControls:(id)switch {
    let switch_state: bool = msg![env; switch isOn];
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).analog_stick_tilt_controls = Some(switch_state);
}
- (())network:(id)switch {
    let switch_state: bool = msg![env; switch isOn];
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).network = Some(switch_state);
}
- (())fullscreen:(id)switch {
    let switch_state: bool = msg![env; switch isOn];
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).fullscreen = Some(switch_state);
}
- (())deviceModel:(id)sender {
    let tag: NSInteger = msg![env; sender tag];
    env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).device_model_tag = Some(tag as i32);
}
- (())deviceModelToggle { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).device_model_toggle = true; }
- (())deviceModelScrollUp { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).device_model_scroll_up = true; }
- (())deviceModelScrollDown { env.objc.borrow_mut::<AppPickerDelegateHostObject>(this).device_model_scroll_down = true; }

- (())openFileManager {
    let _ = env.objc.borrow_mut::<AppPickerDelegateHostObject>(this);
    if let Ok(url) = paths::url_for_opening_user_data_dir() {
        if crate::window::open_url(env, &url).is_ok() {
            std::process::exit(0);
        }
    }
}

- (())visitWebsite {
    let _ = env.objc.borrow_mut::<AppPickerDelegateHostObject>(this);
    let url = ns_string::get_static_str(env, "https://github.com/JereyBeni/BeniHLEAgain");
    let url: id = msg_class![env; NSURL URLWithString:url];
    let ui_application: id = msg_class![env; UIApplication sharedApplication];
    assert!(msg![env; ui_application openURL:url]);
}

@end
};

fn show_app_picker_gui(
    options: Options,
    apps: Result<Vec<AppInfo>, String>,
) -> Result<(PathBuf, Vec<String>), String> {
    let icon = {
        let bytes: &[u8] = match crate::branding() {
            "" => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/icon.png")),
            "UNOFFICIAL" => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/icon_unofficial.png")),
            _ => include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/icon_preview.png")),
        };
        let mut image = Image::from_bytes(bytes).unwrap();
        image.round_corners((10.0 / 57.0) * (image.dimensions().0 as f32), true, true);
        image
    };
    let environment = Environment::new_without_app(options, icon)?;
    Ok(environment.run_app_picker(|env| app_picker_inner(env, apps)))
}

fn app_picker_inner(
    env: &mut Environment,
    mut apps: Result<Vec<AppInfo>, String>,
) -> (PathBuf, Vec<String>) {
    let ui_application: id = msg_class![env; UIApplication new];
    let delegate = env.objc.get_known_class("_touchHLE_AppPickerDelegate", &mut env.mem);
    let delegate = env.objc.alloc_object(delegate, Box::<AppPickerDelegateHostObject>::default(), &mut env.mem);
    () = msg![env; ui_application setDelegate:delegate];

    let screen: id = msg_class![env; UIScreen mainScreen];
    let bounds: CGRect = msg![env; screen bounds];
    let window: id = msg_class![env; UIWindow alloc];
    let window: id = msg![env; window initWithFrame:bounds];

    let app_frame: CGRect = msg![env; screen applicationFrame];
    let main_view: id = msg_class![env; UIView alloc];
    let main_view: id = msg![env; main_view initWithFrame:app_frame];
    () = msg![env; window addSubview:main_view];

    // Fondo Neon & Watermark Estilo Gamer
    let bg_color: id = msg_class![env; UIColor blackColor];
    () = msg![env; main_view setBackgroundColor:bg_color];

    // Header Vibe / Title Label
    let title_frame = CGRect {
        origin: CGPoint { x: 0.0, y: 15.0 },
        size: CGSize { width: app_frame.size.width, height: 30.0 },
    };
    let title_label: id = msg_class![env; UILabel alloc];
    let title_label: id = msg![env; title_label initWithFrame:title_frame];
    let title_text = ns_string::from_rust_string(env, "BeniHLEAgain // App Launcher".to_string());
    () = msg![env; title_label setText:title_text];
    () = msg![env; title_label setTextAlignment:UITextAlignmentCenter];
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:(18.0 as CGFloat)];
    () = msg![env; title_label setFont:font];
    let neon_color: id = msg_class![env; UIColor cyanColor];
    () = msg![env; title_label setTextColor:neon_color];
    () = msg![env; main_view addSubview:title_label];

    // Placeholder final para retornar configuración
    (PathBuf::new(), Vec::new())
    }
