use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::env;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

#[allow(dead_code)]
fn get_output_path() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    let path = Path::new(&manifest_dir_string)
        .join("target")
        .join(build_type);
    return PathBuf::from(path);
}

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
        println!(r"cargo:rustc-link-search={}", vlc_lib_path_dir);
        println!(r"cargo:rustc-link-lib=vlc");
    }

    #[cfg(target_os = "windows")]
    {
        let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(&dir).join("lib").to_str().unwrap().to_owned();
        println!(r"cargo:rustc-link-search=native={}", path);
    }
}
