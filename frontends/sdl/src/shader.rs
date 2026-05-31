use std::ffi::CString;

use gl::types::GLenum;

const VERTEX_SHADER: &str = include_str!("../res/shaders/master.vert");

const PASSTHROUGH_FRAGMENT: &str = include_str!("../res/shaders/passthrough.frag");
const BILINEAR_FRAGMENT: &str = include_str!("../res/shaders/bilinear.frag");
const SMOOTH_BILINEAR_FRAGMENT: &str = include_str!("../res/shaders/smooth_bilinear.frag");
const CRT_FRAGMENT: &str = include_str!("../res/shaders/crt.frag");
const MASTER_FRAGMENT: &str = include_str!("../res/shaders/master.frag");

pub fn load_shader_program(name: &str) -> Result<u32, String> {
    let fragment_partial = match name {
        "pass" => PASSTHROUGH_FRAGMENT,
        "passthrough" => PASSTHROUGH_FRAGMENT,
        "bilinear" => BILINEAR_FRAGMENT,
        "smooth" => SMOOTH_BILINEAR_FRAGMENT,
        "smooth_bilinear" => SMOOTH_BILINEAR_FRAGMENT,
        "crt" => CRT_FRAGMENT,
        "master" => MASTER_FRAGMENT,
        _ => return Err(format!("Shader {name} not found")),
    };
    let fragment_source = MASTER_FRAGMENT.replace("{filter}", fragment_partial);
    unsafe { compile_program(VERTEX_SHADER, &fragment_source) }
}

unsafe fn compile_program(vertex_src: &str, fragment_src: &str) -> Result<u32, String> {
    let vs = compile_shader(gl::VERTEX_SHADER, vertex_src)?;
    let fs = compile_shader(gl::FRAGMENT_SHADER, fragment_src)?;
    let program = gl::CreateProgram();
    gl::AttachShader(program, vs);
    gl::AttachShader(program, fs);
    gl::LinkProgram(program);
    gl::DeleteShader(vs);
    gl::DeleteShader(fs);
    let mut status = 0;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
    if status == 0 {
        let mut len = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = vec![0u8; (len as usize) - 1];
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

unsafe fn compile_shader(kind: GLenum, source: &str) -> Result<u32, String> {
    let shader = gl::CreateShader(kind);
    let c_str = CString::new(source).unwrap();
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
    gl::CompileShader(shader);
    let mut status = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    if status == 0 {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = vec![0u8; (len as usize) - 1];
        gl::GetShaderInfoLog(
            shader,
            len,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut _,
        );
        gl::DeleteShader(shader);
        return Err(String::from_utf8_lossy(&buf).into_owned());
    }
    Ok(shader)
}
