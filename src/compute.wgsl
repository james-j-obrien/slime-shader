[[block]]
struct TextureBuffer {
  texture : array<array<vec4<f32>, 640>, 480>;
};

struct Agent {
	position : vec2<f32>;
	angle : f32;
};

[[block]]
struct AgentsBuffer {
  agents : array<Agent>;
};

[[block]]
struct Params {
  num_agents : u32;
  width : u32;
  height : u32;
  speed: f32;
};

[[block]]
struct FrameData {
  frame_num : u32;
  delta : f32;
};

fn hash(state: u32) -> u32 {
  var out: u32 = state;
  out = out ^ 2747636419u32;
  out = out * 2654435769u32;
  out = out ^ (out >> 16u32);
  out = out * 2654435769u32;
  out = out ^ (out >> 16u32);
  out = out * 2654435769u32;
  return out;
}

fn scaleToRange01(state: u32) -> f32 {
  return f32(state) / 4294967295.0;
}

[[group(0), binding(0)]] var<uniform> params : Params;
[[group(0), binding(1)]] var<uniform> frame_data : FrameData;
[[group(0), binding(2)]] var<storage> texture_buffer : [[access(read_write)]] TextureBuffer;
[[group(0), binding(3)]] var<storage> agents_buffer : [[access(read_write)]] AgentsBuffer;

[[stage(compute), workgroup_size(256)]]
fn update_agents(
  [[builtin(global_invocation_id)]] id: vec3<u32>
) {
  if (id.x >= params.num_agents) {
    return;
  }
  var agent: Agent = agents_buffer.agents[id.x];

  var dir: vec2<f32> = vec2<f32>(cos(agent.angle), sin(agent.angle));
  var pos: vec2<f32> = agent.position + dir * frame_data.delta * params.speed;
  var angle: f32 = agent.angle;

  var random: u32 = hash(u32(pos.y)
    * params.width
    + u32(pos.x) 
    + hash(id.x + frame_data.frame_num * 100000u32)
  );

  if (pos.x < 0.0 || pos.x >= f32(params.width) || pos.y < 0.0 || pos.y >= f32(params.height)) {
    var randomAngle: f32 = scaleToRange01(random) * 2.0 * 3.1415;
    pos.x = min(f32(params.width) - 0.01, max(0.0, pos.x));
    pos.y = min(f32(params.height) - 0.01, max(0.0, pos.y));
    angle = randomAngle;
  }

  texture_buffer.texture[u32(pos.y)][u32(pos.x)] = vec4<f32>(1.0, 1.0, 1.0, 1.0);
  
  agents_buffer.agents[id.x].position = pos;
  agents_buffer.agents[id.x].angle = angle;
}

[[stage(compute), workgroup_size(256)]]
fn post_process(
  [[builtin(global_invocation_id)]] id: vec3<u32>
) {
  if (id.x >= params.width || id.y >= params.height) {
    return;
  }
  var cur : vec4<f32> = texture_buffer.texture[id.y][id.x];
  cur = cur * (1.0 - 0.02 * frame_data.delta * params.speed);
  texture_buffer.texture[id.y][id.x] = cur;
}