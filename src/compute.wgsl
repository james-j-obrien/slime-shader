[[block]]
struct TextureBuffer {
  texture : array<array<vec4<f32>, 1280>, 960>;
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

  sensor_angle: u32;
  sensor_size: i32;
  sensor_dist: f32;
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


var pi: f32 = 3.1416;

[[group(0), binding(0)]] var<uniform> params : Params;
[[group(0), binding(1)]] var<uniform> frame_data : FrameData;
[[group(0), binding(2)]] var<storage> texture_buffer : [[access(read_write)]] TextureBuffer;
[[group(0), binding(3)]] var<storage> agents_buffer : [[access(read_write)]] AgentsBuffer;


fn sense(agent: Agent, angle_offset: f32) -> f32 {
  var sensor_angle: f32 = agent.angle + angle_offset;
  var sensor_dir: vec2<f32> = vec2<f32>(cos(sensor_angle), sin(sensor_angle));

  var sensor_pos: vec2<f32> = agent.position + sensor_dir * params.sensor_dist;
  var sum: f32 = 0.0;
  var sensor_x: i32 = i32(sensor_pos.x);
  var sensor_y: i32 = i32(sensor_pos.y);

  for(var off_x: i32 = -params.sensor_size; off_x <= params.sensor_size; off_x = off_x + 1) {
    for(var off_y: i32 = -params.sensor_size; off_y <= params.sensor_size; off_y = off_y + 1) {
      var sample_x: u32 = min(params.width - 1u32, max(0, sensor_x + off_x));
      var sample_y: u32 = min(params.height - 1u32, max(0, sensor_y + off_y));
      sum = sum + dot(vec4<f32>(1.0, 1.0, 1.0, 0.0), texture_buffer.texture[sample_y][sample_x]);
    }
  }

  return sum;
}

[[stage(compute), workgroup_size(256)]]
fn update_agents(
  [[builtin(global_invocation_id)]] id: vec3<u32>
) {
  if (id.x >= params.num_agents) {
    return;
  }
  var frame_speed: f32 = frame_data.delta * params.speed;
  var agent: Agent = agents_buffer.agents[id.x];

  var sensor_angle: f32 = f32(params.sensor_angle) * (pi / 180.0);
  var weight_forward: f32 = sense(agent, 0.0);
  var weight_left: f32 =    sense(agent, sensor_angle);
  var weight_right: f32 =   sense(agent, -sensor_angle);

  var pos: vec2<f32> = agent.position;

  var random: u32 = hash(u32(pos.y)
    * params.width
    + u32(pos.x) 
    + hash(id.x + frame_data.frame_num * 100000u32)
  );

  var steer_strength: f32 = scaleToRange01(random);
  var steer_angle: f32 = 0.0;
  if (weight_forward >= weight_left && weight_forward >= weight_right) {
    steer_angle = 0.0;
  } elseif (weight_forward < weight_left && weight_forward < weight_right) {
    steer_angle = (steer_strength - 0.5) * 2.0 * frame_speed;
  } elseif (weight_right > weight_left) {
    steer_angle = -steer_strength * frame_speed;
  } elseif (weight_left > weight_right) {
    steer_angle = steer_strength * frame_speed;
  }

  var angle: f32 = agent.angle + steer_angle;
  var dir: vec2<f32> = vec2<f32>(cos(angle), sin(angle));
  pos = pos + dir * frame_speed;

  if (pos.x < 0.0 || pos.x >= f32(params.width) || pos.y < 0.0 || pos.y >= f32(params.height)) {
    random = hash(random);
    var randomAngle: f32 = scaleToRange01(random) * 2.0 * pi;
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