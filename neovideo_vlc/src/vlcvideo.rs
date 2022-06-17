// use super::{support::gl, support::Gl};
use std::sync::{Arc, Mutex};
use std::{ffi::CStr, panic};

mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

use super::vlc::{
	libvlc_instance_t, libvlc_media_new_location, libvlc_media_player_new_from_media,
	libvlc_media_player_play, libvlc_media_player_release, libvlc_media_player_t,
	libvlc_media_release, libvlc_media_t, libvlc_new, libvlc_release,
	libvlc_video_color_primaries_t, libvlc_video_color_space_t, libvlc_video_engine_t,
	libvlc_video_orient_t, libvlc_video_output_cfg_t, libvlc_video_render_cfg_t,
	libvlc_video_set_output_callbacks, libvlc_video_setup_device_cfg_t,
	libvlc_video_setup_device_info_t, libvlc_video_transfer_func_t,
};
use glutin::event_loop::EventLoopWindowTarget;
use glutin::{dpi::PhysicalSize, Context, ContextBuilder, GlProfile, NotCurrent, PossiblyCurrent};

use libc::c_void;

enum SharedContext {
	Current(Context<PossiblyCurrent>),
	NotCurrent(Context<NotCurrent>),
}

const VS_SRC: &'static [u8] = b"
#version 410
in vec2 a_position;
in vec2 a_uv;
out vec2 v_TexCoordinate;
void main()
{
	v_TexCoordinate = a_uv;
	gl_Position = vec4(a_position, 0.0, 1.0);
}
\0";

const FS_SRC: &'static [u8] = b"
#version 410
uniform sampler2D u_videotex;
in vec2 v_TexCoordinate;
out vec4 outColor;
void main()
{
 outColor = texture2D(u_videotex, v_TexCoordinate);
}
\0";

#[rustfmt::skip]
static RECT_DATA: [f32; 16] = [
	0f32, 0.0f32, 0f32, 1.0f32,
	0f32, -0.95f32, 0f32, 0f32,
	1f32, 0f32, 1f32, 1f32,
	1f32, -0.95f32, 1f32, 0f32,
];

pub struct TextureRender {
	vao: u32,
	program: u32,
	tex_uniform: i32,
}

macro_rules! glchk {
	($($s:stmt;)*) => {
		$(
			$s
			if cfg!(debug_assertions) {
				let err = gl::GetError();
				if err != gl::NO_ERROR {
					let err_str = match err {
						gl::INVALID_ENUM => "GL_INVALID_ENUM",
						gl::INVALID_VALUE => "GL_INVALID_VALUE",
						gl::INVALID_OPERATION => "GL_INVALID_OPERATION",
						gl::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
						gl::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
						_ => "unknown error"
					};
					println!("{}:{} - {} caused {}",
							 file!(),
							 line!(),
							 stringify!($s),
							 err_str);
				}
			}
		)*
	};
}

fn print_shader_info(shader: u32) {
	unsafe {
		let mut len: i32 = std::mem::zeroed();
		gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
		if len < 1 {
			return;
		}
		let mut count: i32 = std::mem::zeroed();
		let mut infos = vec![0u8; len as usize + 100];
		gl::GetShaderInfoLog(shader, len, &mut count, infos.as_mut_ptr() as *mut i8);
		println!("Shader Info: {:?}", String::from_utf8(infos));
		let mut status: i32 = gl::TRUE as i32;
		gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
		if status == gl::FALSE as i32 {
			println!("compile shader failed");
		}
	}
}

