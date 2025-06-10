use gl::types::*;
use std::{ffi::CString, fs};
const VERTEX_SHADER: &str = "#version 330 core\nlayout(location = 0) in vec2 pos;\nlayout(location = 1) in vec2 tex;\nout vec2 v_tex;\nvoid main(){v_tex = tex;gl_Position = vec4(pos,0.0,1.0);}";
pub fn load_shader_program(path: &str) -> Result<u32, String> {
    let fragment = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let fragment_source = format!(
        "#version 330 core\n#define STATIC\nin vec2 v_tex;\nout vec4 color;\nuniform sampler2D image;\nuniform vec2 input_resolution;\nuniform vec2 output_resolution;\n{}\nvoid main(){{color = scale(image, v_tex, input_resolution, output_resolution);}}",
        fragment
    );
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
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        gl::GetProgramInfoLog(
            program,
        gl::DeleteProgram(program);
    Ok(program)
}
unsafe fn compile_shader(kind: GLenum, source: &str) -> Result<u32, String> {
    let shader = gl::CreateShader(kind);
    let c_str = CString::new(source).unwrap();
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
    gl::CompileShader(shader);
    let mut status = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
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
    Ok(shader)
    gl::AttachShader(program, vertex_shader);
    gl::AttachShader(program, shader);
    gl::LinkProgram(program);
    
    gl::DeleteShader(shader);
    gl::DeleteShader(vertex_shader);

    // checks if the program linked successfully and if not
    // returns the error message
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

    gl::UseProgram(program);

    Ok(program)
}
