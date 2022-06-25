use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&out_dir).join("gl_bindings.rs")).unwrap();
    Registry::new(
        Api::Gl,
        (3, 3),
        Profile::Core,
        Fallbacks::All,
        ["GL_ARB_blend_func_extended"],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();

    #[cfg(not(target_os = "windows"))]
    {
        let vlc_lib_path_dir = env::var_os("VLC_LIB_PATH")
            .expect("Setup VLC_LIB_PATH for your vlc lib path in order to link")
            .to_str()
            .unwrap()
            .to_owned();
        println!(r"cargo:rustc-link-search=native={}", vlc_lib_path_dir);
    }

    #[cfg(target_os = "windows")]
    {
        let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&dir).join("lib").to_str().unwrap().to_owned();
        println!(r"cargo:rustc-link-search=native={}", path);
    }
}