impl TextureRender {
	#[inline]
	pub fn draw_video_frame(&self, tex: u32) {
		unsafe {
			println!("draw tex: {}, vao: {}, program: {}", tex, self.vao, self.program);
			glchk!(
				gl::Disable(gl::BLEND);
				gl::BindVertexArray(self.vao);
				gl::UseProgram(self.program);
				gl::ActiveTexture(gl::TEXTURE4);
				gl::Uniform1i(self.tex_uniform, 4);
				gl::BindTexture(gl::TEXTURE_2D, tex);
				gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
				gl::BindTexture(gl::TEXTURE_2D, 0);
				gl::Enable(gl::BLEND);
			);
		}
	}
	pub fn new(window_context: &Context<PossiblyCurrent>) -> TextureRender {
		unsafe {
            gl::load_with(|sym|window_context.get_proc_address(sym) as *const _);
			let mut max_attribs = 0;
			gl::GetIntegerv(gl::MAX_VERTEX_ATTRIBS, &mut max_attribs);
			println!("MAX_VERTEX_ATTRIBS: {}", max_attribs);
			let program = {
				let vs = gl::CreateShader(gl::VERTEX_SHADER);
				gl::ShaderSource(vs, 1, [VS_SRC.as_ptr() as *const _].as_ptr(), std::ptr::null());
				gl::CompileShader(vs);

				let fs = gl::CreateShader(gl::FRAGMENT_SHADER);
				gl::ShaderSource(fs, 1, [FS_SRC.as_ptr() as *const _].as_ptr(), std::ptr::null());
				gl::CompileShader(fs);

				print_shader_info(vs);
				print_shader_info(fs);

				let program = gl::CreateProgram();
				gl::AttachShader(program, vs);
				gl::AttachShader(program, fs);
				gl::LinkProgram(program);
				gl::DeleteShader(vs);
				gl::DeleteShader(fs);
				program
			};

			let mut vao: u32 = std::mem::zeroed();
			glchk!(
				gl::GenVertexArrays(1, &mut vao);
				gl::BindVertexArray(vao);

			let mut vbo: u32 = std::mem::zeroed();
			gl::GenBuffers(1, &mut vbo);
			gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
			gl::BufferData(
				gl::ARRAY_BUFFER,
				(std::mem::size_of::<f32>() * RECT_DATA.len()) as gl::types::GLsizeiptr,
				RECT_DATA.as_ptr() as *const _,
				gl::STATIC_DRAW,
			);

			let pos_attrib =
				gl::GetAttribLocation(program, b"a_position\0".as_ptr() as *const _) as u32;
			gl::EnableVertexAttribArray(pos_attrib);
			gl::VertexAttribPointer(
				pos_attrib,
				2,
				gl::FLOAT,
				0,
				4 * std::mem::size_of::<f32>() as gl::types::GLsizei,
				std::ptr::null_mut(),
			);

			let uv_attrib = gl::GetAttribLocation(program, b"a_uv\0".as_ptr() as *const _) as u32;
			gl::EnableVertexAttribArray(uv_attrib);
			gl::VertexAttribPointer(
				uv_attrib,
				2,
				gl::FLOAT,
				0,
				4 * std::mem::size_of::<f32>() as gl::types::GLsizei,
				8 as *const c_void,
			);

			gl::UseProgram(program);
			let tex_uniform = gl::GetUniformLocation(program, b"u_videotex\0".as_ptr() as *const _);
			gl::ActiveTexture(gl::TEXTURE4);
			gl::Uniform1i(tex_uniform, 4);

			);

			println!("tex_uniform: {}", tex_uniform);
			TextureRender { vao, program, tex_uniform }
		}
	}
}

pub struct VLCVideo {
	vlc: *mut libvlc_instance_t,
	player: *mut libvlc_media_player_t,
	media: *mut libvlc_media_t,
	shared_context: Option<SharedContext>,
	textures: [u32; 3],
	fbo: [u32; 3],
	idx_render: usize,
	idx_swap: usize,
	idx_display: usize,
	update: Arc<Mutex<bool>>,
	width: u32,
	height: u32,
}

impl Drop for VLCVideo {
	fn drop(&mut self) {
		self.stop();
		unsafe {
			if self.vlc as *mut _ != std::ptr::null_mut() {
				libvlc_release(self.vlc);
			}
		}
	}
}

