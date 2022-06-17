use std::env;
use std::fs::File;
use std::path::Path;
use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&out_dir).join("gl_bindings.rs")).unwrap();
    Registry::new(Api::Gl, (3, 3), Profile::Core, Fallbacks::All, ["GL_ARB_blend_func_extended"])
        .write_bindings(GlobalGenerator, &mut file)
        .unwrap();
}
