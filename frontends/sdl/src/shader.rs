use gl::types::*;
use std::{ffi::CString, fs};

pub fn load_fragment_shader(path: &str) -> Result<u32, String> {
    let source = fs::read_to_string(path).map_err(|e| e.to_string())?;
    unsafe { compile_shader(gl::FRAGMENT_SHADER, &source) }
}

unsafe fn compile_shader(kind: GLenum, source: &str) -> Result<u32, String> {
    let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
    let vertex_source = fs::read_to_string("res/shaders/base.vsh").map_err(|e| e.to_string())?;
    let c_str = CString::new(vertex_source).unwrap();
    gl::ShaderSource(vertex_shader, 1, &c_str.as_ptr(), std::ptr::null());
    gl::CompileShader(vertex_shader);

    let shader = gl::CreateShader(kind);
    let c_str = CString::new(source).unwrap();
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
    gl::CompileShader(shader);

    let mut status = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    if status == 0 {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = Vec::with_capacity(len as usize);
        buf.set_len((len as usize) - 1);
        gl::GetShaderInfoLog(
            shader,
            len,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut _,
        );
        gl::DeleteShader(shader);
        return Err(String::from_utf8_lossy(&buf).into_owned());
    }
    let program = gl::CreateProgram();
    gl::AttachShader(program, vertex_shader);
    gl::AttachShader(program, shader);
    gl::LinkProgram(program);
    gl::DeleteShader(shader);

    gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
    if status == 0 {
        let mut len = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = Vec::with_capacity(len as usize);
        buf.set_len((len as usize) - 1);
        gl::GetProgramInfoLog(
            program,
            len,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut _,
        );
        gl::DeleteProgram(program);
        return Err(String::from_utf8_lossy(&buf).into_owned());
    }
    Ok(program)
}