impl VLCVideo {
	pub fn new<T>(
		window_context: &Context<PossiblyCurrent>,
		el: &EventLoopWindowTarget<T>,
	) -> VLCVideo {
		unsafe {
			let vlc = libvlc_new(0, std::ptr::null_mut());
			let shared_context = ContextBuilder::new()
				.with_gl_profile(GlProfile::Compatibility)
				.with_shared_lists(window_context)
				.build_headless(el, PhysicalSize::new(1920, 1080))
				.unwrap();

			let current_context = shared_context.make_current().unwrap();

            gl::load_with(|sym|current_context.get_proc_address(sym) as *const _);

			let shared_context = current_context.make_not_current().unwrap();
			VLCVideo {
				vlc,
				player: std::ptr::null_mut(),
				media: std::ptr::null_mut(),
				shared_context: Some(SharedContext::NotCurrent(shared_context)),
				textures: [0; 3],
				fbo: [0; 3],
				idx_render: 0usize,
				idx_swap: 1usize,
				idx_display: 2usize,
				update: Arc::new(Mutex::new(false)),
				width: 0u32,
				height: 0u32,
			}
		}
	}

	#[inline]
	pub fn stop(&mut self) {
		unsafe {
			if self.player != std::ptr::null_mut() {
				libvlc_media_player_release(self.player);
				self.player = std::ptr::null_mut();
			}
			if self.media != std::ptr::null_mut() {
				libvlc_media_release(self.media);
				self.media = std::ptr::null_mut();
			}
		}
	}

	#[inline]
	pub fn get_video_frame(&mut self, update: &mut bool) -> u32 {
		let mut is_update = self.update.lock().unwrap();
		*update = *is_update;
		if *update {
			std::mem::swap(&mut self.idx_swap, &mut self.idx_display);
			*is_update = false;
		}
		self.textures[self.idx_display]
	}

	#[inline]
	pub fn play_media<T: AsRef<std::path::Path>>(&mut self, url: T) -> std::result::Result<(), ()> {
		use std::ffi::CString;
		let url = CString::new(url.as_ref().as_os_str().to_str().unwrap()).unwrap();
		unsafe {
			self.media = libvlc_media_new_location(self.vlc, url.as_ptr());
			if self.media == std::ptr::null_mut() {
				return Err(());
			}
			self.player = libvlc_media_player_new_from_media(self.media);
			if self.player == std::ptr::null_mut() {
				libvlc_media_release(self.media);
				return Err(());
			}

			libvlc_video_set_output_callbacks(
				self.player,
				libvlc_video_engine_t::libvlc_video_engine_opengl,
				Some(VLCVideo::setup),
				Some(VLCVideo::cleanup),
				None,
				Some(VLCVideo::resize),
				Some(VLCVideo::swap),
				Some(VLCVideo::make_current),
				Some(VLCVideo::get_proc_address),
				None,
				None,
				self as *mut _ as *mut c_void,
			);

			libvlc_media_player_play(self.player);
			Ok(())
		}
	}

	unsafe extern "C" fn setup(
		data: *mut *mut c_void,
		_cfg: *const libvlc_video_setup_device_cfg_t,
		_out: *mut libvlc_video_setup_device_info_t,
	) -> bool {
		let that = &mut *(data as *mut VLCVideo);
		that.width = 0;
		that.height = 0;
		true
	}

	unsafe extern "C" fn cleanup(data: *mut c_void) {
		let that = &mut *(data as *mut VLCVideo);
		if that.height == 0 && that.width == 0 {
			return;
		}
		gl::DeleteTextures(3, that.textures.as_mut_ptr());
		gl::DeleteFramebuffers(3, that.fbo.as_mut_ptr());
	}

