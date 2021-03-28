mod shader;
use shader::Shader;

mod framework;

mod constants;

fn main() {
    framework::run::<Shader>("Slime Shader");
}
