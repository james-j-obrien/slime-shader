pub const WIDTH: u32 = 1280;
pub const HEIGHT: u32 = 960;

pub const NUM_AGENTS: u32 = 1000;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Params {
    num_agents: u32,
    width: u32,
    height: u32,
    speed: f32,

    sensor_angle: u32,
    sensor_size: i32,
    sensor_dist: f32
}

pub const PARAMS: Params = Params {
    num_agents: NUM_AGENTS,
    width: WIDTH,
    height: HEIGHT,
    speed: 100.0,

    sensor_angle: 30,
    sensor_size: 20,
    sensor_dist: 35.0,
};

unsafe impl bytemuck::Zeroable for Params {}
unsafe impl bytemuck::Pod for Params {}