	unsafe extern "C" fn swap(data: *mut c_void) {
		let that = &mut *(data as *mut VLCVideo);
		let mut is_update = that.update.lock().unwrap();
		*is_update = true;
		std::mem::swap(&mut that.idx_swap, &mut that.idx_render);
		gl::BindFramebuffer(gl::FRAMEBUFFER, that.fbo[that.idx_render]);
	}

	unsafe extern "C" fn make_current(data: *mut c_void, current: bool) -> bool {
		let that = &mut *(data as *mut VLCVideo);
		if current {
			match that.shared_context.take() {
				Some(SharedContext::NotCurrent(n)) => {
					that.shared_context = Some(SharedContext::Current(n.make_current().unwrap()));
				},
				Some(SharedContext::Current(_)) => panic!("should not be current"),
				None => panic!("should not be None"),
			}
		} else {
			match that.shared_context.take() {
				Some(SharedContext::NotCurrent(_)) => panic!("should be current"),
				Some(SharedContext::Current(c)) => {
					that.shared_context =
						Some(SharedContext::NotCurrent(c.make_not_current().unwrap()));
				},
				None => panic!("should not be None"),
			}
		}
		true
	}

	unsafe extern "C" fn get_proc_address(data: *mut c_void, current: *const i8) -> *mut c_void {
		let that = &*(data as *mut VLCVideo);
		let s = CStr::from_ptr(current).to_str().unwrap();
		match &that.shared_context {
			Some(SharedContext::Current(c)) => {
				let addr = c.get_proc_address(s) as *mut c_void;
				return addr;
			},
			Some(SharedContext::NotCurrent(_)) => {
				panic!("Should be current");
			},
			None => panic!("should not be None"),
		}
	}

	unsafe extern "C" fn resize(
		data: *mut c_void,
		cfg: *const libvlc_video_render_cfg_t,
		render_cfg: *mut libvlc_video_output_cfg_t,
	) -> bool {
		let that = &mut *(data as *mut VLCVideo);
		let cfg = &*(cfg);
		let render_cfg = &mut *render_cfg;
		if cfg.width != that.width || cfg.height != that.height {
			VLCVideo::cleanup(data);
		}
		gl::GenTextures(3, that.textures.as_mut_ptr());
		gl::GenFramebuffers(3, that.fbo.as_mut_ptr());

		for i in 0..3 {
			glchk!(
			gl::BindTexture(gl::TEXTURE_2D, that.textures[i]);
			gl::TexImage2D(
				gl::TEXTURE_2D,
				0,
				gl::RGBA as i32,
				cfg.width as i32,
				cfg.height as i32,
				0,
				gl::RGBA,
				gl::UNSIGNED_BYTE,
				std::ptr::null_mut(),
			);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
			gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
			gl::BindFramebuffer(gl::FRAMEBUFFER, that.fbo[i]);
			gl::FramebufferTexture2D(
				gl::FRAMEBUFFER,
				gl::COLOR_ATTACHMENT0,
				gl::TEXTURE_2D,
				that.textures[i],
				0,
			);
			);
		}
		gl::BindTexture(gl::TEXTURE_2D, 0);
		let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
		if status != gl::FRAMEBUFFER_COMPLETE {
			panic!("FRAMEBUFFER STATUS NOT COMPLETE");
		}
		that.width = cfg.width;
		that.height = cfg.height;
		glchk!(gl::BindFramebuffer(gl::FRAMEBUFFER, that.fbo[that.idx_render]););
		render_cfg.u.opengl_format = gl::RGBA as i32;
		render_cfg.colorspace = libvlc_video_color_space_t::libvlc_video_colorspace_BT709;
		render_cfg.primaries = libvlc_video_color_primaries_t::libvlc_video_primaries_BT709;
		render_cfg.transfer = libvlc_video_transfer_func_t::libvlc_video_transfer_func_SRGB;
		render_cfg.orientation = libvlc_video_orient_t::libvlc_video_orient_top_left;
		true
	}
}
