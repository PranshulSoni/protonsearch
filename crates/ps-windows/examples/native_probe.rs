//! Native adapter host probe: prints the capability matrix + live device data.
//! Used to produce docs/security/NATIVE_ADAPTER_SUPPORT_MATRIX.md.
//!
//!   cargo run -p ps-windows --example native_probe

fn main() {
    let capabilities = ps_windows::native_capabilities();
    println!("=== native capability matrix");
    println!("{}", serde_json::to_string_pretty(&capabilities).unwrap_or_else(|_| format!("{capabilities:#?}")));

    println!("=== audio outputs");
    match ps_windows::list_audio_outputs() {
        Ok(devices) => {
            for device in &devices {
                println!(
                    "device default={} name={:?} id={:?}",
                    device.is_default, device.name, device.id
                );
            }
            match ps_windows::audio_state() {
                Ok(state) => println!("audio state volume={:.2} muted={}", state.volume, state.muted),
                Err(error) => println!("audio state unavailable: {error:#}"),
            }
        }
        Err(error) => println!("audio outputs unavailable: {error:#}"),
    }

    println!("=== brightness");
    if ps_windows::brightness_supported() {
        println!("monitors: {:?}", ps_windows::brightness_monitors());
        println!("current brightness: {:?}", ps_windows::get_brightness());
    } else {
        println!("brightness unsupported on this host");
    }

    println!("=== notifications");
    println!("local store: supported; toast supported={}", ps_windows::toast_supported());
}
